//! D-Bus scaffold for org.lisa.Inference1 (`docs/PLAN.md` §5.1, Appendix A).
//!
//! M0 registers the name and a Ping method so `busctl` can see us; the real
//! surface (OpenSession with fd-passed token streams, Embed, guided
//! generation) is an M1 deliverable. Disabled by default (`dbus = false`)
//! since dev hosts — macOS included — may have no session bus.

pub struct Inference1;

#[zbus::interface(name = "org.lisa.Inference1")]
impl Inference1 {
    /// Liveness probe: `busctl --user call org.lisa.Inference1
    /// /org/lisa/Inference1 org.lisa.Inference1 Ping`
    fn ping(&self) -> String {
        format!("lisa-inferenced {}", env!("CARGO_PKG_VERSION"))
    }

    /// PLAN Appendix A: OpenSession(options) → (session, stream_fd).
    fn open_session(&self) -> zbus::fdo::Result<String> {
        Err(zbus::fdo::Error::NotSupported(
            "OpenSession lands in M1 (PLAN §5.1); use the OpenAI-compat HTTP endpoint".into(),
        ))
    }
}

pub async fn serve() -> zbus::Result<zbus::Connection> {
    zbus::connection::Builder::session()?
        .name("org.lisa.Inference1")?
        .serve_at("/org/lisa/Inference1", Inference1)?
        .build()
        .await
}
