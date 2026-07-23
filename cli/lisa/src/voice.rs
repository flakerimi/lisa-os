//! Voice + ambient loop for the CLI (`docs/PLAN.md` §5.7.5, ADR-0011).
//!
//! `transcribe` (whisper.cpp) → `ambient classify` (addressed-intent via
//! guided generation) → answer → `say` (piper / local TTS). This is the
//! Lisa Ambient loop minus live mic capture + the wake word, driven from
//! an audio file so it runs and tests without hardware. The daemon path
//! (inferenced supervising whisper-server, a `voiced` capture loop) is
//! the productionization — this proves the pieces.

use anyhow::{Context, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Resolve a whisper ggml model: --model, $LISA_WHISPER_MODEL, or the
/// store's `whisper` ref.
pub fn whisper_model(explicit: Option<PathBuf>) -> anyhow::Result<PathBuf> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    if let Some(p) = std::env::var_os("LISA_WHISPER_MODEL") {
        return Ok(PathBuf::from(p));
    }
    let store_ref = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|h| h.join(".local/share/lisa/models/refs/whisper"))
        .filter(|p| p.exists());
    store_ref.ok_or_else(|| {
        anyhow::anyhow!(
            "no whisper model — pass --model, set LISA_WHISPER_MODEL, or \
             `lisa models get whisper-base-en`"
        )
    })
}

/// Transcribe an audio file with whisper.cpp. Returns the plain text.
pub fn transcribe(audio: &Path, model: &Path) -> anyhow::Result<String> {
    if !audio.exists() {
        bail!("audio file {} not found", audio.display());
    }
    let out = Command::new("whisper-cli")
        .arg("-m")
        .arg(model)
        .arg("-f")
        .arg(audio)
        .arg("-nt") // no timestamps
        .output()
        .context("running whisper-cli — is whisper.cpp installed?")?;
    if !out.status.success() {
        bail!(
            "whisper-cli failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Speak text locally: piper on the image, `say` on a macOS dev host.
pub fn say(text: &str) -> anyhow::Result<()> {
    if which("piper") {
        // piper reads text on stdin, writes raw audio; pipe to aplay.
        let piper = Command::new("piper")
            .args([
                "--model",
                "/usr/share/lisa/voices/en_US.onnx",
                "--output-raw",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn();
        if let Ok(mut child) = piper {
            use std::io::Write;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            let _ = child.wait();
            return Ok(());
        }
    }
    if which("say") {
        Command::new("say").arg(text).status()?;
        return Ok(());
    }
    // No TTS available: print so the loop still completes.
    println!("[tts unavailable] {text}");
    Ok(())
}

fn which(bin: &str) -> bool {
    Command::new("which")
        .arg(bin)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Wake-word gate (ADR-0011: "Hey Lisa" is the shipping default). True
/// when the utterance starts by addressing Lisa. Reliable on any model —
/// unlike the addressed-intent classifier, which over-triggers on small
/// models (a 1B model called a "let's order pizza" aside addressed).
/// Returns the request with the wake phrase stripped.
pub fn wake_word(transcript: &str) -> Option<String> {
    let lower = transcript.trim_start().to_ascii_lowercase();
    for phrase in ["hey lisa", "ok lisa", "okay lisa", "lisa,"] {
        if lower.starts_with(phrase) {
            let rest = transcript.trim_start()[phrase.len()..]
                .trim_start_matches([',', '.', '!', '?', ' '])
                .trim();
            return Some(rest.to_string());
        }
    }
    None
}

/// The addressed-intent decision (ADR-0011): {addressed, confidence,
/// intent}, produced by guided generation against the local model.
#[derive(Debug)]
pub struct Addressed {
    pub addressed: bool,
    pub confidence: f64,
    pub intent: String,
}

/// Classify whether `transcript` was addressed to Lisa, via the endpoint.
pub fn classify_addressed(transcript: &str, url: &str) -> anyhow::Result<Addressed> {
    let body = liblisa::tasks::addressed_intent().request(transcript);
    let endpoint = format!("{}/v1/chat/completions", url.trim_end_matches('/'));
    let mut resp = ureq::post(&endpoint)
        .send_json(&body)
        .with_context(|| format!("request to {endpoint} — is lisa-inferenced running?"))?;
    let json: serde_json::Value = resp.body_mut().read_json()?;
    if let Some(err) = json["error"]["message"].as_str() {
        bail!("inference error: {err}");
    }
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("{}");
    let v: serde_json::Value =
        serde_json::from_str(content).with_context(|| format!("classifier output: {content}"))?;
    Ok(Addressed {
        addressed: v["addressed"].as_bool().unwrap_or(false),
        confidence: v["confidence"].as_f64().unwrap_or(0.0),
        intent: v["intent"].as_str().unwrap_or("").to_string(),
    })
}

/// One non-streaming answer from the local model (used by the ambient
/// loop to speak a reply).
pub fn answer(prompt: &str, url: &str) -> anyhow::Result<String> {
    let endpoint = format!("{}/v1/chat/completions", url.trim_end_matches('/'));
    let body = serde_json::json!({
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 200,
    });
    let mut resp = ureq::post(&endpoint)
        .send_json(&body)
        .with_context(|| format!("request to {endpoint} — is lisa-inferenced running?"))?;
    let json: serde_json::Value = resp.body_mut().read_json()?;
    if let Some(err) = json["error"]["message"].as_str() {
        bail!("inference error: {err}");
    }
    Ok(json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_word_gates_and_strips() {
        assert_eq!(
            wake_word("Hey Lisa, what is the capital of France?").as_deref(),
            Some("what is the capital of France?")
        );
        assert_eq!(
            wake_word("OK Lisa turn off the lights").as_deref(),
            Some("turn off the lights")
        );
        assert_eq!(wake_word("I think we should order pizza tonight"), None);
        assert_eq!(wake_word("the mona lisa is a painting"), None);
    }
}
