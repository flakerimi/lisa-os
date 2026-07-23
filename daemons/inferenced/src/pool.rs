//! Multi-model residency (`docs/PLAN.md` §5.1): one supervised engine
//! child per resident model, spawned lazily on first request and evicted
//! LRU when the residency cap is reached. Eviction kills the child —
//! mmap-backed weights make a later respawn cheap. PSI-driven pressure
//! eviction refines the cap on Linux later.

use crate::engine::{Engine, EngineError};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Resolves a request's model name to a live engine.
pub trait EngineProvider: Send + Sync {
    fn engine_for(
        &self,
        model: Option<&str>,
    ) -> BoxFuture<'_, Result<Arc<dyn Engine>, EngineError>>;
    /// Model names this provider can currently serve (for /v1/models).
    fn known_models(&self) -> Vec<String>;
}

/// Single fixed engine (the stub, or a single-model deployment).
pub struct SingleEngine {
    pub engine: Arc<dyn Engine>,
    pub name: String,
}

impl EngineProvider for SingleEngine {
    fn engine_for(
        &self,
        _model: Option<&str>,
    ) -> BoxFuture<'_, Result<Arc<dyn Engine>, EngineError>> {
        let engine = Arc::clone(&self.engine);
        Box::pin(async move { Ok(engine) })
    }

    fn known_models(&self) -> Vec<String> {
        vec![self.name.clone()]
    }
}

type EngineFactory =
    Box<dyn Fn(&str, PathBuf, u16) -> Result<Arc<dyn Engine>, EngineError> + Send + Sync>;

/// Pool of per-model engines, keyed by store ref name.
pub struct ModelPool {
    default_model: String,
    refs_dir: PathBuf,
    base_port: u16,
    max_resident: usize,
    factory: EngineFactory,
    inner: Mutex<PoolState>,
}

#[derive(Default)]
struct PoolState {
    engines: HashMap<String, Arc<dyn Engine>>,
    /// Least-recently-used order, most recent last.
    lru: Vec<String>,
    next_port_offset: u16,
}

impl ModelPool {
    pub fn new(
        default_model: String,
        refs_dir: PathBuf,
        base_port: u16,
        max_resident: usize,
        factory: EngineFactory,
    ) -> Self {
        Self {
            default_model,
            refs_dir,
            base_port,
            max_resident: max_resident.max(1),
            factory,
            inner: Mutex::new(PoolState::default()),
        }
    }

    async fn resolve(&self, model: Option<&str>) -> Result<Arc<dyn Engine>, EngineError> {
        // Well-known aliases resolve to the resident model, so callers can
        // just say "lisa" (or omit the field) without knowing the exact
        // model id — the common single-model case just works.
        let requested = model.unwrap_or(&self.default_model);
        let name = if matches!(
            requested,
            "lisa" | "lisa-system" | "lisa-system-stub" | "default" | "auto" | ""
        ) {
            self.default_model.clone()
        } else {
            requested.to_string()
        };
        let mut state = self.inner.lock().await;

        if let Some(engine) = state.engines.get(&name) {
            let engine = Arc::clone(engine);
            state.lru.retain(|n| n != &name);
            state.lru.push(name);
            return Ok(engine);
        }

        // The model must exist in the store (or be the default, whose
        // path was validated at startup).
        let path = if name == self.default_model {
            self.refs_dir.join(&name)
        } else {
            let candidate = self.refs_dir.join(&name);
            if !candidate.exists() {
                return Err(EngineError::Unavailable(format!(
                    "model `{name}` is not in the store (lisa models list)"
                )));
            }
            candidate
        };

        // Evict LRU beyond the residency cap before spawning another.
        while state.lru.len() >= self.max_resident {
            let evicted_name = state.lru.remove(0);
            if let Some(evicted) = state.engines.remove(&evicted_name) {
                info!(model = evicted_name, "evicting LRU resident model");
                evicted.shutdown().await;
            }
        }

        let port = self.base_port + state.next_port_offset;
        state.next_port_offset = state.next_port_offset.wrapping_add(1);
        info!(model = name, port, "admitting model to the pool");
        let engine = (self.factory)(&name, path, port)?;
        state.engines.insert(name.clone(), Arc::clone(&engine));
        state.lru.push(name);
        Ok(engine)
    }
}

impl EngineProvider for ModelPool {
    fn engine_for(
        &self,
        model: Option<&str>,
    ) -> BoxFuture<'_, Result<Arc<dyn Engine>, EngineError>> {
        let model = model.map(str::to_string);
        Box::pin(async move { self.resolve(model.as_deref()).await })
    }

    fn known_models(&self) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(&self.refs_dir)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        if !names.contains(&self.default_model) {
            names.push(self.default_model.clone());
        }
        names.sort();
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{GenerateRequest, StubEngine, TokenStream};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct MockEngine {
        alive: Arc<AtomicUsize>,
    }

    impl Engine for MockEngine {
        fn name(&self) -> &'static str {
            "mock"
        }
        fn generate(&self, req: GenerateRequest) -> TokenStream {
            StubEngine.generate(req)
        }
        fn embed(
            &self,
            texts: Vec<String>,
        ) -> BoxFuture<'static, Result<Vec<Vec<f32>>, EngineError>> {
            StubEngine.embed(texts)
        }
        fn shutdown(&self) -> BoxFuture<'static, ()> {
            let alive = Arc::clone(&self.alive);
            Box::pin(async move {
                alive.fetch_sub(1, Ordering::SeqCst);
            })
        }
    }

    fn test_pool(dir: &std::path::Path, cap: usize) -> (Arc<AtomicUsize>, ModelPool) {
        let alive = Arc::new(AtomicUsize::new(0));
        let spawned = Arc::clone(&alive);
        let pool = ModelPool::new(
            "default-model".into(),
            dir.to_path_buf(),
            7800,
            cap,
            Box::new(move |_name, _path, _port| {
                spawned.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::new(MockEngine {
                    alive: Arc::clone(&spawned),
                }))
            }),
        );
        (alive, pool)
    }

    #[tokio::test]
    async fn same_model_reuses_the_engine() {
        let dir = tempfile::tempdir().unwrap();
        let (alive, pool) = test_pool(dir.path(), 2);
        pool.resolve(None).await.unwrap();
        pool.resolve(None).await.unwrap();
        pool.resolve(Some("default-model")).await.unwrap();
        assert_eq!(alive.load(Ordering::SeqCst), 1, "one spawn for one model");
    }

    #[tokio::test]
    async fn lru_eviction_shuts_down_the_oldest() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("model-b"), b"x").unwrap();
        std::fs::write(dir.path().join("model-c"), b"x").unwrap();
        let (alive, pool) = test_pool(dir.path(), 2);

        pool.resolve(None).await.unwrap(); // default
        pool.resolve(Some("model-b")).await.unwrap();
        assert_eq!(alive.load(Ordering::SeqCst), 2);

        // Touch default so model-b becomes LRU, then admit model-c.
        pool.resolve(None).await.unwrap();
        pool.resolve(Some("model-c")).await.unwrap();
        assert_eq!(alive.load(Ordering::SeqCst), 2, "cap holds: one evicted");

        // model-b was evicted; asking for it again respawns.
        pool.resolve(Some("model-b")).await.unwrap();
        assert_eq!(alive.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn well_known_aliases_resolve_to_the_default() {
        let dir = tempfile::tempdir().unwrap();
        let (alive, pool) = test_pool(dir.path(), 2);
        for alias in ["lisa", "default", "auto", ""] {
            pool.resolve(Some(alias)).await.unwrap();
        }
        // All aliases hit the one default model — a single spawn.
        assert_eq!(alive.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn unknown_model_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (_alive, pool) = test_pool(dir.path(), 2);
        let err = match pool.resolve(Some("no-such-model")).await {
            Err(e) => e,
            Ok(_) => panic!("unknown model must be refused"),
        };
        assert!(err.to_string().contains("not in the store"));
    }
}
