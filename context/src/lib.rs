use config::Config;
use email::Sender;
use error::AppResult;
use sea_orm::Database;

/// Re-export the database connection type
pub use sea_orm::DatabaseConnection;

pub use email::contract::SenderContract;

/// Holder of the application context
/// all the available database connections
/// and application configurations are in here.
/// Context is passed through the application
pub struct Context {
    pub config: Config,
    pub db: DatabaseConnection,
    pub sender: Option<Sender>,
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
                DatabaseConnection::Disconnected => DatabaseConnection::Disconnected,

                #[cfg(feature = "mock")]
                DatabaseConnection::MockDatabaseConnection(conn) => {
                    DatabaseConnection::MockDatabaseConnection(conn.clone())
                }
            },
            sender: self.sender.clone(),
        }
    }
}

impl Context {
    pub async fn new(config: Config) -> AppResult<Context> {
        let sqlite_file = format!("sqlite:{}/sqlite.db?mode=rwc", &config.app.data_dir);

        let db = match &config.app.database_url {
            Some(value) => Database::connect(value).await?,
            None => Database::connect(sqlite_file).await?,
        };

        let sender = email::Sender::new(&config)?;

        Ok(Context { config, db, sender })
    }

    #[cfg(feature = "mock")]
    pub fn mock_inject(db: DatabaseConnection) -> Context {
        let config = Config::mock_with_env();

        Context {
            config,
            db,
            sender: None,
        }
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Context {
        let config = Config::mock_with_env();

        Context {
            config,
            db: DatabaseConnection::Disconnected,
            sender: None,
        }
    }

    #[cfg(feature = "mock")]
    pub async fn mock_with_data_dir(data_dir: Option<String>) -> Context {
        use migration::MigratorTrait;

        let mut config = Config::empty();
        config.app.ensure_data_dir(data_dir);

        if env_logger::try_init().is_ok() {
            log::debug!("Log has been initialized");
        }

        let db = Database::connect("sqlite::memory:?mode=rwc").await.unwrap();

        let context = Context {
            config,
            db,
            sender: None,
        };

        migration::Migrator::up(&context.db, None).await.unwrap();

        context
    }

    #[cfg(feature = "mock")]
    pub async fn mock_sqlite() -> Context {
        use migration::MigratorTrait;

        let config = Config::mock_with_env();

        if env_logger::try_init().is_ok() {
            log::debug!("Log has been initialized");
        }

        let db = Database::connect("sqlite::memory:?mode=rwc").await.unwrap();

        let context = Context {
            config,
            db,
            sender: None,
        };

        migration::Migrator::up(&context.db, None).await.unwrap();

        context
    }

    #[cfg(feature = "mock")]
    pub fn add_mock_sender(mut context: Context) -> Context {
        let sender = Sender::mock();

        context.sender = Some(sender);

        context
    }

    #[cfg(feature = "mock")]
    pub fn is_disconnected(&self) -> bool {
        matches!(self.db, DatabaseConnection::Disconnected)
    }
}
