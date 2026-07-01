pub(crate) mod account_deletion;
pub(crate) mod audit;
pub(crate) mod discover;
pub(crate) mod discover_rate_limit;
pub(crate) mod folder_members;
pub(crate) mod fork;
pub(crate) mod groups;
pub(crate) mod members_list_sig;
pub(crate) mod move_subtree;
pub(crate) mod multikey_upload;
pub(crate) mod nonce;
pub(crate) mod notify;
pub(crate) mod queries;
pub(crate) mod rate_limit;
pub(crate) mod share;
pub(crate) mod share_sig;

use context::Context;

pub(crate) struct Repository<'ctx> {
    context: &'ctx Context,
}

impl<'ctx> Repository<'ctx> {
    pub(crate) fn new(context: &'ctx Context) -> Self {
        Self { context }
    }
}
