use sea_orm_migration::prelude::*;

#[async_std::main]
async fn main() {
    config::Config::env_only();

    cli::run_cli(migration::Migrator).await;
}
