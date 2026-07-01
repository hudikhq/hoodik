//! Per-(sender, recipient) tree cap: 100 distinct `root_file_id` grants
//! over the last 24 hours. Counted in
//! the durable `share_events` table — a server restart keeps the limit
//! intact, unlike the nonce cache.
use std::collections::HashSet;

use entity::{
    share_events, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Uuid,
};
use error::AppResult;

const WINDOW_SECONDS: i64 = 24 * 60 * 60;
const TREES_PER_PAIR: usize = 100;

const GRANT_ACTIONS: [&str; 2] = ["grant", "shared_by_co_owner"];

/// True when adding `root_file_id` to the existing grant trees from this
/// sender to this recipient over the last 24 hours would exceed the
/// 100-tree-per-pair cap. The proposed root is included so a brand-new
/// tree request increments the counter correctly.
pub(crate) async fn over_per_pair_cap<C: ConnectionTrait>(
    db: &C,
    sender_id: Uuid,
    recipient_id: Uuid,
    root_file_id: Uuid,
    now: i64,
) -> AppResult<bool> {
    let mut trees: HashSet<Uuid> = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(sender_id))
        .filter(share_events::Column::RecipientId.eq(recipient_id))
        .filter(share_events::Column::CreatedAt.gte(now - WINDOW_SECONDS))
        .filter(share_events::Column::Action.is_in(GRANT_ACTIONS))
        .select_only()
        .column(share_events::Column::FileId)
        .into_tuple::<Uuid>()
        .all(db)
        .await?
        .into_iter()
        .collect();
    trees.insert(root_file_id);
    Ok(trees.len() > TREES_PER_PAIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_constants() {
        assert_eq!(TREES_PER_PAIR, 100);
        assert_eq!(WINDOW_SECONDS, 86_400);
    }
}
