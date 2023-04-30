use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::{TransactionTrait, Value};
use error::{AppResult, Error};

use crate::{data::create_file::CreateFile, repository::Repository};

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
    let (create_file, encrypted_metadata, hashed_tokens) = data.into_inner().into_active_model()?;
    let repository = Repository::new(&connection);
    let manage = repository.manage(&authenticated.user);

    let name_hash = create_file
        .name_hash
        .clone()
        .into_value()
        .unwrap()
        .unwrap::<String>();

    let file_id = match create_file.file_id.clone().into_value().unwrap() {
        Value::Int(v) => v,
        _ => None,
    };

    if manage.by_name(&name_hash, file_id).await.is_ok() {
        return Err(Error::BadRequest("file_or_directory_exists".to_string()));
    }

    let file = manage
        .create(create_file, &encrypted_metadata, hashed_tokens)
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
