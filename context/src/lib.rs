use config::Config;
use email::Sender;
use error::AppResult;
use sea_orm::Database;

/// Re-export the database connection type
pub use sea_orm::DatabaseConnection;

pub use email::contract::SenderContract;
use settings::{factory::Factory, Settings};

/// Holder of the application context
/// all the available database connections
/// and application configurations are in here.
/// Context is passed through the application
pub struct Context {
    pub config: Config,
    pub db: DatabaseConnection,
    pub sender: Option<Sender>,
    pub settings: Settings,
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
            settings: self.settings.clone(),
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

        let settings = Settings::default().create(&config).await?;

        Ok(Context {
            config,
            db,
            sender,
            settings,
        })
    }

    #[cfg(feature = "mock")]
    pub fn mock_inject(db: DatabaseConnection) -> Context {
        let config = Config::mock_with_env();
        let settings = Settings::mock();

        Context {
            config,
            db,
            sender: None,
            settings,
        }
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Context {
        let config = Config::mock_with_env();
        let settings = Settings::mock();

        Context {
            config,
            db: DatabaseConnection::Disconnected,
            sender: None,
            settings,
        }
    }

    #[cfg(feature = "mock")]
    pub async fn mock_with_data_dir(data_dir: Option<String>) -> Context {
        use migration::MigratorTrait;

        let mut config = Config::empty();
        config.app.ensure_data_dir(data_dir.clone());

        if env_logger::try_init().is_ok() {
            log::debug!("Log has been initialized");
        }

        let db = Self::mock_db_connection(data_dir.as_deref()).await;
        let settings = Settings::mock();

        let context = Context {
            config,
            db,
            sender: None,
            settings,
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

        let db = Self::mock_db_connection(None).await;
        let settings = Settings::mock();

        let context = Context {
            config,
            db,
            sender: None,
            settings,
        };

        migration::Migrator::up(&context.db, None).await.unwrap();

        context
    }

    /// When `TEST_DATABASE_URL` is set (e.g. a Postgres URL to a superuser-
    /// accessible admin DB), create a throwaway database with a unique name
    /// and connect to it. Falls back to `sqlite::memory:` otherwise. Used
    /// only by the `mock_*` helpers — lets the integration suite run
    /// against Postgres to verify SQL parity with the SQLite default.
    #[cfg(feature = "mock")]
    async fn mock_db_connection(slug_hint: Option<&str>) -> DatabaseConnection {
        match std::env::var("TEST_DATABASE_URL").ok() {
            Some(admin_url) => Self::bootstrap_fresh_pg(&admin_url, slug_hint).await,
            None => Database::connect("sqlite::memory:?mode=rwc").await.unwrap(),
        }
    }

    #[cfg(feature = "mock")]
    async fn bootstrap_fresh_pg(
        admin_url: &str,
        slug_hint: Option<&str>,
    ) -> DatabaseConnection {
        use sea_orm::{ConnectionTrait, Statement};

        let slug_source = slug_hint
            .map(|s| s.trim_start_matches("../").to_string())
            .unwrap_or_else(|| format!("mock_{}", entity::Uuid::new_v4().simple()));
        let sanitized: String = slug_source
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
            .collect();
        let unique = entity::Uuid::new_v4().simple().to_string();
        let db_name = format!("hoodik_test_{}_{}", &sanitized[..sanitized.len().min(32)], &unique[..8]);

        let admin = Database::connect(admin_url).await.unwrap();
        let backend = admin.get_database_backend();
        admin
            .execute(Statement::from_string(
                backend,
                format!(r#"CREATE DATABASE "{db_name}""#),
            ))
            .await
            .unwrap();

        let parsed: url::Url = admin_url.parse().unwrap();
        let mut target = parsed.clone();
        target.set_path(&format!("/{db_name}"));
        Database::connect(target.as_str()).await.unwrap()
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
