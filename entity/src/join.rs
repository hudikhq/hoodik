use sea_orm::prelude::*;
use sea_orm::sea_query::{Alias, Expr, IntoIden, SelectExpr, SelectStatement};
use sea_orm::{EntityTrait, QueryTrait};

/// Helper found in this discussion https://github.com/SeaQL/sea-orm/discussions/1502
/// that allows us to select all columns from a table with a prefix and stack them together
/// so we can easily join multiple tables together.
///
/// Example query build:
/// ```
/// use entity::{RelationTrait, ColumnTrait, EntityTrait, QuerySelect, QueryFilter, QueryTrait};
///
/// let id = entity::Uuid::new_v4();
/// let mut selector = entity::links::Entity::find().select_only();
///
/// entity::join::add_columns_with_prefix::<_, entity::links::Entity>(&mut selector, "link");
/// entity::join::add_columns_with_prefix::<_, entity::users::Entity>(&mut selector, "user");
/// entity::join::add_columns_with_prefix::<_, entity::files::Entity>(&mut selector, "file");
///
/// selector
///     .filter(entity::links::Column::Id.eq(id))
///     .join(entity::JoinType::LeftJoin, entity::links::Relation::Users.def())
///     .join(entity::JoinType::LeftJoin, entity::links::Relation::Files.def())
///     .build(entity::DbBackend::Sqlite)
///     .to_string();
/// ```
///
/// Example output:
/// ```sql
/// SELECT "links"."id" AS "linkid",
///        "links"."user_id" AS "linkuser_id",
///        "links"."file_id" AS "linkfile_id",
///        "links"."signature" AS "linksignature",
///        "links"."downloads" AS "linkdownloads",
///        "links"."encrypted_name" AS "linkencrypted_name",
///        "links"."encrypted_link_key" AS "linkencrypted_link_key",
///        "links"."encrypted_thumbnail" AS "linkencrypted_thumbnail",
///        "links"."encrypted_file_key" AS "linkencrypted_file_key",
///        "links"."created_at" AS "linkcreated_at",
///        "links"."expires_at" AS "linkexpires_at",
///        "users"."id" AS "userid",
///        "users"."email" AS "useremail",
///        "users"."password" AS "userpassword",
///        "users"."secret" AS "usersecret",
///        "users"."pubkey" AS "userpubkey",
///        "users"."fingerprint" AS "userfingerprint",
///        "users"."encrypted_private_key" AS "userencrypted_private_key",
///        "users"."email_verified_at" AS "useremail_verified_at",
///        "users"."created_at" AS "usercreated_at",
///        "users"."updated_at" AS "userupdated_at",
///        "files"."id" AS "fileid",
///        "files"."name_hash" AS "filename_hash",
///        "files"."mime" AS "filemime",
///        "files"."size" AS "filesize",
///        "files"."chunks" AS "filechunks",
///        "files"."chunks_stored" AS "filechunks_stored",
///        "files"."file_id" AS "filefile_id",
///        "files"."file_created_at" AS "filefile_created_at",
///        "files"."created_at" AS "filecreated_at",
///        "files"."finished_upload_at" AS "filefinished_upload_at"
/// FROM "links"
/// LEFT JOIN "users" ON "links"."user_id" = "users"."id"
/// LEFT JOIN "files" ON "links"."file_id" = "files"."id"
/// WHERE "links"."id" = '5c0037e0-0b9f-460a-8e78-ec14a20387f8'
/// ```
pub fn add_columns_with_prefix<S: QueryTrait<QueryStatement = SelectStatement>, T: EntityTrait>(
    selector: &mut S,
    prefix: &'static str,
) {
    add_columns_with_prefix_nulling::<S, T>(selector, prefix, &[])
}

/// Same as [`add_columns_with_prefix`], but the `nulled` columns are
/// selected as a literal `NULL` instead of their stored value.
///
/// The projection still carries every alias the row's `FromQueryResult`
/// expects, so a large blob column can be dropped from a listing without
/// the database ever reading it off the page. Pair it with a computed
/// flag (`<column> IS NOT NULL`) when the caller still needs to know
/// whether a value was there.
///
/// The NULL is cast to `TEXT` because Postgres types a bare NULL literal
/// as `unknown`, which the row decoder rejects. Only string-typed columns
/// may be nulled through this helper.
pub fn add_columns_with_prefix_nulling<
    S: QueryTrait<QueryStatement = SelectStatement>,
    T: EntityTrait,
>(
    selector: &mut S,
    prefix: &'static str,
    nulled: &[&str],
) {
    for col in <T::Column as sea_orm::entity::Iterable>::iter() {
        let name = col.to_string();
        let alias = format!("{}{}", prefix, name);

        let expr = match nulled.contains(&name.as_str()) {
            true => Expr::cust("CAST(NULL AS TEXT)"),
            false => col.select_as(col.into_expr()),
        };

        selector.query().expr(SelectExpr {
            expr,
            alias: Some(Alias::new(&alias).into_iden()),
            window: None,
        });
    }
}

/// Append a computed `<column> IS NOT NULL` boolean under `alias`.
///
/// Lets a projection report that a value exists without selecting it —
/// the companion to nulling a blob column out of a listing.
pub fn add_not_null_flag<S: QueryTrait<QueryStatement = SelectStatement>, T: EntityTrait>(
    selector: &mut S,
    column: T::Column,
    alias: &str,
) {
    selector.query().expr(SelectExpr {
        expr: Expr::col((T::default(), column)).is_not_null(),
        alias: Some(Alias::new(alias).into_iden()),
        window: None,
    });
}
