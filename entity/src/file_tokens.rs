use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// One tagged token of a file's searchable text.
///
/// The tag is an HMAC of the token under a key the server never sees, so this
/// table cannot be read back into words. Each file carries the same tokens
/// twice, under the two keys described on [`Scope`].
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "file_tokens")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub file_id: Uuid,
    pub scope: i32,
    pub tag: String,
    pub weight: i32,
}

/// Which key a row's tag was produced under.
///
/// A client sends root tags for what it owns and file tags for what is shared
/// with it, never both for the same file, so the two scopes cannot double-count
/// a file in the weight ranking.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Keyed on the owner's account-wide search key. Answers the owner's own
    /// search in a single tag per query word.
    Root = 0,
    /// Keyed on the file's own encryption key, which reaches every share
    /// recipient inside `user_files.encrypted_key`. This is the scope that
    /// makes a share searchable without any index work at share time.
    File = 1,
}

impl From<Scope> for i32 {
    fn from(scope: Scope) -> Self {
        scope as i32
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
