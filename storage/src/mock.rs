use context::Context;
use entity::{users, Uuid};
use error::AppResult;

use crate::{
    data::{app_file::AppFile, create_file::CreateFile},
    repository::Repository,
};

pub async fn create_file<'ctx>(
    context: &'ctx Context,
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

    let search_tokens_hashed =
        cryptfns::tokenizer::into_string(cryptfns::tokenizer::into_tokens(name).unwrap())
            .split(';')
            .map(|i| i.to_string())
            .collect::<Vec<_>>();

    let file = CreateFile {
        encrypted_key: Some(name.to_string()),
        encrypted_name: Some(name.to_string()),
        encrypted_thumbnail: None,
        search_tokens_hashed: Some(search_tokens_hashed),
        mime: mime.map(|m| m.to_string()),
        name_hash: Some(cryptfns::sha256::digest(name.as_bytes())),
        size,
        chunks,
        file_id: file_id.map(|f| f.to_string()),
        file_created_at: None,
    };

    let (am, _, tokens) = file.into_active_model()?;
    repository.manage(user.id).create(am, name, tokens).await
}
