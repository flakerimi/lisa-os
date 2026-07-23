# ADR-0011: Lisa Ambient — the always-on, wake-word-free assistant

- **Status:** accepted (design; implementation staged)
- **Date:** 2026-07-23

## Context

The product vision is an assistant that is simply *present* — you speak,
it answers, no "hey Lisa" handshake — and that can see, hear, and read.
This is the single most trust-sensitive feature Lisa will ship:
"always listening" is exactly what makes people distrust Alexa, and
"sees your screen" is what made Recall a scandal. Lisa's whole thesis
(PLAN §1, §4: radical legibility, egress blocked by mechanism) has to be
what makes always-on *acceptable* rather than creepy.

So the design question is not "can we transcribe continuously" — llama
+ whisper make that easy — but "how do we make always-on provably
private, and how do we respond without a wake word without uploading a
live transcript of your life."

## Decision

Ship **Lisa Ambient**: a local, always-on listening loop that responds
only when it decides you addressed it, with privacy enforced by
architecture, not policy.

### The loop (all on-device)

```
mic ─▶ VAD (voice activity) ─▶ [speech segment] ─▶ local STT (whisper)
        │ silence: nothing happens, nothing stored
        ▼
   transcript ─▶ addressed-intent classifier (system model, guided:
                 {addressed: bool, confidence, intent})
        │ not addressed: transcript discarded, ring buffer overwritten
        ▼ addressed
   assistant loop (context: [selection] [screen] [my stuff]) ─▶ answer
        ▼
   local TTS (piper/kokoro) ─▶ speaker
```

### No wake word, done honestly (the novel piece)

Instead of a wake-word model gating the mic, the mic is always
VAD-gated locally, and **the system model classifies whether a
completed utterance was addressed to Lisa** — grammar-constrained to
`{addressed, confidence, intent}` (guided generation, §5.6). Speaking
*near* Lisa is not speaking *to* Lisa; the classifier is what tells them
apart. Wake-word mode (openWakeWord) remains available as a
lower-power / higher-precision option and the default until Ambient's
false-accept rate is measured acceptable on real hardware.

### Privacy as mechanism (non-negotiable invariants)

1. **Nothing leaves the device.** The whole loop runs on
   `lisa-inferenced` (STT, classify, generate) and local TTS. Egress
   stays blocked (rule 5); a remote provider is used only if a request's
   scopes are explicitly consented (§5.11) — never for ambient audio.
2. **Nothing is persisted by default.** Audio lives in a fixed-size
   in-process **ring buffer**; segments are transcribed and the audio is
   overwritten. Transcripts of *un-addressed* speech are discarded
   immediately, never indexed. Only an *addressed* exchange is ledgered
   (and only its text envelope, per §5.7.6) — and only pinned to the
   context fabric if the user pins it.
3. **Every activation is in the Ledger.** "Lisa woke up at 15:04, heard
   N seconds, decided addressed=true, answered" — the Ledger app is the
   answer to "what did it hear?", which no competitor can give.
4. **A hard mute that is real.** A global mute cuts the capture thread
   (not just the UI), reflected by a persistent, always-visible
   indicator whenever the mic is live (the §5.7.5 "hardware-LED-style"
   dot). Mute state survives reboot.
5. **Not Recall.** Ambient is audio-on-request-of-speech, never ambient
   *screen* capture. Screen/selection context is pulled only for an
   addressed turn, per-invocation, provenance-tagged `screen`
   (untrusted, §5.7.4). No continuous visual capture, ever.

### Multimodal ("see, hear, read")

- **Hear:** the loop above.
- **Read:** `[selection]` (app-published `selection://current` or
  AT-SPI) and `[my stuff]` (context-fabric scopes) — already primitives
  (§5.6, §5.3).
- **See:** `[this window]` → ScreenCast portal frame → local VLM
  (§5.7.4), pulled only for an addressed turn, with the sharing
  indicator lit.

## Consequences

- Ambient is a strict superset of the existing Super+Space overlay
  (§5.7.1): the overlay is Ambient with the "addressed" decision made by
  a keypress instead of the classifier. They share one backend.
- New failure mode — **false activation** (responding when not
  addressed). Mitigated by: classifier confidence threshold, a visible
  "Lisa is listening/answering" state the user can cancel, and a
  measured false-accept CI/eval gate before Ambient is default.
- Compute: a small always-warm VAD + whisper-small + the resident
  system model. On Tier 0/1 hardware Ambient may fall back to wake-word
  mode (§5.9 power/thermal caps apply; background QoS).
- The addressed-intent classifier and the false-accept eval are new
  eval-harness targets (§11).

## Staging

1. **Substrate (now):** local STT + TTS in `lisa-inferenced`
   (transcribe/speak), OpenAI-compat endpoints, catalog pins, `lisa
   transcribe` / `lisa say`.
2. **Addressed-intent classifier (now):** guided-generation module +
   eval fixtures.
3. **Ambient loop (needs audio hardware / the field iMac):** VAD +
   ring-buffer capture, the mute + indicator, ledger wiring, the overlay
   backend consuming audio turns.
4. **Multimodal turn:** screen/selection/context attached to an
   addressed turn.
