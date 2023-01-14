use config::Config;
use error::AppResult;
use log::debug;
use sea_orm::{Database, DatabaseConnection};

pub struct Context {
    pub config: Config,
    pub db: DatabaseConnection,
}

impl Context {
    pub async fn new(config: Config) -> AppResult<Context> {
        let sqlite_file = format!("sqlite:{}/sqlite.db?mode=rwc", &config.data_dir);

        debug!("{}", &sqlite_file);

        let db = match &config.database_url {
            Some(value) => Database::connect(value).await?,
            None => Database::connect(sqlite_file).await?,
        };

        Ok(Context { config, db })
    }
}
