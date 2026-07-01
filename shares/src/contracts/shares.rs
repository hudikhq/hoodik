use crate::data::incoming::IncomingShareQuery;

/// Pagination defaults shared between `/api/shares/mine` and
/// `/api/shares/mine/by/{user_id}`. The page size is capped at
/// 200 items; an unset or out-of-range value falls back to the default 50.
pub(crate) trait IncomingQueryExt {
    fn resolved_limit(&self) -> u64;
    fn resolved_offset(&self) -> u64;
}

impl IncomingQueryExt for IncomingShareQuery {
    fn resolved_limit(&self) -> u64 {
        self.limit.unwrap_or(50).clamp(1, 200)
    }

    fn resolved_offset(&self) -> u64 {
        self.offset.unwrap_or(0)
    }
}
