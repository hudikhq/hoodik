use context::Context;

/// Expose the inner context to the implementor
pub(crate) trait Ctx {
    fn ctx(&self) -> &Context;
}
