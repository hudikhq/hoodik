use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One tagged token of a file's searchable text.
///
/// The tag is an HMAC of the token under a key the server never sees, so this
/// table cannot be read back into words. Each file carries the same tokens
/// twice, under the two keys described on [`Scope`]. [`Source`] is a separate
/// axis: which client-side text produced the token, so a rename cannot wipe
/// a note body or extra context tags.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "file_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub file_id: Uuid,
    pub scope: i32,
    pub source: i32,
    pub tag: String,
    pub weight: i32,
}

/// Which key a row's tag was produced under, and what kind of token it is.
///
/// For the word scopes, a client sends root tags for what it owns and file
/// tags for what is shared with it, never both for the same file, so the two
/// cannot double-count a file in the weight ranking. Content digests get
/// scopes of their own because their lifecycle differs: a rename replaces the
/// name source, and digest tags — derived from the bytes, not the name —
/// must survive that replacement. Searching treats each digest scope
/// as part of its word counterpart, so an exact digest query matches through
/// the ordinary tag equality.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Keyed on the owner's account-wide search key. Answers the owner's own
    /// search in a single tag per query word.
    Root = 0,
    /// Keyed on the file's own encryption key, which reaches every share
    /// recipient inside `user_files.encrypted_key`. This is the scope that
    /// makes a share searchable without any index work at share time.
    File = 1,
    /// A content digest tagged under the owner's root key. Written when the
    /// digest lands, replaced only by another digest write.
    DigestRoot = 2,
    /// A content digest tagged under the file's own key, reaching every
    /// holder of the file key the same way [`Scope::File`] does.
    DigestFile = 3,
}

impl From<Scope> for i32 {
    fn from(scope: Scope) -> Self {
        scope as i32
    }
}

/// Which client-side text a word-scope row was produced from.
///
/// Orthogonal to [`Scope`]: that column is which HMAC key tagged the token,
/// this one is what the client tokenized. Write paths replace only their own
/// source, so extra tags survive rename and note-save. Digest-scope rows also
/// carry a source because the column is not null; they are replaced by scope,
/// not source.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Source {
    /// Create, rename, and the re-index sweep write here.
    Name = 0,
    /// Note-body tokens. `replace_content` always writes here; create,
    /// reindex, and fork do when the client sends `content_tokens_*`.
    Content = 1,
    /// `PUT .../extra-tokens` writes here.
    Extra = 2,
}

impl From<Source> for i32 {
    fn from(source: Source) -> Self {
        source as i32
    }
}

/// The tag sets a client sends when indexing a file, each in the
/// `"{tag}:{weight}"` wire form.
///
/// Either may be absent. An editor who is not the owner holds the file key but
/// not the owner's root key, so their save carries only [`Scope::File`], and
/// the writer replaces just the scopes it was given.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchTags {
    pub root: Option<Vec<String>>,
    pub file: Option<Vec<String>>,
}

impl SearchTags {
    pub fn new(root: Option<Vec<String>>, file: Option<Vec<String>>) -> Self {
        Self { root, file }
    }
}

/// Content-digest tags, in the same wire form as [`SearchTags`] but destined
/// for the digest scopes. Kept as a separate type because the two sets have
/// different lifecycles: word tags are replaced by renames and content saves,
/// digest tags only by another digest write.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DigestTags {
    pub root: Option<Vec<String>>,
    pub file: Option<Vec<String>>,
}

impl DigestTags {
    pub fn new(root: Option<Vec<String>>, file: Option<Vec<String>>) -> Self {
        Self { root, file }
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::files::Entity",
        from = "Column::FileId",
        to = "super::files::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Files,
}

impl Related<super::files::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Files.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
