//! lisa-inferenced entrypoint — the one process that owns compute for
//! inference (`docs/PLAN.md` §5.1). M0 walking skeleton: OpenAI-compat HTTP
//! on loopback with the stub engine, llama-server supervision scaffold, and
//! an optional D-Bus liveness surface.

use clap::Parser;
use lisa_inferenced::config::{self, Config, EngineKind};
use lisa_inferenced::engine::StubEngine;
use lisa_inferenced::pool::{EngineProvider, ModelPool, SingleEngine};
use lisa_inferenced::{api, dbus, llama};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Parser)]
#[command(
    name = "lisa-inferenced",
    about = "Lisa OS model runtime & scheduler (M0 scaffold)"
)]
struct Args {
    /// TOML config path; defaults are used when omitted.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Override the bind address (default 127.0.0.1:7777).
    #[arg(long)]
    bind: Option<String>,
    /// Override the engine: stub | llama.
    #[arg(long)]
    engine: Option<String>,
    /// Model path for the llama engine (implies --engine llama).
    #[arg(long)]
    model: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();
    let mut cfg = Config::load(args.config.as_deref())?;
    if let Some(bind) = args.bind {
        cfg.bind = config::Bind(bind);
    }
    if let Some(model) = args.model {
        cfg.llama.model_path = Some(model);
        cfg.engine = EngineKind::Llama;
    }
    match args.engine.as_deref() {
        Some("stub") => cfg.engine = EngineKind::Stub,
        Some("llama") => cfg.engine = EngineKind::Llama,
        Some(other) => anyhow::bail!("unknown engine `{other}` (stub | llama)"),
        None => {}
    }

    let model_name_for = |p: &std::path::Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "lisa-system".to_string())
    };
    let engines: Arc<dyn EngineProvider> = match cfg.engine {
        EngineKind::Stub => Arc::new(SingleEngine {
            engine: Arc::new(StubEngine),
            name: "lisa-system-stub".to_string(),
        }),
        EngineKind::Llama => {
            let model_path = cfg.llama.model_path.clone().ok_or_else(|| {
                anyhow::anyhow!("engine llama requires --model or llama.model_path")
            })?;
            let refs_dir = model_path
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            let default_model = model_name_for(&model_path);
            let base = cfg.llama.clone();
            // One supervised llama-server child per resident model,
            // lazily spawned, LRU-evicted beyond max_resident (§5.1).
            Arc::new(ModelPool::new(
                default_model,
                refs_dir,
                base.port,
                base.max_resident,
                Box::new(move |_name, path, port| {
                    let mut child_cfg = base.clone();
                    child_cfg.model_path = Some(path);
                    child_cfg.port = port;
                    Ok(Arc::new(llama::LlamaEngine::new(child_cfg)))
                }),
            ))
        }
    };
    // Wrap the local provider so `remote:<provider>:<model>` names route
    // to the lisa-remoted broker (§5.11); local models pass through
    // unchanged. inferenced stays network-free — the broker owns egress.
    let engines: Arc<dyn EngineProvider> = Arc::new(lisa_inferenced::remote::RemoteRouter::new(
        engines,
        lisa_inferenced::remote::default_socket(),
    ));
    info!("engine provider initialized (remote routing enabled)");

    let scheduler = Arc::new(lisa_inferenced::scheduler::Scheduler::new(1));

    // D-Bus is opt-in until the portal lands; never fatal.
    let _dbus_conn = if cfg.dbus {
        match dbus::serve(Arc::clone(&engines), Arc::clone(&scheduler)).await {
            Ok(conn) => {
                info!("org.lisa.Inference1 registered on the session bus");
                Some(conn)
            }
            Err(e) => {
                warn!("D-Bus unavailable, continuing HTTP-only: {e}");
                None
            }
        }
    } else {
        None
    };

    let model_name = match cfg.engine {
        EngineKind::Stub => "lisa-system-stub".to_string(),
        EngineKind::Llama => cfg
            .llama
            .model_path
            .as_ref()
            .map(|p| model_name_for(p))
            .unwrap_or_else(|| "lisa-system".to_string()),
    };
    let engine_kind = match cfg.engine {
        EngineKind::Stub => "stub".to_string(),
        EngineKind::Llama => "llama".to_string(),
    };
    // No ledger, no inference (PLAN §4 rule 4): refuse to serve at all
    // if the audit log cannot be opened.
    let ledger_path = std::env::var_os("LISA_LEDGER_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(lisa_ledger::Ledger::default_path);
    let ledger = lisa_ledger::Ledger::open(&ledger_path)
        .map_err(|e| anyhow::anyhow!("cannot open ledger {}: {e}", ledger_path.display()))?;
    info!(ledger = %ledger_path.display(), "ledger open (append-only)");

    let state = api::AppState {
        engines,
        scheduler,
        engine_kind,
        model_name,
        ledger: Arc::new(ledger),
    };
    let listener = tokio::net::TcpListener::bind(&cfg.bind.0).await?;
    info!("OpenAI-compat endpoint on http://{}", cfg.bind.0);
    axum::serve(listener, api::router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            info!("shutting down");
        })
        .await?;
    Ok(())
}
