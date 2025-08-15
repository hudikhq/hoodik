use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::{prelude::*, MAX_CHUNK_SIZE_BYTES};

use crate::{
    data::{app_file::AppFile, meta::Meta},
    repository::{cached::get_file, Repository},
};

/// Method to upload file chunks to the server
///
/// Query: [crate::data::meta::Meta]
///
/// Request:
///  - Content-Type: application/octet-stream (chunk content bytes)
///  - Body: (chunk content bytes)
///
/// Response: [crate::data::app_file::AppFile]
///
/// **Note**: Chunk data is trusted as is, no validation is done on the content
/// because the content is encrypted and we cannot ensure it is the correct chunk or data.
/// Only thing we will do is compare the checksum the uploader gave us for the uploaded chunk
/// to verify if we received the payload sender wanted to give us.
#[route("/api/storage/{file_id}", method = "POST")]
pub(crate) async fn upload(
    req: HttpRequest,
    claims: Claims,
    context: web::Data<Context>,
    meta: web::Query<Meta>,
    mut request_body: web::Bytes,
) -> AppResult<HttpResponse> {
    if request_body.is_empty() {
        return Err(Error::BadRequest("no_file_data_received".to_string()));
    }

    let context = context.into_inner();
    let file_id: String = util::actix::path_var(&req, "file_id")?;
    let file_id = Uuid::from_str(&file_id)?;
    let (chunk, checksum, checksum_function, key_hex) = meta.into_inner().into_tuple()?;

    validate_checksum(checksum, checksum_function, &request_body)?;

    if let Some(key) = key_hex {
        request_body = encrypt_request_body(&key, request_body)?;
    }

    let storage = Fs::new(&context.config);

    let mut file = get_file(&context, claims.sub, file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    validate_chunk_size(&file, chunk, request_body.len())?;

    let chunks = file
        .chunks
        .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

    if chunk > chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }

    if storage.exists(&file, chunk).await? {
        return Err(Error::as_validation("chunk", "chunk_already_exists"));
    }

    storage.push(&file, chunk, &request_body).await?;

    if file.is_file() {
        let chunks = storage.get_uploaded_chunks(&file).await?;

        file.chunks_stored = Some(chunks.len() as i64);
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

/// Run the checksum validation based on the given function
/// and checksum from the request data.
fn validate_checksum(
    checksum: Option<String>,
    checksum_function: Option<String>,
    data: &[u8],
) -> AppResult<()> {
    let body_checksum = match checksum_function.as_deref() {
        Some("crc16") => Some(cryptfns::crc::crc16_digest(data)),
        Some("sha256") => Some(cryptfns::sha256::digest(data)),
        _ => None,
    };

    if let Some(body_checksum) = body_checksum.as_deref() {
        if checksum.as_deref() != Some(body_checksum) {
            let error = format!(
                "checksum_mismatch: '{}' != '{}'",
                checksum.unwrap_or_default(),
                body_checksum
            );

            return Err(Error::as_validation("checksum", &error));
        }
    } else {
        log::warn!("Not validating uploaded chunk checksum");
    }

    Ok(())
}

/// This is option that will most likely never be used, but its here just in case.
/// Basically, this takes the key in hex from the uploader and then performs
/// the encryption of the data on the server.
///
/// This is less secure option that might be used in case uploader is
/// uploading data from a toaster or something else less performant.
fn encrypt_request_body(key: &str, request_body: web::Bytes) -> AppResult<web::Bytes> {
    let key = cryptfns::hex::decode(key)?;
    let encrypted = cryptfns::aes::encrypt(key, request_body.to_vec())?;

    Ok(web::Bytes::from(encrypted))
}

/// Validate the chunk size of the uploaded chunk.
/// This is done by comparing the size of the chunk to the size of the file
/// and the number of chunks the file should have.
/// If the chunk size is not equal to the size of the file divided by the number of chunks
/// then we know that the chunk is not the last chunk and we can validate the size.
fn validate_chunk_size(_file: &AppFile, _chunk: i64, data_len: usize) -> AppResult<()> {
    let max_size = MAX_CHUNK_SIZE_BYTES as f64 + (MAX_CHUNK_SIZE_BYTES as f64 * 0.01);

    if data_len as f64 > max_size {
        let error =
            format!("chunk_size_mismatch: expected max {max_size}, but received {data_len}");

        return Err(Error::as_validation("chunk", &error));
    }

    Ok(())
}
