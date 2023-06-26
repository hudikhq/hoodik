use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::TransactionTrait;
use error::{AppResult, Error};

use crate::{data::create_file::CreateFile, repository::Repository};

/// Create a file or get the file context to resume the upload
///
/// Request: [crate::data::create_file::CreateFile]
///
/// Response: [crate::data::app_file::AppFile]
#[route("/api/storage", method = "POST")]
pub(crate) async fn create(
    claims: Claims,
    context: web::Data<Context>,
    data: web::Json<CreateFile>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let connection = context.db.begin().await?;
    let (create_file, encrypted_metadata, hashed_tokens, file_size, file_id) =
        data.into_inner().into_active_model()?;

    let repository = Repository::new(&connection);

    if let Some(quota) = claims.get_quota(&context).await {
        let used_space = repository.query(claims.sub).used_space().await? + file_size;

        if used_space > quota as i64 {
            return Err(Error::BadRequest("quota_exceeded".to_string()));
        }
    }

    let manage = repository.manage(claims.sub);

    let name_hash = create_file
        .name_hash
        .clone()
        .into_value()
        .unwrap()
        .unwrap::<String>();

    if manage.by_name(&name_hash, file_id).await.is_ok() {
        return Err(Error::BadRequest("file_or_directory_exists".to_string()));
    }

    let file = manage
        .create(create_file, &encrypted_metadata, hashed_tokens)
        .await?;

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
