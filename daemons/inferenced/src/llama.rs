//! llama-server child supervision scaffold (`docs/PLAN.md` §5.1).
//!
//! M0 scope: spawn/health-check/kill lifecycle for one child, proving the
//! supervisor shape. M1 adds: request proxying, one child per resident
//! model, LoRA hot-swap, VRAM budget arbitration, PSI-driven eviction, and
//! the crash-restart acceptance test (kill -9 → service restored < 5 s).

use crate::config::LlamaConfig;
use crate::engine::{Engine, EngineError, TokenStream};
use crate::openai::ChatMessage;
use std::process::Stdio;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

pub struct LlamaEngine {
    cfg: LlamaConfig,
    child: Mutex<Option<Child>>,
}

impl LlamaEngine {
    pub fn new(cfg: LlamaConfig) -> Self {
        Self {
            cfg,
            child: Mutex::new(None),
        }
    }

    /// Spawn llama-server if not already running and wait until its port
    /// accepts connections.
    pub async fn ensure_running(&self) -> Result<(), EngineError> {
        let mut guard = self.child.lock().await;
        if let Some(child) = guard.as_mut()
            && child
                .try_wait()
                .map_err(|e| EngineError::Unavailable(e.to_string()))?
                .is_none()
        {
            return Ok(());
        }

        let model =
            self.cfg.model_path.as_ref().ok_or_else(|| {
                EngineError::Unavailable("llama.model_path not configured".into())
            })?;
        let child = Command::new(&self.cfg.server_bin)
            .arg("--model")
            .arg(model)
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| {
                EngineError::Unavailable(format!("spawning {}: {e}", self.cfg.server_bin.display()))
            })?;
        *guard = Some(child);
        drop(guard);

        // Poll the port until the server is up (~10 s budget).
        let addr = format!("127.0.0.1:{}", self.cfg.port);
        for _ in 0..100 {
            if TcpStream::connect(&addr).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err(EngineError::Unavailable(format!(
            "llama-server did not come up on {addr}"
        )))
    }

    pub async fn shutdown(&self) {
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await;
        }
    }
}

impl Engine for LlamaEngine {
    fn name(&self) -> &'static str {
        "llama"
    }

    fn generate(&self, _messages: Vec<ChatMessage>) -> TokenStream {
        // Proxying to the supervised child's endpoint is an M1 deliverable;
        // failing loudly here beats pretending.
        Box::pin(futures::stream::once(async {
            Err(EngineError::Unavailable(
                "llama-server proxying lands in M1 (PLAN §5.1); run with engine = \"stub\"".into(),
            ))
        }))
    }
}
