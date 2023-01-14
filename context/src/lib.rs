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

    #[cfg(feature = "mock")]
    pub fn mock(db: DatabaseConnection) -> Context {
        let config = Config::mock();

        Context { config, db }
    }

    #[cfg(feature = "mock")]
    pub async fn mock_sqlite() -> Context {
        use migration::MigratorTrait;

        let config = Config::mock();
        std::env::set_var("RUST_LOG", "debug");

        if let Ok(_) = env_logger::try_init() {
            debug!("Log has been initialized");
        }

        let db = Database::connect("sqlite::memory:?mode=rwc").await.unwrap();

        let context = Context { config, db };

        migration::Migrator::up(&context.db, None).await.unwrap();

        context
    }
}
