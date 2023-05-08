use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::{data::authenticated::Authenticated, middleware::verify::Verify};
use context::Context;
use entity::{TransactionTrait, Uuid};
use error::{AppResult, Error};

use crate::{
    contract::StorageProvider, data::meta::Meta, repository::Repository, storage::Storage,
};

/// Method to upload file chunks to the server
///
/// Query: [crate::data::meta::Meta]
///
/// Request:
///  - Content-Type: application/octet-stream [chunk content bytes]
///  - Body: [chunk content bytes]
///
/// Response: [crate::data::app_file::AppFile]
///
/// **Note**: Chunk data is trusted as is, no validation is done on the content
/// because the content is encrypted and we cannot ensure it is the correct chunk or data.
/// Only thing we will do is compare the checksum the uploader gave us for the uploaded chunk
#[route(
    "/api/storage/{file_id}",
    method = "POST",
    wrap = "Verify::csrf_header_default()"
)]
pub(crate) async fn upload(
    req: HttpRequest,
    context: web::Data<Context>,
    meta: web::Query<Meta>,
    mut request_body: web::Bytes,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let authenticated = Authenticated::try_from(&req)?;
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let (chunk, checksum, checksum_function, key_hex) = meta.into_inner().into_tuple()?;

    let body_checksum = match checksum_function.as_str() {
        "crc16" => cryptfns::crc::crc16_digest(&request_body),
        _ => cryptfns::sha256::digest(request_body.as_ref()),
    };

    // Encrypting the payload if the encryption key is provided
    if let Some(key) = key_hex {
        let key = cryptfns::hex::decode(key)?;
        let encrypted = cryptfns::aes::encrypt(key, request_body.to_vec())?;
        request_body = web::Bytes::from(encrypted);
    }

    if checksum != body_checksum {
        let error = format!("checksum_mismatch: {checksum} != {body_checksum}");
        return Err(Error::as_validation("checksum", &error));
    }

    let connection = context.db.begin().await?;
    let repository = Repository::new(&connection);
    let storage = Storage::new(&context.config);

    let file = repository.manage(&authenticated.user).file(file_id).await?;

    let chunks = file
        .chunks
        .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

    let filename = file
        .get_filename()
        .ok_or(Error::BadRequest("file_is_dir".to_string()))?;

    if chunk > chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }

    if storage.exists(&filename, chunk).await? {
        return Err(Error::as_validation("chunk", "chunk_already_exists"));
    }

    let mut file = repository
        .manage(&authenticated.user)
        .increment(&file)
        .await?;

    if request_body.is_empty() {
        return Err(Error::BadRequest("no_file_data_received".to_string()));
    }

    storage.push(&filename, chunk, &request_body).await?;

    if file.is_file() {
        let filename = file.get_filename().unwrap();
        file.uploaded_chunks = Some(
            Storage::new(&context.config)
                .get_uploaded_chunks(&filename)
                .await?,
        );
    }

    connection.commit().await?;

    Ok(HttpResponse::Ok().json(file))
}
