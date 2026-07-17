//! Coverage for `m20260705_000005`, which relaxes `share_events.file_id` to
//! nullable.
//!
//! On SQLite the column cannot be altered in place, so the migration rebuilds the
//! table (create-copy-drop-rename). That copy is the data-loss risk: a populated,
//! hash-chained audit log must come through untouched. The SQLite tests seed such
//! a log and assert the rebuild is lossless — every row byte-identical and in
//! order, the chain still linking, and every column, index and foreign key
//! carried across (only `file_id`'s nullability flips).
//!
//! The rebuild runs under `PRAGMA foreign_keys = ON`, matching production: sqlx
//! enables enforcement by default (pinned in `sqlx_default_enforces_foreign_keys`).
//!
//! On Postgres the migration takes a different branch — `modify_column`, a
//! metadata-only `DROP NOT NULL`. `postgres_branch_is_metadata_only_not_a_rebuild`
//! renders that branch's SQL and asserts it neither rewrites the table nor moves
//! any data, so the Postgres path cannot exhibit the failure the SQLite tests guard.

use std::collections::{BTreeMap, BTreeSet};

use sea_orm_migration::prelude::*;
use sea_orm_migration::sea_orm::{
    ConnectOptions, Database, DatabaseBackend, DatabaseConnection, Statement,
};

use crate::m20260601_000004_create_share_events::ShareEvents;
use crate::Migrator;

const U1: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const U2: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const F1: &str = "ffffffff-ffff-4fff-8fff-ffffffffffff";
const E1: &str = "11111111-1111-4111-8111-111111111111";
const E2: &str = "22222222-2222-4222-8222-222222222222";
const E3: &str = "33333333-3333-4333-8333-333333333333";

/// Zero-based position of the migration under test, resolved by name so an
/// insertion anywhere else in the list can't silently point the test at the
/// wrong migration.
fn target_index() -> usize {
    Migrator::migrations()
        .iter()
        .position(|m| m.name().contains("share_events_file_id_nullable"))
        .expect("m20260705_000005 must be registered in the migrator")
}

async fn connect(name: &str) -> DatabaseConnection {
    let mut opt = ConnectOptions::new(format!("sqlite:file:{name}?mode=memory&cache=shared"));
    opt.max_connections(1).min_connections(1);
    let db = Database::connect(opt).await.expect("in-memory sqlite connect");
    db.execute_unprepared("PRAGMA foreign_keys = ON;")
        .await
        .expect("enable foreign keys");
    db
}

async fn exec(db: &DatabaseConnection, sql: &str) {
    db.execute_unprepared(sql)
        .await
        .unwrap_or_else(|e| panic!("statement failed: {sql}\n{e}"));
}

/// One user pair and one file so every `share_events` foreign key resolves.
async fn seed_parents(db: &DatabaseConnection) {
    exec(db, &format!("INSERT INTO users (id, email, created_at, updated_at) VALUES ('{U1}', 'u1@example.com', 1, 1)")).await;
    exec(db, &format!("INSERT INTO users (id, email, created_at, updated_at) VALUES ('{U2}', 'u2@example.com', 1, 1)")).await;
    exec(db, &format!("INSERT INTO files (id, name_hash, encrypted_name, mime, file_modified_at, created_at) VALUES ('{F1}', 'nh', 'en', 'text/plain', 100, 100)")).await;
}

/// Three linked audit events covering every column and every nullable case: a
/// genesis row (`prev_event_hash` NULL), a signed middle row, and a
/// system-cascade row (`recipient_id` and `sender_signature` NULL). The chain is
/// `this(E1) -> prev(E2)`, `this(E2) -> prev(E3)`.
async fn seed_chain(db: &DatabaseConnection) {
    let h1 = "AA".repeat(32);
    let h2 = "BB".repeat(32);
    let h3 = "CC".repeat(32);
    let sig1 = "11".repeat(64);
    let sig2 = "22".repeat(64);
    let cols = "id, sender_id, recipient_id, file_id, action, share_role_before, share_role_after, created_at, prev_event_hash, this_event_hash, sender_signature";

    exec(db, &format!("INSERT INTO share_events ({cols}) VALUES ('{E1}', '{U1}', '{U2}', '{F1}', 'share', NULL, 'viewer', 1000, NULL, x'{h1}', x'{sig1}')")).await;
    exec(db, &format!("INSERT INTO share_events ({cols}) VALUES ('{E2}', '{U1}', '{U2}', '{F1}', 'role_change', 'viewer', 'editor', 2000, x'{h1}', x'{h2}', x'{sig2}')")).await;
    exec(db, &format!("INSERT INTO share_events ({cols}) VALUES ('{E3}', '{U1}', NULL, '{F1}', 'revoke', 'editor', NULL, 3000, x'{h2}', x'{h3}', NULL)")).await;
}

#[derive(Debug, PartialEq)]
struct EventRow {
    id: String,
    sender_id: Option<String>,
    recipient_id: Option<String>,
    file_id: Option<String>,
    action: String,
    role_before: Option<String>,
    role_after: Option<String>,
    created_at: i64,
    prev_hash: Option<Vec<u8>>,
    this_hash: Option<Vec<u8>>,
    signature: Option<Vec<u8>>,
}

/// Blob columns are read as raw `Option<Vec<u8>>` so the comparison is exact
/// bytes and distinguishes NULL from an empty blob — `hex()` would collapse both
/// to `""`. Passing `order = ""` scans in physical (rowid) order, which is what
/// "same order" means for the audit log.
async fn fetch_events(db: &DatabaseConnection, order: &str) -> Vec<EventRow> {
    let sql = format!(
        "SELECT id, sender_id, recipient_id, file_id, action, share_role_before, share_role_after, \
         created_at, prev_event_hash, this_event_hash, sender_signature FROM share_events {order}"
    );
    db.query_all(Statement::from_string(DatabaseBackend::Sqlite, sql))
        .await
        .expect("select share_events")
        .iter()
        .map(|r| EventRow {
            id: r.try_get("", "id").unwrap(),
            sender_id: r.try_get("", "sender_id").unwrap(),
            recipient_id: r.try_get("", "recipient_id").unwrap(),
            file_id: r.try_get("", "file_id").unwrap(),
            action: r.try_get("", "action").unwrap(),
            role_before: r.try_get("", "share_role_before").unwrap(),
            role_after: r.try_get("", "share_role_after").unwrap(),
            created_at: r.try_get("", "created_at").unwrap(),
            prev_hash: r.try_get("", "prev_event_hash").unwrap(),
            this_hash: r.try_get("", "this_event_hash").unwrap(),
            signature: r.try_get("", "sender_signature").unwrap(),
        })
        .collect()
}

/// `name -> (declared_type, notnull, pk)` from `PRAGMA table_info`.
async fn columns(db: &DatabaseConnection) -> BTreeMap<String, (String, i32, i32)> {
    db.query_all(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA table_info(share_events)".to_owned(),
    ))
    .await
    .expect("table_info")
    .iter()
    .map(|r| {
        (
            r.try_get::<String>("", "name").unwrap(),
            (
                r.try_get::<String>("", "type").unwrap(),
                r.try_get::<i32>("", "notnull").unwrap(),
                r.try_get::<i32>("", "pk").unwrap(),
            ),
        )
    })
    .collect()
}

/// Named indexes (skipping SQLite's implicit PK autoindex) mapped to their
/// ordered column list.
async fn named_indexes(db: &DatabaseConnection) -> BTreeMap<String, Vec<String>> {
    let list = db
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA index_list(share_events)".to_owned(),
        ))
        .await
        .expect("index_list");

    let mut out = BTreeMap::new();
    for row in list.iter() {
        let name: String = row.try_get("", "name").unwrap();
        if !name.starts_with("idx_") {
            continue;
        }
        let cols = db
            .query_all(Statement::from_string(
                DatabaseBackend::Sqlite,
                format!("PRAGMA index_info({name})"),
            ))
            .await
            .expect("index_info")
            .iter()
            .map(|c| c.try_get::<String>("", "name").unwrap())
            .collect();
        out.insert(name, cols);
    }
    out
}

/// Foreign keys as `(from, referenced_table, referenced_col, on_delete, on_update)`.
/// A set, because `PRAGMA foreign_key_list` ordering is not guaranteed stable
/// across a rebuild.
async fn foreign_keys(
    db: &DatabaseConnection,
) -> BTreeSet<(String, String, String, String, String)> {
    db.query_all(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_key_list(share_events)".to_owned(),
    ))
    .await
    .expect("foreign_key_list")
    .iter()
    .map(|r| {
        (
            r.try_get::<String>("", "from").unwrap(),
            r.try_get::<String>("", "table").unwrap(),
            r.try_get::<Option<String>>("", "to").unwrap().unwrap_or_default(),
            r.try_get::<String>("", "on_delete").unwrap(),
            r.try_get::<String>("", "on_update").unwrap(),
        )
    })
    .collect()
}

async fn foreign_key_violations(db: &DatabaseConnection) -> usize {
    db.query_all(Statement::from_string(
        DatabaseBackend::Sqlite,
        "PRAGMA foreign_key_check".to_owned(),
    ))
    .await
    .expect("foreign_key_check")
    .len()
}

#[async_std::test]
async fn up_preserves_rows_and_schema_and_relaxes_file_id() {
    let db = connect("rebuild_up").await;
    Migrator::up(&db, Some(target_index() as u32))
        .await
        .expect("migrate up to target");
    seed_parents(&db).await;
    seed_chain(&db).await;

    let rows_before = fetch_events(&db, "").await;
    let cols_before = columns(&db).await;
    let idx_before = named_indexes(&db).await;
    let fk_before = foreign_keys(&db).await;
    assert_eq!(rows_before.len(), 3, "seed must produce three rows");
    assert_eq!(cols_before["file_id"].1, 1, "file_id starts NOT NULL");

    Migrator::up(&db, Some(1)).await.expect("apply the rebuild");

    let rows_after = fetch_events(&db, "").await;
    assert_eq!(
        rows_before, rows_after,
        "every row must survive the rebuild byte-identical and in the same order"
    );
    assert_eq!(
        foreign_key_violations(&db).await,
        0,
        "rebuilt table must have no dangling foreign keys"
    );

    let chained = fetch_events(&db, "ORDER BY created_at").await;
    assert_eq!(chained[0].prev_hash, None, "genesis event has no predecessor");
    assert_eq!(
        chained[1].prev_hash, chained[0].this_hash,
        "hash chain link E1->E2 must survive the rebuild"
    );
    assert_eq!(
        chained[2].prev_hash, chained[1].this_hash,
        "hash chain link E2->E3 must survive the rebuild"
    );

    let cols_after = columns(&db).await;
    assert_eq!(
        cols_before.keys().collect::<Vec<_>>(),
        cols_after.keys().collect::<Vec<_>>(),
        "no column may be added or dropped"
    );
    for (name, before) in &cols_before {
        let after = &cols_after[name];
        if name == "file_id" {
            assert_eq!(before.0, after.0, "file_id declared type must not change");
            assert_eq!(before.2, after.2, "file_id primary-key flag must not change");
            assert_eq!(after.1, 0, "file_id must be nullable after the rebuild");
        } else {
            assert_eq!(before, after, "column {name} must be unchanged");
        }
    }

    assert_eq!(
        idx_before,
        named_indexes(&db).await,
        "every named index must be recreated with the same columns"
    );
    assert_eq!(
        fk_before,
        foreign_keys(&db).await,
        "every foreign key (incl. file_id CASCADE) must be preserved"
    );

    // The whole point of the migration: an account-level event with no file.
    exec(&db, &format!("INSERT INTO share_events (id, sender_id, file_id, action, created_at, this_event_hash) VALUES ('{E1}0', '{U1}', NULL, 'key_rotation', 4000, x'DD')")).await;

    // The file_id FK must still bite for non-NULL values.
    let orphan = db
        .execute_unprepared(&format!("INSERT INTO share_events (id, file_id, action, created_at, this_event_hash) VALUES ('{E2}0', 'dead-beef-file', 'share', 5000, x'EE')"))
        .await;
    assert!(
        orphan.is_err(),
        "a non-NULL file_id pointing at no file must be rejected by the FK"
    );
}

#[async_std::test]
async fn down_preserves_rows_and_restores_not_null() {
    let db = connect("rebuild_down").await;
    Migrator::up(&db, Some(target_index() as u32 + 1))
        .await
        .expect("migrate up through target");
    seed_parents(&db).await;
    seed_chain(&db).await;

    let before = fetch_events(&db, "").await;
    let fk_before = foreign_keys(&db).await;
    let idx_before = named_indexes(&db).await;
    assert_eq!(before.len(), 3);
    assert_eq!(columns(&db).await["file_id"].1, 0, "file_id nullable before down()");

    Migrator::down(&db, Some(1)).await.expect("revert the rebuild");

    assert_eq!(
        before,
        fetch_events(&db, "").await,
        "down() must not lose or reorder any row"
    );
    assert_eq!(
        columns(&db).await["file_id"].1,
        1,
        "down() must restore file_id NOT NULL"
    );
    assert_eq!(idx_before, named_indexes(&db).await, "down() must keep the indexes");
    assert_eq!(fk_before, foreign_keys(&db).await, "down() must keep the foreign keys");
    assert_eq!(foreign_key_violations(&db).await, 0);

    let null_file = db
        .execute_unprepared(&format!("INSERT INTO share_events (id, sender_id, file_id, action, created_at, this_event_hash) VALUES ('{E3}0', '{U1}', NULL, 'key_rotation', 6000, x'DD')"))
        .await;
    assert!(
        null_file.is_err(),
        "after down() a NULL file_id must again violate NOT NULL"
    );
}

/// Production connects with sqlx defaults and never sets the pragma itself, so
/// this pins the assumption the rebuild tests rely on: foreign keys are enforced.
#[async_std::test]
async fn sqlx_default_enforces_foreign_keys() {
    let db = Database::connect("sqlite::memory:?mode=rwc")
        .await
        .expect("connect");
    let row = db
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA foreign_keys".to_owned(),
        ))
        .await
        .expect("query pragma")
        .expect("pragma returns a row");
    assert_eq!(
        row.try_get::<i32>("", "foreign_keys").unwrap(),
        1,
        "sqlx-sqlite must enforce foreign keys by default; the migration runs under this condition"
    );
}

/// The Postgres branch relaxes the column in place instead of rebuilding the
/// table. Rendering the exact SQL it emits proves that: `up` is an
/// `ALTER COLUMN ... DROP NOT NULL`, `down` is `SET NOT NULL`, and neither
/// recreates the table or moves rows — so the data-loss failure mode the SQLite
/// tests guard against does not exist on this path. (Executing it against a live
/// Postgres is not covered here; the metadata-only shape is what carries the risk.)
#[test]
fn postgres_branch_is_metadata_only_not_a_rebuild() {
    let up = Table::alter()
        .table(ShareEvents::Table)
        .modify_column(ColumnDef::new(ShareEvents::FileId).uuid().null())
        .to_string(PostgresQueryBuilder);
    let up_upper = up.to_uppercase();
    assert!(up_upper.contains("ALTER COLUMN"), "PG up must alter in place: {up}");
    assert!(up_upper.contains("DROP NOT NULL"), "PG up must drop NOT NULL: {up}");

    let down = Table::alter()
        .table(ShareEvents::Table)
        .modify_column(ColumnDef::new(ShareEvents::FileId).uuid().not_null())
        .to_string(PostgresQueryBuilder);
    assert!(
        down.to_uppercase().contains("SET NOT NULL"),
        "PG down must restore NOT NULL: {down}"
    );

    for forbidden in ["CREATE TABLE", "INSERT INTO", "DROP TABLE", "RENAME"] {
        assert!(
            !up_upper.contains(forbidden),
            "PG path must not rebuild the table (found {forbidden}): {up}"
        );
    }
}
