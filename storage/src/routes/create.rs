use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::TransactionTrait;
use error::{AppResult, Error};

use crate::{
    contract::StorageProvider, data::create_file::CreateFile, repository::Repository,
    storage::Storage,
};

/// Create a file or get the file context to resume the upload
///
/// Request: [crate::data::create_file::CreateFile]
///
/// Response: [crate::data::app_file::AppFile]
#[route(
    "/api/storage",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub async fn create(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<CreateFile>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let connection = context.db.begin().await?;
    let (create_file, encrypted_key) = data.into_inner().into_active_model()?;

    let file = Repository::new(&connection)
        .manage(&authenticated.user)
        .create(create_file, &encrypted_key)
        .await?;

    let filename = file
        .get_filename()
        .ok_or(Error::BadRequest("file_is_dir".to_string()))?;

    Storage::new(&context.config).get_or_create(&filename)?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
