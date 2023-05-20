//! Repository module for manipulating the tokens for a file in order to index
//! it better and enable full text search.

use std::cmp::Ordering;

use cryptfns::tokenizer::Token;
use entity::{
    file_tokens, files, tokens, user_files, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait,
    Expr, IntoCondition, JoinType, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Uuid,
};
use error::AppResult;

use crate::data::{app_file::AppFile, search::Search};

use super::Repository;

pub(crate) struct Tokens<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    user_id: Uuid,
}

impl<'repository, T> Tokens<'repository, T>
where
    T: ConnectionTrait,
{
    pub(crate) fn new(repository: &'repository Repository<'repository, T>, user_id: Uuid) -> Self {
        Self {
            repository,
            user_id,
        }
    }

    /// Link file with given tokens
    pub(crate) async fn upsert(
        &self,
        file: &files::Model,
        hashed_tokens: Vec<String>,
    ) -> AppResult<u64> {
        let tokens = cryptfns::tokenizer::from_vec(hashed_tokens)?;

        let existing = tokens::Entity::find()
            .filter(
                tokens::Column::Hash.is_in(
                    tokens
                        .iter()
                        .map(|t| t.token.clone())
                        .collect::<Vec<String>>(),
                ),
            )
            .all(self.repository.connection())
            .await?;

        let mut links = vec![];
        let mut new_tokens = vec![];

        for token in tokens {
            if let Some(existing) = existing.iter().find(|t| t.hash == token.token) {
                links.push(file_tokens::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    file_id: ActiveValue::Set(file.id),
                    token_id: ActiveValue::Set(existing.id),
                    weight: ActiveValue::Set(token.weight as i32),
                });
            } else {
                let id = Uuid::new_v4();

                links.push(file_tokens::ActiveModel {
                    id: ActiveValue::Set(Uuid::new_v4()),
                    file_id: ActiveValue::Set(file.id),
                    token_id: ActiveValue::Set(id),
                    weight: ActiveValue::Set(token.weight as i32),
                });

                new_tokens.push(tokens::ActiveModel {
                    id: ActiveValue::Set(id),
                    hash: ActiveValue::Set(token.token),
                });
            }
        }

        if !new_tokens.is_empty() {
            tokens::Entity::insert_many(new_tokens)
                .exec_without_returning(self.repository.connection())
                .await?;
        }

        if !links.is_empty() {
            let result = file_tokens::Entity::insert_many(links)
                .exec_without_returning(self.repository.connection())
                .await?;

            Ok(result)
        } else {
            Ok(0)
        }
    }

    /// Delete all tokens for a file and then recreate them.
    /// This is used when renaming a file or doing file content update. It is not the most
    /// efficient way to get this done, but it is the easiest.
    #[allow(dead_code)]
    pub(crate) async fn rename(
        &self,
        file: &files::Model,
        hashed_tokens: Vec<String>,
    ) -> AppResult<u64> {
        file_tokens::Entity::delete_many()
            .filter(file_tokens::Column::FileId.eq(file.id))
            .exec(self.repository.connection())
            .await?;

        self.upsert(file, hashed_tokens).await
    }

    /// Create a new token
    #[allow(dead_code)]
    pub(crate) async fn create(&self, token: Token) -> AppResult<tokens::Model> {
        let id = Uuid::new_v4();

        let token = tokens::ActiveModel {
            id: ActiveValue::Set(id),
            hash: ActiveValue::Set(token.token),
        };

        tokens::Entity::insert(token)
            .exec_without_returning(self.repository.connection())
            .await?;

        tokens::Entity::find_by_id(id)
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| error::Error::NotFound("token_not_found".to_string()))
    }

    /// Get a token by hash
    #[allow(dead_code)]
    pub(crate) async fn get(&self, hash: &str) -> AppResult<tokens::Model> {
        tokens::Entity::find()
            .filter(tokens::Column::Hash.eq(hash))
            .one(self.repository.connection())
            .await?
            .ok_or_else(|| error::Error::NotFound("token_not_found".to_string()))
    }

    /// Get all tokens for a file
    #[allow(dead_code)]
    pub(crate) async fn get_tokens(&self, file_id: Uuid) -> AppResult<Vec<Token>> {
        let tokens = file_tokens::Entity::find()
            .inner_join(tokens::Entity)
            .filter(file_tokens::Column::FileId.eq(file_id))
            .select_also(tokens::Entity)
            .all(self.repository.connection())
            .await?;

        let mut tokens = tokens
            .into_iter()
            .filter(|(_, token)| token.is_some())
            .map(|(file_token, token)| Token {
                token: token.unwrap().hash,
                weight: file_token.weight as usize,
            })
            .collect::<Vec<_>>();

        tokens.sort_by(|a, b| match a.weight.cmp(&b.weight) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        });

        Ok(tokens)
    }

    /// Search files based on given tokens and sort by the token weight
    pub(crate) async fn search(&self, search: Search) -> AppResult<Vec<AppFile>> {
        let (file_id, hashed_tokens, limit, skip) = search.into_tuple();

        if hashed_tokens.is_empty() {
            return Ok(vec![]);
        }

        let tokens = cryptfns::tokenizer::from_vec(hashed_tokens)?;

        // let user_id = self.user_id;
        let mut query = files::Entity::find();

        if let Some(file_id) = file_id {
            query = query.filter(files::Column::FileId.eq(file_id));
        }

        let user_id = self.user_id;
        let mut query = query
            .inner_join(tokens::Entity)
            .join(
                JoinType::InnerJoin,
                files::Relation::UserFiles
                    .def()
                    .on_condition(move |_left, right| {
                        Expr::col((right, user_files::Column::UserId))
                            .eq(user_id)
                            .into_condition()
                    }),
            )
            .filter(
                tokens::Column::Hash.is_in(
                    tokens
                        .iter()
                        .map(|t| t.token.clone())
                        .collect::<Vec<String>>(),
                ),
            )
            .group_by(files::Column::Id)
            .select_also(user_files::Entity)
            .order_by_desc(file_tokens::Column::Weight.sum());

        if let Some(limit) = limit {
            query = query.limit(limit);
        }

        if let Some(skip) = skip {
            query = query.offset(skip);
        }

        let results = query
            .all(self.repository.connection())
            .await?
            .into_iter()
            .map(|(file, user_file)| AppFile::from((file, user_file.unwrap())))
            .collect::<Vec<_>>();

        Ok(results)
    }
}
