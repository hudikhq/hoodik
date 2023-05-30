use sea_orm::prelude::*;
use sea_orm::sea_query::{Alias, IntoIden, SelectExpr, SelectStatement};
use sea_orm::{EntityTrait, QueryTrait};

/// Helper found in this discussion https://github.com/SeaQL/sea-orm/discussions/1502
/// that allows us to select all columns from a table with a prefix and stack them together
/// so we can easily join multiple tables together.
///
/// Example query build:
/// ```
/// let mut selector = links::Entity::find().select_only();
///
/// entity::join::add_columns_with_prefix::<_, links::Entity>(&mut selector, "link");
/// entity::join::add_columns_with_prefix::<_, users::Entity>(&mut selector, "user");
/// entity::join::add_columns_with_prefix::<_, files::Entity>(&mut selector, "file");
///
/// selector
///     .filter(links::Column::Id.eq(id))
///     .join(JoinType::LeftJoin, links::Relation::Users.def())
///     .join(JoinType::LeftJoin, links::Relation::Files.def())
///     .build(DbBackend::Sqlite)
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
    for col in <T::Column as sea_orm::entity::Iterable>::iter() {
        let alias = format!("{}{}", prefix, col.to_string());

        selector.query().expr(SelectExpr {
            expr: col.select_as(col.into_expr()),
            alias: Some(Alias::new(&alias).into_iden()),
            window: None,
        });
    }
}
