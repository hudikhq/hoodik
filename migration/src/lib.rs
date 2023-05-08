pub use sea_orm_migration::prelude::*;

pub mod m20220101_000001_create_users;
pub mod m20220101_000002_create_user_actions;
pub mod m20230114_091730_create_sessions;
pub mod m20230409_091730_create_files;
pub mod m20230409_101730_create_user_files;
pub mod m20230429_091730_create_tokens;
pub mod m20230429_101730_create_file_tokens;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_users::Migration),
            Box::new(m20220101_000002_create_user_actions::Migration),
            Box::new(m20230114_091730_create_sessions::Migration),
            Box::new(m20230409_091730_create_files::Migration),
            Box::new(m20230409_101730_create_user_files::Migration),
            Box::new(m20230429_091730_create_tokens::Migration),
            Box::new(m20230429_101730_create_file_tokens::Migration),
        ]
    }
}
