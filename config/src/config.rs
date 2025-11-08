use crate::{app::AppConfig, email::EmailConfig, ssl::SslConfig, vars::Vars};

/// Config struct that holds all the loaded configuration
/// from the env and arguments.
///
/// Arguments will overwrite any env variables
///
/// Point of this configuration struct is to provide us with structured
/// way to hold all of the needed data, and it will also provide us with
/// safety that we have all the needed data to run the application.
///
/// Initialization of the Config will fail if any of the required
/// configuration is missing. This will prevent the app from startup.
#[derive(Clone, Debug)]
pub struct Config {
    /// Basic application configuration
    /// see more details in the [crate::app::AppConfig] struct.
    pub app: crate::app::AppConfig,

    /// Application authentication configuration
    /// see more details in the [crate::auth::AuthConfig] struct.
    pub auth: crate::auth::AuthConfig,

    /// SSL configuration for the application
    /// see more details in the [crate::ssl::SslConfig] struct.
    pub ssl: crate::ssl::SslConfig,

    /// Email configuration holder, there are couple of options for this configuration,
    /// see more details in the [crate::email::EmailConfig] struct.
    pub mailer: crate::email::EmailConfig,

    /// Warnings collected during configuration initialization
    pub(crate) warnings: Vec<String>,
}

impl From<Vars> for Config {
    fn from(mut vars: Vars) -> Config {
        if let Some(v) = vars.maybe_var::<String>("RUST_LOG").maybe_get() {
            std::env::set_var("RUST_LOG", v);
        }

        let app = AppConfig::new(&mut vars);
        let ssl = SslConfig::new(&app, &mut vars);

        let mailer = EmailConfig::new(&mut vars);
        let auth = crate::auth::AuthConfig::new(&app, &mut vars);

        vars.panic_if_errors("Config");

        // Collect warnings before consuming vars
        let warnings = vars.get_warnings().to_vec();

        Self {
            ssl,
            app,
            auth,
            mailer,
            warnings,
        }
    }
}

impl Config {
    /// Don't load any environment variables, just use the defaults
    pub fn empty() -> Self {
        Self::from(Vars::create("Hoodik", "v0.1.0", "Hoodik"))
    }

    /// Create a new config with the given name, version and about
    pub fn new(name: &str, version: &str, about: &str) -> Self {
        Self::from(Vars::new(name, version, about))
    }

    /// Create a new config with the default name, version and about
    pub fn env_only(name: &str, version: &str, about: &str) -> Self {
        Self::from(Vars::env_only(name, version, about))
    }

    #[cfg(feature = "mock")]
    /// Create a new config with the given name, version and about
    pub fn mock_with_env() -> Self {
        Self::from(Vars::env_only("Hoodik", "v0.1.0", "Hoodik"))
    }
}
