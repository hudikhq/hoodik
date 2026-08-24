use context::Context;
use entity::{file_tokens::SearchTags, users, Uuid};
use error::AppResult;

use crate::{
    data::{app_file::AppFile, create_file::CreateFile},
    repository::Repository,
};

/// Stand-in for the account-wide search key a real client derives from its
/// private key. Tests only need it to be fixed, so that what they index and
/// what they later query agree.
pub fn search_key() -> [u8; 32] {
    [7u8; 32]
}

/// Per-file search key, derived from the file's name so each mock file gets a
/// distinct one without the test having to carry key material around.
pub fn file_search_key(name: &str) -> [u8; 32] {
    cryptfns::search::file_key(name.as_bytes()).unwrap()
}

/// Tags in the `"{tag}:{weight}"` form the index routes accept.
pub fn index_tags(key: &[u8], text: &str) -> Vec<String> {
    cryptfns::tokenizer::into_string(cryptfns::search::tag_tokens(key, text).unwrap())
        .split(';')
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Bare tags, as a client sends them to `/api/storage/search`.
pub fn query_tags(key: &[u8], text: &str) -> Vec<String> {
    cryptfns::search::tag_tokens(key, text)
        .unwrap()
        .into_iter()
        .map(|token| token.token)
        .collect()
}

pub async fn create_file(
    context: &Context,
    user: &users::Model,
    name: &str,
    file_id: Option<Uuid>,
    mime: Option<&str>,
) -> AppResult<AppFile> {
    let repository = Repository::new(&context.db);
    let mut size = None;
    let mut chunks = None;

    if mime != Some("dir") {
        size = Some(100);
        chunks = Some(1);
    }

    let root_key = search_key();
    let file_key = file_search_key(name);

    let file = CreateFile {
        encrypted_key: Some(name.to_string()),
        encrypted_name: Some(name.to_string()),
        encrypted_thumbnail: None,
        search_tokens_root: Some(index_tags(&root_key, name)),
        search_tokens_file: Some(index_tags(&file_key, name)),
        mime: mime.map(|m| m.to_string()),
        name_hash: Some(cryptfns::search::tag(&root_key, name).unwrap()),
        size,
        chunks,
        file_id: file_id.map(|f| f.to_string()),
        file_modified_at: None,
        md5: Some("asd".to_string()),
        sha1: Some("asd".to_string()),
        sha256: Some("asd".to_string()),
        blake2b: Some("asd".to_string()),
        digest_tokens_root: None,
        digest_tokens_file: None,
        content_tokens_root: None,
        content_tokens_file: None,
        cipher: None,
        editable: None,
    };

    let (am, _, tags, content, digests, _, _) = file.into_active_model()?;
    repository
        .manage(user.id)
        .create(am, name, tags, content, digests)
        .await
}

/// Index a file under a second account's root key, standing in for a file that
/// account owns. Used by tests that need one user's tags to be invisible to
/// another's query.
pub async fn reindex_for(
    context: &Context,
    user_id: Uuid,
    file_id: Uuid,
    root_key: &[u8],
    name: &str,
) -> AppResult<u64> {
    Repository::new(&context.db)
        .tokens(user_id)
        .reindex(
            file_id,
            SearchTags::new(Some(index_tags(root_key, name)), None),
        )
        .await
}
