use std::str::FromStr;

use actix_web::{route, web, HttpRequest, HttpResponse};
use auth::data::transfer_claims::StorageClaims;
use context::Context;
use entity::Uuid;
use error::{AppResult, Error};
use fs::{prelude::*, MAX_CHUNK_SIZE_BYTES};

use crate::{
    data::{app_file::AppFile, meta::Meta},
    repository::{
        cached::{evict_file, get_file},
        Repository,
    },
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
    claims: StorageClaims,
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
    claims.validate_transfer_path(file_id, "upload")?;
    let (chunk, checksum, checksum_function, key_hex) = meta.into_inner().into_tuple()?;

    validate_checksum(checksum, checksum_function, &request_body)?;

    if let Some(key) = key_hex {
        request_body = encrypt_request_body(&key, request_body)?;
    }

    let storage = Fs::new(&context.config);

    let mut file = get_file(&context, claims.sub(), file_id)
        .await
        .ok_or_else(|| Error::NotFound("file_not_found".to_string()))?;

    validate_chunk_size(&file, chunk, request_body.len())?;

    let versioned = file.use_versioned_layout();

    // Target version only matters on the versioned path. Non-editable
    // files never snapshot/swap; their chunks live directly in the
    // legacy flat layout and finalize only stamps `finished_upload_at`.
    let target_chunks = file
        .target_chunks()
        .ok_or(Error::BadRequest("file_has_no_chunks".to_string()))?;

    if chunk > target_chunks {
        return Err(Error::as_validation("chunk", "chunk_out_of_range"));
    }

    let chunk_exists = if versioned {
        storage.exists_v(&file, file.target_version(), chunk).await?
    } else {
        storage.exists(&file, chunk).await?
    };
    if chunk_exists {
        return Err(Error::as_validation("chunk", "chunk_already_exists"));
    }

    if versioned {
        storage
            .push_v(&file, file.target_version(), chunk, &request_body)
            .await?;
    } else {
        storage.push(&file, chunk, &request_body).await?;
    }

    if file.is_file() {
        let stored = if versioned {
            storage
                .get_uploaded_chunks_v(&file, file.target_version())
                .await?
        } else {
            storage.get_uploaded_chunks(&file).await?
        };

        file.chunks_stored = Some(stored.len() as i64);
        file.uploaded_chunks = Some(stored);
    }

    if file.chunks_stored == Some(target_chunks) {
        // On the versioned (edit) path, snapshot insert + pointer swap +
        // prune commit together so a crash mid-finalize can't leave history
        // pointing at a version that doesn't exist. On the non-versioned
        // path `finish` only stamps `finished_upload_at` — the transaction
        // is redundant there, but keeping it uniform avoids a branch.
        use entity::TransactionTrait;
        let txn = context.db.begin().await?;
        let (mut finished_file, pruned) = Repository::new(&txn)
            .manage(claims.sub())
            .finish(&file, context.config.app.max_file_versions)
            .await?;
        txn.commit().await?;

        // Stale cache would point any immediate follow-up read at the
        // pre-swap version.
        evict_file(file_id).await;

        for version in pruned {
            if let Err(e) = storage.purge_version(&finished_file, version).await {
                log::warn!(
                    "Failed to purge pruned v{} for file {}: {}. \
                     On-disk garbage; caught by next purge_all on file delete.",
                    version,
                    file_id,
                    e
                );
            }
        }

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
