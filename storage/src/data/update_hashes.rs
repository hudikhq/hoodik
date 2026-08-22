use ::error::AppResult;
use entity::{file_tokens::SearchTags, files::ActiveModel as ActiveModelFile, ActiveValue, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateHashes {
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub sha256: Option<String>,
    pub blake2b: Option<String>,
    /// Digest tags to append to the search index, in the `"{tag}:{weight}"`
    /// form the create route accepts. Digests only exist once the upload has
    /// been read in full, so this is the moment they can be indexed — and
    /// appending rather than replacing is what keeps the name and body tokens
    /// written at create untouched. An uploader who is not the owner cannot
    /// produce root-scope tags and sends only the file scope, the same
    /// asymmetry every other index write follows.
    pub search_tokens_root: Option<Vec<String>>,
    pub search_tokens_file: Option<Vec<String>>,
}

impl Validation for UpdateHashes {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            // SHA-256 is the integrity anchor; optional hashes may be omitted when the client
            // skipped computing them (e.g. performance experiments).
            rule_required!(sha256),
        ]
    }
}

impl UpdateHashes {
    pub fn into_active_model(self, id: Uuid) -> AppResult<(ActiveModelFile, SearchTags)> {
        let data = self.validate()?;

        let tags = SearchTags::new(data.search_tokens_root, data.search_tokens_file);

        Ok((ActiveModelFile {
            id: ActiveValue::Set(id),
            md5: match data.md5 {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            sha1: match data.sha1 {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            sha256: ActiveValue::Set(data.sha256),
            blake2b: match data.blake2b {
                Some(v) => ActiveValue::Set(Some(v)),
                None => ActiveValue::NotSet,
            },
            ..Default::default()
        }, tags))
    }
}
