use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::TransactionTrait;
use error::{AppResult, Error};

use crate::{contract::StorageProvider, repository::Repository, storage::Storage};

/// Delete a file or directory by its id
#[route(
    "/api/storage/{file_id}",
    method = "DELETE",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn delete(req: HttpRequest, context: web::Data<Context>) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id = util::actix::path_var(&req, "file_id")?;

    let connection = context.db.begin().await?;

    let repository = Repository::new(&connection);
    let manage = repository.manage(&authenticated.user);

    let mut file = manage.get(file_id).await?;

    if !file.is_owner {
        return Err(Error::Unauthorized("not_your_file".to_string()));
    }

    file = manage.delete(file.id).await?;

    if file.is_file() {
        Storage::new(&context.config).purge(&file.get_filename().unwrap())?;
    }

    connection.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
