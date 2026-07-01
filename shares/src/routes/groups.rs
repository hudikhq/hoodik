//! Routes for the group surface: CRUD, role management, and roster.
//!
//! A group is a saved recipient selection — owner plus members. It carries
//! no file associations of its own; sharing to a group is a client-side
//! fan-out of individual shares to each person.
//!
//! `POST/GET/PATCH /api/shares/groups[/{id}]`,
//! `POST/DELETE /api/shares/groups/{id}/members[/{user_id}]`,
//! `PUT /api/shares/groups/{id}/members/{user_id}/role`,
//! `GET /api/shares/groups/{id}/members`.

use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};

use crate::{
    data::group::{AddGroupMemberBody, CreateGroupBody, RenameGroupBody, SetMemberRoleBody},
    repository::Repository,
    routes::gate,
};

#[route("/api/shares/groups", method = "POST")]
pub(crate) async fn create_group(
    context: web::Data<Context>,
    authenticated: Authenticated,
    body: web::Json<CreateGroupBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let repository = Repository::new(&context);
    let group = repository
        .create_group(&authenticated.user, body.into_inner())
        .await?;
    Ok(HttpResponse::Created().json(group))
}

#[route("/api/shares/groups", method = "GET")]
pub(crate) async fn list_groups(
    context: web::Data<Context>,
    authenticated: Authenticated,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let repository = Repository::new(&context);
    let response = repository.list_groups(&authenticated.user).await?;
    Ok(HttpResponse::Ok().json(response))
}

#[route("/api/shares/groups/{id}", method = "PATCH")]
pub(crate) async fn rename_group(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<String>,
    body: web::Json<RenameGroupBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let group_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    let group = repository
        .rename_group(&authenticated.user, group_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(group))
}

#[route("/api/shares/groups/{id}", method = "DELETE")]
pub(crate) async fn delete_group(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let group_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    repository
        .delete_group(&authenticated.user, group_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[route("/api/shares/groups/{id}/members", method = "POST")]
pub(crate) async fn add_group_member(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<String>,
    body: web::Json<AddGroupMemberBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let group_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    repository
        .add_group_member(&authenticated.user, group_id, body.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// `GET /api/shares/groups/{id}/members` — the full recipient set the
/// client fans a share out to: the group owner plus every member, each with
/// the pubkey + fingerprint needed to wrap a file key. The owner is a valid
/// recipient when a non-owner member initiates the share, so they are part
/// of the returned set.
#[route("/api/shares/groups/{id}/members", method = "GET")]
pub(crate) async fn group_members(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let group_id = Uuid::parse_str(&path.into_inner())
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    let members = repository
        .group_members_roster(&authenticated.user, group_id)
        .await?;
    Ok(HttpResponse::Ok().json(members))
}

#[route("/api/shares/groups/{id}/members/{user_id}/role", method = "PUT")]
pub(crate) async fn set_member_role(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<(String, String)>,
    body: web::Json<SetMemberRoleBody>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let (group_id_str, member_id_str) = path.into_inner();
    let group_id = Uuid::parse_str(&group_id_str)
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let member_id = Uuid::parse_str(&member_id_str)
        .map_err(|_| Error::BadRequest("member_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    repository
        .set_member_role(&authenticated.user, group_id, member_id, body.into_inner())
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

#[route("/api/shares/groups/{id}/members/{user_id}", method = "DELETE")]
pub(crate) async fn remove_group_member(
    context: web::Data<Context>,
    authenticated: Authenticated,
    path: web::Path<(String, String)>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    gate::ensure_enabled(&context).await?;
    let (group_id_str, member_id_str) = path.into_inner();
    let group_id = Uuid::parse_str(&group_id_str)
        .map_err(|_| Error::BadRequest("group_id_invalid".to_string()))?;
    let member_id = Uuid::parse_str(&member_id_str)
        .map_err(|_| Error::BadRequest("member_id_invalid".to_string()))?;
    let repository = Repository::new(&context);
    repository
        .remove_group_member(&authenticated.user, group_id, member_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}
