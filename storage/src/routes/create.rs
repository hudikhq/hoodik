use actix_web::{route, web, HttpResponse};
use auth::data::claims::Claims;
use context::Context;
use entity::TransactionTrait;
use error::{AppResult, Error};
use futures::lock::Mutex;
use std::sync::OnceLock;

use crate::{data::create_file::CreateFile, repository::Repository};

/// Serializes the instance-quota reserve window so two concurrent creates that
/// each fit alone cannot jointly exceed the instance ceiling. A Hoodik instance
/// is a single process, so a process-wide lock around "read instance usage →
/// reserve → commit" gives the guarantee on every backend; SQLite (the default)
/// has no row-level `SELECT … FOR UPDATE`, so this is the portable equivalent.
/// Only taken when an instance quota is configured, so the default self-hosted
/// path is unaffected.
fn instance_quota_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

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

    // Held until the transaction commits so the reserved bytes are visible to
    // the next create before it reads the instance total.
    let _instance_guard = match context.config.app.storage_instance_quota_bytes {
        Some(instance_quota) => {
            let guard = instance_quota_lock().lock().await;
            let used_space = repository.instance_used_space().await? + file_size;

            if used_space > instance_quota as i64 {
                return Err(Error::BadRequest("quota_exceeded".to_string()));
            }

            Some(guard)
        }
        None => None,
    };

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
