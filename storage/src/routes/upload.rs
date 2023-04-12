use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::TransactionTrait;
use error::{AppResult, Error};

use crate::{
    contract::StorageProvider, data::meta::Meta, repository::Repository, storage::Storage,
};

/// Method to upload file chunks to the server
///
/// Query: [crate::data::meta::Meta]
///
/// Request:
///  - Content-Type: multipart/form-data
///  - Field name: file
///  - File Content-Type: application/octet-stream [chunk content bytes]
///
/// Response: [crate::data::app_file::AppFile]
///
/// **Note**: Chunk data is trusted as is, no validation is done on the content
/// because the content is encrypted and we cannot ensure it is the correct chunk or data.
#[route(
    "/api/storage/{file_id}",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
async fn upload(
    req: HttpRequest,
    context: web::Data<Context>,
    meta: web::Query<Meta>,
    request_body: web::Bytes,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id: i32 = util::actix::path_var(&req, "file_id")?;
    let (chunk, checksum) = meta.into_inner().into_tuple()?;

    let body_checksum = cryptfns::sha256::digest(request_body.as_ref());

    if checksum != body_checksum {
        let error = format!("checksum_mismatch: {} != {}", checksum, body_checksum);
        return Err(Error::as_validation("checksum", &error));
    }

    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let storage = Storage::new(&context.config);

    let file = repository.manage(&authenticated.user).file(file_id).await?;

    let chunks = file
        .chunks
        .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

    let chunks_stored = file
        .chunks_stored
        .ok_or(Error::BadRequest("file_has_no_chunks_stored".to_string()))?;

    let filename = file
        .get_filename()
        .ok_or(Error::BadRequest("file_is_dir".to_string()))?;

    if chunk > chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }

    if storage.part_exists(&filename, chunk)? {
        return Err(Error::as_validation("chunk", "chunk_already_exists"));
    }

    let file = repository
        .manage(&authenticated.user)
        .increment(&file)
        .await?;

    if request_body.is_empty() {
        return Err(Error::BadRequest("no_file_data_received".to_string()));
    }

    storage.push_part(&filename, chunk, &request_body)?;

    if chunks == chunks_stored + 1 {
        storage.concat_files(&filename, chunks as u64)?;
    }

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
