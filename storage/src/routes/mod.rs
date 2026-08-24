//! # Storage routes
//!
//! Routes for manipulating files and folders, plus the chunked upload and
//! download endpoints. Sharing routes live in the `links` crate.

pub mod chunk_urls;
pub mod create;
pub mod delete;
pub mod delete_many;
pub mod download;
pub mod extra_tokens;
pub mod index;
pub mod metadata;
pub mod move_many;
pub mod name_hash;
pub mod reindex;
pub mod rename;
pub mod replace_content;
pub mod search;
pub mod set_editable;
pub mod stats;
pub mod thumbnail;
pub mod update_hashes;
pub mod upload;
pub(crate) mod upload_tar;

/// Refuse `?format=tar` when the operator has switched the archive off.
///
/// Clients read the capability and skip the archive without asking, so this
/// answers the ones that did not: an older app that only learns a server's
/// shape by trying, or anything hand-rolled. 501 rather than 404 says the
/// route exists and this deployment will not serve it — and every client that
/// falls back does so on any status from a `?format=tar` URL, so the code is
/// for the reader, not the fallback.
pub(crate) fn reject_tar_when_disabled(context: &context::Context) -> error::AppResult<()> {
    if context.config.app.tar_transfer_disabled {
        return Err(error::Error::NotImplemented(
            "tar_transfer_disabled".to_string(),
        ));
    }
    Ok(())
}
pub mod versions;

/// Register the storage routes
/// on to the application server
pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(chunk_urls::download_urls);
    cfg.service(chunk_urls::upload_urls);
    cfg.service(chunk_urls::finalize);
    cfg.service(chunk_urls::version_urls);
    cfg.service(create::create);
    cfg.service(delete_many::delete_many);
    cfg.service(delete::delete);
    // Registers before the catch-all `GET /api/storage/{file_id}` below —
    // actix-web walks services in registration order, so the download route
    // otherwise matches this path with `file_id = "reindex"` and fails on the
    // UUID parse.
    cfg.service(reindex::pending);
    cfg.service(download::download);
    cfg.service(download::head);
    cfg.service(extra_tokens::extra_tokens);
    cfg.service(index::index);
    cfg.service(metadata::metadata);
    cfg.service(move_many::move_many);
    cfg.service(name_hash::name_hash);
    cfg.service(reindex::reindex);
    cfg.service(rename::rename);
    cfg.service(replace_content::replace_content);
    cfg.service(search::search);
    cfg.service(set_editable::set_editable);
    cfg.service(stats::stats);
    cfg.service(thumbnail::thumbnail);
    cfg.service(update_hashes::update_hashes);
    cfg.service(upload::upload);
    cfg.service(versions::list);
    cfg.service(versions::download);
    cfg.service(versions::restore);
    cfg.service(versions::fork);
    cfg.service(versions::delete);
    cfg.service(versions::purge_all_history);
}
