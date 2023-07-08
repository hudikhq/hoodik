use crate::contracts::{
    account::Account, cookies::Cookies, ctx::Ctx, email::Email, register::Register,
    repository::Repository, sessions::Sessions,
};
use context::Context;

pub(crate) struct Auth<'ctx> {
    pub(crate) context: &'ctx Context,
}

impl Cookies for Auth<'_> {}
impl Email for Auth<'_> {}
impl Register for Auth<'_> {}
impl Repository for Auth<'_> {}
impl Sessions for Auth<'_> {}
impl Account for Auth<'_> {}

impl Ctx for Auth<'_> {
    fn ctx(&self) -> &Context {
        self.context
    }
}

impl<'ctx> Auth<'ctx> {
    pub(crate) fn new(context: &'ctx Context) -> Auth<'ctx> {
        Auth { context }
    }
}
