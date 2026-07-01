use crate::data::capabilities::Capabilities;

pub(crate) async fn resolve(context: &context::Context) -> Capabilities {
    let enabled = context.settings.inner().await.sharing.enabled();
    Capabilities::for_enabled(enabled)
}
