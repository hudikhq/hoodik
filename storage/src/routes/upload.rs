use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use log::warn;

use crate::{
    contract::StorageProvider,
    data::meta::Meta,
    repository::{cached::get_file, Repository},
    storage::Storage,
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
#[route("/api/storage/{file_id}", method = "POST")]
pub async fn upload(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    meta: web::Query<Meta>,
    mut request_body: web::Bytes,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let (chunk, checksum, checksum_function, key_hex) = meta.into_inner().into_tuple()?;

    let body_checksum = match checksum_function.as_deref() {
        Some("crc16") => Some(cryptfns::crc::crc16_digest(&request_body)),
        Some("sha256") => Some(cryptfns::sha256::digest(request_body.as_ref())),
        _ => None,
    };

    // Encrypting the payload if the encryption key is provided
    if let Some(key) = key_hex {
        let key = cryptfns::hex::decode(key)?;
        let encrypted = cryptfns::aes::encrypt(key, request_body.to_vec())?;
        request_body = web::Bytes::from(encrypted);
    }

    if let Some(body_checksum) = body_checksum {
        if checksum.as_ref() != Some(&body_checksum) {
            let error = format!(
                "checksum_mismatch: '{}' != '{}'",
                checksum.unwrap_or_default(),
                body_checksum
            );
            return Err(Error::as_validation("checksum", &error));
        }
    } else {
        warn!("Not validating uploaded chunk checksum");
    }

    let storage = Storage::new(&context.config);

    let mut file = get_file(&context, claims.sub, file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

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

    if request_body.is_empty() {
        return Err(Error::BadRequest("no_file_data_received".to_string()));
    }

    storage.push(&filename, chunk, &request_body).await?;

    if file.is_file() {
        let filename = file.get_filename().unwrap();
        let chunks = Storage::new(&context.config)
            .get_uploaded_chunks(&filename)
            .await?;
        file.chunks_stored = Some(chunks.len() as i32);
        file.uploaded_chunks = Some(chunks);
    }

    if file.chunks == file.chunks_stored {
        let mut finished_file = Repository::new(&context.db)
            .manage(claims.sub)
            .finish(&file)
            .await?;

        finished_file.chunks_stored = file.chunks_stored;
        finished_file.uploaded_chunks = file.uploaded_chunks;
        file = finished_file;
    }

    Ok(HttpResponse::Ok().json(file))
}
