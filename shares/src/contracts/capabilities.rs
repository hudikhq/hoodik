use crate::data::capabilities::Capabilities;

pub(crate) async fn resolve(context: &context::Context) -> Capabilities {
    let settings = context.settings.inner().await;
    let sharing = &settings.sharing;
    Capabilities::for_enabled(
        sharing.enabled(),
        sharing.default_cipher().to_string(),
        // Settled once at startup: see `fs::direct`. A bucket that cannot
        // serve clients directly leaves this false, and every client keeps
        // transferring through this server.
        config::direct::verdict().enabled,
    )
}
