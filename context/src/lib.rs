use config::Config;
use error::AppResult;
use log::debug;
use sea_orm::{Database, DatabaseConnection};

/// Holder of the application context
/// all the available database connections
/// and application configurations are in here.
/// Context is passed through the application
pub struct Context {
    pub config: Config,
    pub db: DatabaseConnection,
}

/// We need to implement clone for the context manually because
/// the DatabaseConnection does not implement Clone when feature flag "mock" is enabled
/// We are not too worries about the pitfalls of cloning the mock database connection since
/// we will be using this clone only to spread out the context to the different actix actors
/// and that means we are running in the regular mode and not the testing mode where we would need mock
impl Clone for Context {
    fn clone(&self) -> Context {
        Context {
            config: self.config.clone(),
            db: match &self.db {
                DatabaseConnection::SqlxPostgresPoolConnection(conn) => {
                    DatabaseConnection::SqlxPostgresPoolConnection(conn.clone())
                }
                DatabaseConnection::SqlxSqlitePoolConnection(conn) => {
                    DatabaseConnection::SqlxSqlitePoolConnection(conn.clone())
                }
                #[cfg(feature = "mock")]
                DatabaseConnection::MockDatabaseConnection(conn) => {
                    DatabaseConnection::MockDatabaseConnection(conn.clone())
                }
                DatabaseConnection::Disconnected => DatabaseConnection::Disconnected,
            },
        }
    }
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
    pub fn mock_inject(db: DatabaseConnection) -> Context {
        let config = Config::mock();

        Context { config, db }
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Context {
        let config = Config::mock();

        Context {
            config,
            db: DatabaseConnection::Disconnected,
        }
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

    #[cfg(feature = "mock")]
    pub fn is_disconnected(&self) -> bool {
        matches!(self.db, DatabaseConnection::Disconnected)
    }
}
