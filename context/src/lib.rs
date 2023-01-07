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
        let sqlite_file = format!("sqlite:{}/sqlite.db?mode=rwc", parse_path(&config.data_dir));

        debug!("{}", &sqlite_file);

        let db = match &config.pg_url {
            Some(v) => Database::connect(v).await?,
            None => Database::connect(sqlite_file).await?,
        };

        Ok(Context { config, db })
    }
}

fn parse_path(path: &str) -> String {
    let mut path = path.trim().to_string();

    if path.ends_with('/') {
        let _ = path.pop();
    }

    path
}
