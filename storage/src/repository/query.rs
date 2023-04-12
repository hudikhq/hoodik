//! Repository module for running query operations on files that will automatically filter
//! them for only the files where the user has the file shared with him.

use entity::{
    files, user_files, users, ColumnTrait, ConnectionTrait, EntityTrait, Expr, IntoCondition,
    JoinType, Order, QueryFilter, QueryOrder, QuerySelect, RelationTrait,
};
use error::{AppResult, Error};

use crate::data::{app_file::AppFile, query::Query as RequestQuery, response::Response};

use super::Repository;

pub struct Query<'repository, T: ConnectionTrait> {
    repository: &'repository Repository<'repository, T>,
    user: &'repository users::Model,
}

impl<'repository, T> Query<'repository, T>
where
    T: ConnectionTrait,
{
    pub fn new(
        repository: &'repository Repository<'repository, T>,
        user: &'repository users::Model,
    ) -> Self {
        Self { repository, user }
    }

    /// Find all files that are shared with the user
    pub async fn find(&self, request_query: RequestQuery) -> AppResult<Response> {
        let mut dir = None;

        let mut query = files::Entity::find();

        if let Some(dir_id) = request_query.dir_id {
            dir = self
                .repository
                .manage(self.user)
                .dir(dir_id)
                .await
                .map(Some)?;

            query = query.filter(files::Column::FileId.eq(dir_id));
        } else {
            query = query.filter(files::Column::FileId.is_null());
        }

        let mut order = Order::Asc;
        if let Some(ord) = &request_query.order {
            if ord == "desc" {
                order = Order::Desc;
            }
        }

        match request_query.order_by.as_ref() {
            Some(order_by) => {
                let column = match order_by.as_str() {
                    "created_at" => files::Column::CreatedAt,
                    _ => return Err(Error::BadRequest("invalid_order_by".to_string())),
                };

                query = query.order_by(column, order);
            }
            None => {
                query = query.order_by(files::Column::FileCreatedAt, order);
            }
        }

        let user_id = self.user.id;

        query
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
            .select_also(user_files::Entity)
            .all(self.repository.connection())
            .await
            .map(|files| Response {
                dir,
                children: files
                    .into_iter()
                    .map(|(file, uf)| {
                        // And again, we are good to unwrap here due to the inner_join
                        AppFile::from((file, uf.unwrap()))
                    })
                    .collect::<Vec<AppFile>>(),
            })
            .map_err(Error::from)
    }
}
