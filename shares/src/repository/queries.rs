//! Read-side helpers shared between the recipient list, the audit-events
//! query, and `GET /api/shares/mine`. Each call runs at most two SQL
//! statements: one to fetch the rows, one to batch-load auxiliary user
//! data (granter email, owner identity) for the rows actually returned.

use std::collections::{HashMap, HashSet};

use entity::{
    files, share_events, user_files, users, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, Expr,
    FromQueryResult, IntoCondition, JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder,
    QueryResult, QuerySelect, RelationTrait, Uuid,
};
use error::AppResult;

use crate::data::{
    app_share::AppShare,
    incoming::IncomingShare,
    share_event::{AppShareEvent, AuditUserRef},
};

/// All non-owner recipient rows for `file_id`, ordered by share time.
pub(crate) async fn recipient_list<C: ConnectionTrait>(
    db: &C,
    file_id: Uuid,
) -> AppResult<Vec<AppShare>> {
    let rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(file_id))
        .filter(user_files::Column::IsOwner.eq(false))
        .order_by(user_files::Column::CreatedAt, Order::Asc)
        .all(db)
        .await?;

    let user_ids: HashSet<Uuid> = rows
        .iter()
        .map(|r| r.user_id)
        .chain(rows.iter().filter_map(|r| r.shared_by_user_id))
        .collect();
    let users_by_id = lookup_users(db, &user_ids).await?;

    let shares = rows
        .into_iter()
        .map(|row| {
            let recipient = users_by_id
                .get(&row.user_id)
                .cloned()
                .map(|u| (u.email, u.fingerprint))
                .unwrap_or_default();
            let granter_email = row
                .shared_by_user_id
                .and_then(|id| users_by_id.get(&id))
                .map(|u| u.email.clone());

            AppShare {
                file_id: row.file_id,
                recipient_id: row.user_id,
                recipient_email: recipient.0,
                recipient_pubkey_fingerprint: recipient.1,
                share_role: row.share_role,
                created_at: row.created_at,
                shared_at: row.shared_at,
                shared_by_user_id: row.shared_by_user_id,
                shared_by_email: granter_email,
            }
        })
        .collect();
    Ok(shares)
}

/// Paginated "shared with me" list for `recipient_id`. Optional
/// `sender_filter` narrows to grants from one specific granter.
///
/// Returns only the *roots* of every share the recipient has — rows whose
/// parent directory is either unknown to the recipient or NULL. The
/// recipient inherits descendants implicitly: when a folder is shared,
/// every descendant gets its own `user_files` row keyed against the
/// recipient (so RSA wraps land per-user), but surfacing all of them at
/// the synthetic "Shared with me" root would flatten the tree.
/// `__shared_with_me__` lists the entry points; navigation drills into the
/// real folder ids from there.
pub(crate) async fn incoming_for_recipient<C: ConnectionTrait>(
    db: &C,
    recipient_id: Uuid,
    sender_filter: Option<Uuid>,
    limit: u64,
    offset: u64,
) -> AppResult<(Vec<IncomingShare>, u64)> {
    let mut all_query = user_files::Entity::find()
        .filter(user_files::Column::UserId.eq(recipient_id))
        .filter(user_files::Column::IsOwner.eq(false));
    if let Some(sender) = sender_filter {
        all_query = all_query.filter(user_files::Column::SharedByUserId.eq(sender));
    }
    let all_rows = all_query
        .order_by(user_files::Column::SharedAt, Order::Desc)
        .order_by(user_files::Column::CreatedAt, Order::Desc)
        .all(db)
        .await?;

    let recipient_file_ids: HashSet<Uuid> = all_rows.iter().map(|r| r.file_id).collect();
    let parents_by_file =
        parent_pointers_for_files(db, recipient_file_ids.iter().copied()).await?;

    let mut root_rows: Vec<user_files::Model> = all_rows
        .into_iter()
        .filter(|row| match parents_by_file.get(&row.file_id) {
            None => true,
            Some(None) => true,
            Some(Some(parent_id)) => !recipient_file_ids.contains(parent_id),
        })
        .collect();
    let total = root_rows.len() as u64;
    let start = offset as usize;
    let end = start.saturating_add(limit as usize).min(root_rows.len());
    let rows: Vec<user_files::Model> = if start >= root_rows.len() {
        Vec::new()
    } else {
        root_rows.drain(start..end).collect()
    };

    let owner_rows = owner_rows_for_files(db, rows.iter().map(|r| r.file_id)).await?;
    let file_metas = file_metas_for_files(db, rows.iter().map(|r| r.file_id)).await?;

    let extra_user_ids: HashSet<Uuid> = owner_rows
        .values()
        .map(|r| r.user_id)
        .chain(rows.iter().filter_map(|r| r.shared_by_user_id))
        .collect();
    let users_by_id = lookup_users(db, &extra_user_ids).await?;

    let items = rows
        .into_iter()
        .map(|row| {
            let owner_row = owner_rows.get(&row.file_id);
            let owner = owner_row.and_then(|r| users_by_id.get(&r.user_id));
            let granter_email = row
                .shared_by_user_id
                .and_then(|id| users_by_id.get(&id))
                .map(|u| u.email.clone());

            let meta = file_metas.get(&row.file_id);
            IncomingShare {
                file_id: row.file_id,
                mime: meta.map(|m| m.mime.clone()).unwrap_or_default(),
                encrypted_name: meta.map(|m| m.encrypted_name.clone()).unwrap_or_default(),
                encrypted_thumbnail: meta.and_then(|m| m.encrypted_thumbnail.clone()),
                cipher: meta.map(|m| m.cipher.clone()).unwrap_or_default(),
                editable: meta.map(|m| m.editable).unwrap_or(false),
                size: meta.and_then(|m| m.size),
                chunks: meta.and_then(|m| m.chunks),
                chunks_stored: meta.and_then(|m| m.chunks_stored),
                finished_upload_at: meta.and_then(|m| m.finished_upload_at),
                md5: meta.and_then(|m| m.md5.clone()),
                sha1: meta.and_then(|m| m.sha1.clone()),
                sha256: meta.and_then(|m| m.sha256.clone()),
                blake2b: meta.and_then(|m| m.blake2b.clone()),
                share_role: row.share_role,
                encrypted_key: row.encrypted_key,
                created_at: row.created_at,
                shared_at: row.shared_at,
                owner_id: owner.map(|u| u.id).unwrap_or(Uuid::nil()),
                owner_email: owner.map(|u| u.email.clone()).unwrap_or_default(),
                owner_pubkey: owner.map(|u| u.pubkey.clone()).unwrap_or_default(),
                owner_key_type: owner.map(|u| u.key_type.clone()).unwrap_or_default(),
                owner_wrapping_pubkey: owner.and_then(|u| u.wrapping_pubkey.clone()),
                owner_pubkey_fingerprint: owner
                    .map(|u| u.fingerprint.clone())
                    .unwrap_or_default(),
                shared_by_user_id: row.shared_by_user_id,
                shared_by_email: granter_email,
            }
        })
        .collect();
    Ok((items, total))
}

/// User-visible slice of `share_events`. The caller sees rows they
/// authored, rows that target them, OR rows on files they own. Optional
/// `file_id` and `action` filters narrow further. The returned `users`
/// map carries the minimal identity record (id, email, key material,
/// fingerprint) for every sender and recipient referenced in the page —
/// enough for the client to label rows and verify per-row signatures.
pub(crate) async fn events_for_user<C: ConnectionTrait>(
    db: &C,
    user_id: Uuid,
    file_id: Option<Uuid>,
    action: Option<String>,
    limit: u64,
    offset: u64,
) -> AppResult<(Vec<AppShareEvent>, HashMap<Uuid, AuditUserRef>, u64)> {
    let owned_file_ids: Vec<Uuid> = user_files::Entity::find()
        .select_only()
        .column(user_files::Column::FileId)
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .into_tuple()
        .all(db)
        .await?;

    let visibility = share_events::Column::SenderId
        .eq(user_id)
        .or(share_events::Column::RecipientId.eq(user_id))
        .or(share_events::Column::FileId.is_in(owned_file_ids));

    let mut count_query = share_events::Entity::find().filter(visibility.clone());
    if let Some(fid) = file_id {
        count_query = count_query.filter(share_events::Column::FileId.eq(fid));
    }
    if let Some(act) = action.as_deref() {
        count_query = count_query.filter(share_events::Column::Action.eq(act));
    }
    let total = count_query.count(db).await?;

    // The page query runs as a single statement: share_events row + two
    // LEFT JOINs that surface the file's encrypted name/cipher and the
    // caller's wrapped key. Caller scoping on `user_files` lives in the
    // join's ON clause — pushing it into WHERE would drop rows whose
    // caller has no wrap (revoked recipient), which is exactly the
    // bare-id fallback we want the client to render.
    let mut query = share_events::Entity::find().filter(visibility).select_only();
    entity::join::add_columns_with_prefix::<_, share_events::Entity>(&mut query, "evt");
    query = query
        .column_as(files::Column::EncryptedName, "file_encrypted_name")
        .column_as(files::Column::Cipher, "file_cipher")
        .column_as(user_files::Column::EncryptedKey, "uf_encrypted_key")
        .join(JoinType::LeftJoin, share_events::Relation::Files.def())
        .join(
            JoinType::LeftJoin,
            files::Relation::UserFiles
                .def()
                .on_condition(move |_left, right| {
                    Expr::col((right, user_files::Column::UserId))
                        .eq(user_id)
                        .into_condition()
                }),
        );
    if let Some(fid) = file_id {
        query = query.filter(share_events::Column::FileId.eq(fid));
    }
    if let Some(act) = action.as_deref() {
        query = query.filter(share_events::Column::Action.eq(act));
    }
    let rows: Vec<EventRow> = query
        .order_by(share_events::Column::CreatedAt, Order::Desc)
        .order_by(share_events::Column::Id, Order::Desc)
        .limit(limit)
        .offset(offset)
        .into_model::<EventRow>()
        .all(db)
        .await?;

    let mut referenced: HashSet<Uuid> = HashSet::new();
    for row in &rows {
        if let Some(id) = row.model.sender_id {
            referenced.insert(id);
        }
        if let Some(id) = row.model.recipient_id {
            referenced.insert(id);
        }
    }
    let user_map = lookup_users(db, &referenced).await?;
    let users: HashMap<Uuid, AuditUserRef> = user_map
        .into_iter()
        .map(|(id, model)| {
            (
                id,
                AuditUserRef {
                    id: model.id,
                    email: model.email,
                    pubkey: model.pubkey,
                    key_type: model.key_type,
                    wrapping_pubkey: model.wrapping_pubkey,
                    fingerprint: model.fingerprint,
                },
            )
        })
        .collect();

    Ok((
        rows.into_iter()
            .map(|r| {
                AppShareEvent::from_parts(
                    r.model,
                    r.encrypted_name,
                    r.cipher,
                    r.encrypted_key,
                )
            })
            .collect(),
        users,
        total,
    ))
}

/// One audit-log row plus the optional LEFT JOIN material the client
/// uses to render the file's plaintext name. `encrypted_name` + `cipher`
/// come from `files`; `encrypted_key` comes from the caller's `user_files`
/// row scoped on the join predicate. Any of the three may be null.
struct EventRow {
    model: share_events::Model,
    encrypted_name: Option<String>,
    cipher: Option<String>,
    encrypted_key: Option<String>,
}

impl FromQueryResult for EventRow {
    fn from_query_result(res: &QueryResult, _pre: &str) -> Result<Self, DbErr> {
        Ok(Self {
            model: share_events::Model::from_query_result(res, "evt")?,
            encrypted_name: res.try_get("", "file_encrypted_name")?,
            cipher: res.try_get("", "file_cipher")?,
            encrypted_key: res.try_get("", "uf_encrypted_key")?,
        })
    }
}

async fn lookup_users<C: ConnectionTrait>(
    db: &C,
    ids: &HashSet<Uuid>,
) -> AppResult<HashMap<Uuid, users::Model>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = users::Entity::find()
        .filter(users::Column::Id.is_in(ids.iter().copied().collect::<Vec<_>>()))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|u| (u.id, u)).collect())
}

async fn owner_rows_for_files<C: ConnectionTrait, I: IntoIterator<Item = Uuid>>(
    db: &C,
    file_ids: I,
) -> AppResult<HashMap<Uuid, user_files::Model>> {
    let ids: Vec<Uuid> = file_ids.into_iter().collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.is_in(ids))
        .filter(user_files::Column::IsOwner.eq(true))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| (r.file_id, r)).collect())
}

/// Per-file metadata the recipient UI needs alongside the share grant.
/// Loaded in one batch by `file_metas_for_files` so each row on the
/// "Shared with me" list renders with the same shape the owner sees in
/// their own file browser — size, upload progress, checksums.
pub(crate) struct IncomingFileMeta {
    pub mime: String,
    pub encrypted_name: String,
    pub encrypted_thumbnail: Option<String>,
    pub cipher: String,
    pub editable: bool,
    pub size: Option<i64>,
    pub chunks: Option<i64>,
    pub chunks_stored: Option<i64>,
    pub finished_upload_at: Option<i64>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
}

async fn parent_pointers_for_files<C: ConnectionTrait, I: IntoIterator<Item = Uuid>>(
    db: &C,
    file_ids: I,
) -> AppResult<HashMap<Uuid, Option<Uuid>>> {
    let ids: Vec<Uuid> = file_ids.into_iter().collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = files::Entity::find()
        .filter(files::Column::Id.is_in(ids))
        .all(db)
        .await?;
    Ok(rows.into_iter().map(|r| (r.id, r.file_id)).collect())
}

async fn file_metas_for_files<C: ConnectionTrait, I: IntoIterator<Item = Uuid>>(
    db: &C,
    file_ids: I,
) -> AppResult<HashMap<Uuid, IncomingFileMeta>> {
    let ids: Vec<Uuid> = file_ids.into_iter().collect();
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = files::Entity::find()
        .filter(files::Column::Id.is_in(ids))
        .all(db)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.id,
                IncomingFileMeta {
                    mime: r.mime,
                    encrypted_name: r.encrypted_name,
                    encrypted_thumbnail: r.encrypted_thumbnail,
                    cipher: r.cipher,
                    editable: r.editable,
                    size: r.size,
                    chunks: r.chunks,
                    chunks_stored: r.chunks_stored,
                    finished_upload_at: r.finished_upload_at,
                    md5: r.md5,
                    sha1: r.sha1,
                    sha256: r.sha256,
                    blake2b: r.blake2b,
                },
            )
        })
        .collect())
}

/// Files within the subtree rooted at `scope_file_id` that `user_id`
/// owns (`is_owner = true`). Used by revoke to find a departing
/// co-owner's own files so they can be relocated to that user's root
/// instead of orphaned under a folder they can no longer reach.
pub(crate) async fn file_tree_owner_ids<C: ConnectionTrait>(
    db: &C,
    scope_file_id: Uuid,
    user_id: Uuid,
) -> AppResult<Vec<Uuid>> {
    let subtree = file_tree_ids(db, scope_file_id).await?;
    if subtree.is_empty() {
        return Ok(Vec::new());
    }
    let rows = user_files::Entity::find()
        .select_only()
        .column(user_files::Column::FileId)
        .filter(user_files::Column::UserId.eq(user_id))
        .filter(user_files::Column::IsOwner.eq(true))
        .filter(user_files::Column::FileId.is_in(subtree))
        .into_tuple::<Uuid>()
        .all(db)
        .await?;
    Ok(rows)
}

/// File subtree rooted at `root_file_id`, used for revoke cascade. Returns
/// every descendant file id (folders + leaves) plus the root itself.
///
/// The `depth` guard bounds the recursion: callers reject parent cycles
/// before they can be written (see `move_into_shared`), and this cap is the
/// backstop so a cycle that ever did exist degrades to a bounded walk on
/// both SQLite and Postgres instead of looping forever. Real folder nesting
/// is nowhere near 10000 levels deep.
pub(crate) async fn file_tree_ids<C: ConnectionTrait>(
    db: &C,
    root_file_id: Uuid,
) -> AppResult<Vec<Uuid>> {
    let sql = r#"
        WITH RECURSIVE file_tree(id, file_id, depth) AS (
            SELECT id, file_id, 0 FROM files WHERE id = $1
            UNION ALL
            SELECT child.id, child.file_id, parent.depth + 1 FROM files child
            JOIN file_tree parent ON parent.id = child.file_id
            WHERE parent.depth < 10000
        )
        SELECT id FROM file_tree;
    "#;
    let rows = files::Entity::find()
        .from_raw_sql(entity::Statement::from_sql_and_values(
            db.get_database_backend(),
            sql,
            [root_file_id.into()],
        ))
        .into_json()
        .all(db)
        .await?;
    let mut ids = Vec::with_capacity(rows.len());
    for row in rows {
        if let Some(id_str) = row.get("id").and_then(|v| v.as_str()) {
            if let Ok(id) = Uuid::parse_str(id_str) {
                ids.push(id);
            }
        }
    }
    Ok(ids)
}
