use clap::{builder::Str, Arg, ArgMatches, Command};
use dotenv::{from_path, vars};
use email::EmailConfig;
use std::{
    env::{set_var as set_env_var, var as env_var},
    fs::{self, DirBuilder},
};

pub mod email;
pub mod ssl;

/// Config struct that holds all the loaded configuration
/// from the env and arguments.
///
/// Arguments will overwrite any env variables
#[derive(Clone, Debug)]
pub struct Config {
    /// APP_NAME
    /// This is the name of the application, it will be automatically
    /// filled if it hasn't been provided in the env to be something else then Hoodik
    pub app_name: String,

    /// APP_VERSION
    /// if this is left empty it will be automatically filled with the version
    /// from the Cargo.toml file.
    pub app_version: String,

    /// HTTP_PORT where the application will listen at
    ///
    /// *optional*
    ///
    /// default: 5443
    pub port: i32,

    /// HTTP_ADDRESS address where the application will address
    /// this represents the IP address that the application
    /// will listen to when running.
    ///
    /// In Docker image this will be automatically set to 0.0.0.0
    /// and you shouldn't set this unless you are deploying the application
    /// by yourself outside of the docker image.
    ///
    /// *optional*
    ///
    /// default: localhost
    pub address: String,

    /// DATA_DIR where all the uploaded data will be stored
    ///
    /// *required*
    pub data_dir: String,

    /// URL connection info for Postgres database
    ///
    /// *optional*
    ///
    /// default: uses sqlite instead of postgres
    pub database_url: Option<String>,

    /// APP_URL, this is the URL of the application.
    /// When you are running in production this should be the URL
    /// to your application.
    ///
    /// *optional*
    ///
    /// default: https://{HTTP_ADDRESS}:{HTTP_PORT}
    pub app_url: Option<String>,

    /// APP_CLIENT_URL this is the URL of the client application.
    /// This is mostly used while developing and in production this should
    /// ideally be the same as the APP_URL to get the provided
    /// web client interface.
    ///
    /// This will also be used for any kind of calls to actions, like links
    /// from emails will be pointing to this URL with the proper path.
    ///
    /// *optional*
    ///
    /// default: APP_URL
    pub client_url: Option<String>,

    /// JWT_SECRET secret that will be used to sign the JWT tokens
    /// if you don't set this it will generate a random secret every time
    /// the application restarts, that means that all the sessions will be
    /// invalidated every time the application restarts.
    ///
    /// *optional*
    ///
    /// default: generates a random secret
    pub jwt_secret: String,

    /// COOKIE_DOMAIN: If the backend is working by using cookies and not JWT this will be used as the cookie domain.
    /// it automatically defaults to be the same as the APP_URL
    ///
    /// *optional*
    pub cookie_domain: Option<String>,

    /// SESSION_COOKIE This should be the name of the cookie that will be used to store the session
    /// in your browser it is not that important and you probably don't need to set it
    ///
    /// *optional*
    ///
    /// *default: hoodik_session*
    pub session_cookie: String,

    /// REFRESH_COOKIE This is the cookie name of the refresh token that will be used to refresh the session
    /// alongside the session_cookie.
    ///
    /// *optional*
    ///
    /// *default: hoodik_refresh*
    pub refresh_cookie: String,

    /// COOKIE_HTTP_ONLY This tells us if the cookie is supposed to be http only or not. Http only cookie will
    /// only be seen by the browser and not by the javascript frontend. This is okay and its supposed
    /// to work like this.
    ///
    /// *optional*
    ///
    /// *default: true*
    pub cookie_http_only: bool,

    /// COOKIE_SECURE This tells us if the cookie is supposed to be secure or not. Secure cookie will
    /// only be sent over https.
    ///
    /// *optional*
    ///
    /// *default: true*
    pub cookie_secure: bool,

    /// COOKIE_SAME_SITE: This tells us if the cookie is supposed to be same site or not. Same site cookie will
    /// only be sent over same site.
    ///
    /// *optional*
    ///
    /// *default: Lax*
    ///
    /// *possible values: Lax, Strict, None*
    pub cookie_same_site: String,

    /// LONG_TERM_SESSION_DURATION_DAYS: This tells us for how long
    /// will the session be refreshed if the user is not using the application.
    ///
    /// *optional*
    ///
    /// default: 30
    pub long_term_session_duration_days: i64,

    /// SHORT_TERM_SESSION_DURATION_SECONDS: This is the period of time that the user will be logged in
    /// if he leaves the application (web client).
    /// While the user is browsing the application the session will keep extending for this period of time.
    ///
    /// *optional*
    ///
    /// default: 300
    pub short_term_session_duration_seconds: i64,

    /// Location of the ssl cert file, this will be loaded and setup on to the server
    /// if you don't provide this, the server will generate a self signed certificate
    /// and place them in the /tmp directory. This is not recommended for production.
    ///
    /// *optional*
    ///
    /// default: DATA_DIR/hoodik.crt.pem
    pub ssl_cert_file: String,

    /// Location of the ssl key file, this will be loaded and setup on to the server
    /// if you don't provide this, the server will generate a self signed certificate
    /// and place them in the /tmp directory. This is not recommended for production.
    ///
    /// *optional*
    ///
    /// default: DATA_DIR/hoodik.key.pem
    pub ssl_key_file: String,

    /// Email configuration holder, there are couple of options for this configuration,
    /// see more details in the [crate::email::EmailConfig] struct.
    pub mailer: EmailConfig,
}

impl Config {
    #[cfg(feature = "mock")]
    pub fn mock() -> Config {
        if let Ok(e) = env_var("ENV_FILE") {
            dotenv(Some(e));

            return Config::env_only();
        }

        let app_name = "Hoodik".to_string();
        let app_version = env_var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());

        let data_dir = "./data".to_string();

        let (ssl_cert_file, ssl_key_file) = Self::parse_ssl_files(&Some(data_dir.clone()));

        let mailer = EmailConfig::None;

        Config {
            app_name,
            app_version,
            port: 5443,
            address: "localhost".to_string(),
            data_dir,
            database_url: None,
            app_url: Some("http://localhost:5443".to_string()),
            client_url: Some("http://localhost:5443".to_string()),
            jwt_secret: uuid::Uuid::new_v4().to_string(),
            cookie_domain: None,
            session_cookie: "hoodik_session".to_string(),
            refresh_cookie: "hoodik_refresh".to_string(),
            cookie_http_only: false,
            cookie_secure: false,
            cookie_same_site: "None".to_string(),
            long_term_session_duration_days: 30,
            short_term_session_duration_seconds: 300,
            ssl_cert_file,
            ssl_key_file,
            mailer,
        }
        .ensure_data_dir()
    }

    /// Read the env and arguments and init the config struct
    /// **panics** if required attributes are missing or parsing went wrong
    pub fn new(name: &str, version: &str, about: &str) -> Config {
        let matches = Some(arguments(name, version, about));

        if let Some(m) = matches.as_ref() {
            dotenv(m.try_get_one("CONFIG_PATH").unwrap_or(None).cloned());
        }

        let app_name = env_var("APP_NAME").unwrap_or_else(|_| "Hoodik".to_string());
        let app_version = env_var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());

        parse_log(matches.as_ref());

        let mut errors = vec![];

        let port = Self::parse_port(matches.as_ref(), &mut errors);
        let address = Self::parse_address(matches.as_ref(), &mut errors);
        let data_dir = Self::parse_data_dir(matches.as_ref(), &mut errors);
        let database_url = Self::parse_database_url(matches.as_ref(), &mut errors);
        let app_url = Self::parse_app_url(&address, port);
        let client_url = Self::parse_client_url(&app_url);
        let jwt_secret = env_var("JWT_SECRET").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
        let use_cookies = env_var("USE_COOKIES")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "false".to_string())
            .as_str()
            == "true";
        let cookie_domain = match env_var("COOKIE_DOMAIN") {
            Ok(v) => Some(v),
            Err(_) => app_url.clone(),
        };
        let session_cookie = env_var("SESSION_COOKIE")
            .ok()
            .unwrap_or_else(|| "hoodik_session".to_string());
        let refresh_cookie = env_var("REFRESH_COOKIE")
            .ok()
            .unwrap_or_else(|| "hoodik_refresh".to_string());
        let cookie_http_only = env_var("COOKIE_HTTP_ONLY")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str()
            == "true";
        let cookie_secure = env_var("COOKIE_SECURE")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str()
            == "true";
        let cookie_same_site = Self::parse_cookie_same_site();
        let long_term_session_duration_days = env_var("LONG_TERM_SESSION_DURATION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);
        let short_term_session_duration_seconds = env_var("SHORT_TERM_SESSION_DURATION_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300);

        let (ssl_cert_file, ssl_key_file) = Self::parse_ssl_files(&data_dir);
        let mailer = EmailConfig::new(&mut errors);

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            app_name,
            app_version,
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            client_url,
            app_url,
            jwt_secret,
            cookie_domain,
            session_cookie,
            refresh_cookie,
            cookie_http_only,
            cookie_secure,
            cookie_same_site,
            long_term_session_duration_days,
            short_term_session_duration_seconds,
            ssl_cert_file,
            ssl_key_file,
            mailer,
        }
        .set_env()
        .ensure_data_dir()
    }

    pub fn env_only() -> Config {
        let matches = None;

        dotenv(None);

        parse_log(matches.as_ref());

        let app_name = env_var("APP_NAME").unwrap_or_else(|_| "Hoodik".to_string());
        let app_version = env_var("CARGO_PKG_VERSION").unwrap_or_else(|_| "unknown".to_string());

        let mut errors = vec![];

        let port = Self::parse_port(matches.as_ref(), &mut errors);
        let address = Self::parse_address(matches.as_ref(), &mut errors);
        let data_dir = Self::parse_data_dir(matches.as_ref(), &mut errors);
        let database_url = Self::parse_database_url(matches.as_ref(), &mut errors);
        let app_url = Self::parse_app_url(&address, port);
        let client_url = Self::parse_client_url(&app_url);
        let jwt_secret = env_var("JWT_SECRET").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
        let use_cookies = env_var("USE_COOKIES")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "false".to_string())
            .as_str()
            == "true";
        let cookie_domain = match env_var("COOKIE_DOMAIN") {
            Ok(v) => Some(v),
            Err(_) => app_url.clone(),
        };
        let session_cookie = env_var("SESSION_COOKIE")
            .ok()
            .unwrap_or_else(|| "hoodik_session".to_string());
        let refresh_cookie = env_var("REFRESH_COOKIE")
            .ok()
            .unwrap_or_else(|| "hoodik_refresh".to_string());
        let cookie_http_only = env_var("COOKIE_HTTP_ONLY")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str()
            == "true";
        let cookie_secure = env_var("COOKIE_SECURE")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "true".to_string())
            .as_str()
            == "true";
        let cookie_same_site = Self::parse_cookie_same_site();
        let long_term_session_duration_days = env_var("LONG_TERM_SESSION_DURATION_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse()
            .unwrap_or(30);
        let short_term_session_duration_seconds = env_var("SHORT_TERM_SESSION_DURATION_SECONDS")
            .unwrap_or_else(|_| "300".to_string())
            .parse()
            .unwrap_or(300);

        let (ssl_cert_file, ssl_key_file) = Self::parse_ssl_files(&data_dir);
        let mailer = EmailConfig::new(&mut errors);

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            app_name,
            app_version,
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            app_url,
            client_url,
            jwt_secret,
            cookie_domain,
            session_cookie,
            refresh_cookie,
            cookie_http_only,
            cookie_secure,
            cookie_same_site,
            long_term_session_duration_days,
            short_term_session_duration_seconds,
            ssl_cert_file,
            ssl_key_file,
            mailer,
        }
        .set_env()
        .ensure_data_dir()
    }

    /// Set everything back into the env variables so it can be used also for the migration
    fn set_env(self) -> Self {
        set_env_var("HTTP_PORT", format!("{}", self.port));
        set_env_var("HTTP_ADDRESS", self.address.clone());

        if let Some(db) = &self.database_url {
            set_env_var("DATABASE_URL", db);
        } else {
            set_env_var(
                "DATABASE_URL",
                format!("sqlite:{}/sqlite.db?mode=rwc", &self.data_dir),
            );
        }

        self
    }

    /// Make sure the data directory exists and create it if not
    fn ensure_data_dir(self) -> Self {
        let mut dir_builder = DirBuilder::new();

        match dir_builder.recursive(true).create(&self.data_dir) {
            Ok(_) => (),
            Err(e) => println!("Error creating directory: {:?}", e),
        };

        let metadata = fs::metadata(&self.data_dir).unwrap();
        let permissions = metadata.permissions();

        if permissions.readonly() {
            panic!("DATA_DIR is not writeable to the application, aborting...")
        }

        self
    }

    /// Try loading the port from env or arguments
    fn parse_port(matches: Option<&ArgMatches>, errors: &mut Vec<String>) -> i32 {
        let value = match env_var("HTTP_PORT") {
            Ok(v) => match v.parse::<i32>() {
                Ok(v) => Some(v),
                Err(e) => {
                    errors.push(e.to_string());

                    None
                }
            },
            Err(_) => match matches {
                Some(m) => match m.try_get_one::<i32>("HTTP_PORT") {
                    Ok(v) => v.cloned(),
                    Err(_) => None,
                },
                None => None,
            },
        };

        value.unwrap_or(5443)
    }

    /// Try loading the address address from env or arguments
    fn parse_address(matches: Option<&ArgMatches>, _errors: &mut [String]) -> String {
        let value = match env_var("HTTP_ADDRESS") {
            Ok(v) => Some(v),
            Err(_) => match matches {
                Some(m) => match m.try_get_one::<String>("HTTP_ADDRESS") {
                    Ok(v) => v.cloned(),
                    Err(_) => None,
                },
                None => None,
            },
        };

        value.unwrap_or_else(|| "localhost".to_string())
    }

    /// Try loading the app url from env
    fn parse_app_url(address: &str, port: i32) -> Option<String> {
        if let Ok(app_url) = env_var("APP_URL") {
            Some(app_url)
        } else {
            Some(format!("https://{}:{}", address, port))
        }
    }

    /// Try loading the app url from env
    fn parse_client_url(app_url: &Option<String>) -> Option<String> {
        if let Ok(client_app_url) = env_var("APP_CLIENT_URL") {
            Some(client_app_url)
        } else {
            app_url.clone()
        }
    }

    /// Try loading the data_dir from env or arguments
    fn parse_data_dir(matches: Option<&ArgMatches>, errors: &mut Vec<String>) -> Option<String> {
        let data_dir = match env_var("DATA_DIR") {
            Ok(v) => Some(v),
            Err(_) => match matches {
                Some(m) => m.get_one::<String>("DATA_DIR").cloned(),
                None => None,
            },
        };

        if data_dir.is_none() {
            errors.push("Required attribute 'data_dir' not specified, please provide 'DATA_DIR' environment variable or '--data-dir' cli argument when starting the application".to_string());
        }

        data_dir
    }

    /// Try loading the database_url from the arguments or env
    fn parse_database_url(matches: Option<&ArgMatches>, _errors: &mut [String]) -> Option<String> {
        let value = match env_var("DATABASE_URL") {
            Ok(v) => Some(v),
            Err(_) => match matches {
                Some(m) => match m.try_get_one::<String>("DATABASE_URL") {
                    Ok(v) => match v {
                        Some(v) => {
                            if !v.is_empty() {
                                None
                            } else {
                                Some(v.clone())
                            }
                        }
                        None => None,
                    },
                    Err(_) => None,
                },
                None => None,
            },
        };

        value
    }

    pub fn parse_cookie_same_site() -> String {
        let value = match env_var("COOKIE_SAME_SITE") {
            Ok(v) => Some(v),
            Err(_) => None,
        };

        match value {
            Some(x) => {
                if matches!(x.as_str(), "Strict" | "Lax" | "None") {
                    x
                } else {
                    "Lax".to_string()
                }
            }
            _ => "Lax".to_string(),
        }
    }

    /// Try to make sense of the SSL_CERT_FILE and SSL_KEY_FILE env variables
    pub fn parse_ssl_files(data_dir: &Option<String>) -> (String, String) {
        let data_dir = data_dir.clone().unwrap_or_else(|| "/tmp".to_string());

        let ssl_cert_file =
            env_var("SSL_CERT_FILE").unwrap_or_else(|_| format!("{}/hoodik.crt.pem", &data_dir));
        let ssl_key_file =
            env_var("SSL_KEY_FILE").unwrap_or_else(|_| format!("{}/hoodik.key.pem", &data_dir));

        (ssl_cert_file, ssl_key_file)
    }

    /// Get the full bind address
    pub fn get_full_bind_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Get the session cookie name
    pub fn get_session_cookie(&self) -> String {
        self.session_cookie.clone()
    }

    /// Get the refresh token cookie name
    pub fn get_refresh_cookie(&self) -> String {
        self.refresh_cookie.clone()
    }

    /// Get URL of the client application
    pub fn get_client_url(&self) -> String {
        parse_path(
            self.client_url
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or_else(|| self.get_app_url()),
        )
    }

    /// Get URL of the client application
    pub fn get_app_url(&self) -> String {
        parse_path(
            self.app_url
                .as_ref()
                .map(|u| u.to_string())
                .unwrap_or_else(|| format!("https://{}:{}", &self.address, self.port)),
        )
    }

    pub fn get_app_name(&self) -> String {
        self.app_name.clone()
    }

    pub fn get_app_version(&self) -> String {
        self.app_version.clone()
    }
}

/// Define matches from the command line arguments
pub fn arguments<'a>(name: &'a str, version: &'a str, about: &'a str) -> ArgMatches {
    Command::new(name.to_string())
        .version(Str::from(version.to_string()))
        .about(about.to_string())
        .arg(
            Arg::new("port")
                .id("HTTP_PORT")
                .short('p')
                .long("port")
                .help("HTTP port where this application will listen")
                .required(false),
        )
        .arg(
            Arg::new("address")
                .id("HTTP_ADDRESS")
                .short('a')
                .long("address")
                .help("HTTP address where the application will attach itself")
                .required(false),
        )
        .arg(
            Arg::new("data_dir")
                .id("DATA_DIR")
                .short('d')
                .long("data-dir")
                .help("Location where the application will store the data")
                .required(false),
        )
        .arg(
            Arg::new("database_url")
                .id("DATABASE_URL")
                .long("pg-url")
                .help("Connection string for the postgres database, by default we will fallback to sqlite database stored in your data-dir")
                .required(false),
        )
        .arg(
            Arg::new("config_path")
                .id("CONFIG_PATH")
                .short('c')
                .long("config")
                .help("Specify the custom config location where the env variables will be loaded from")
                .required(false),
        )
        .arg(
            Arg::new("log")
                .id("RUST_LOG")
                .short('l')
                .long("log")
                .help("Set the RUST_LOG variable")
                .required(false),
        )
        .get_matches()
}

/// Load the env variables
fn dotenv(path: Option<String>) {
    let vars: Vec<(String, String)> = match path {
        Some(p) => {
            match from_path(&p) {
                Ok(_) => (),
                Err(e) => panic!("Couldn't load the dotenv config at '{}', error: {}", p, e),
            }

            vars().collect()
        }
        None => vars().collect(),
    };

    for (key, value) in vars.iter() {
        std::env::set_var(key, value);
    }
}

/// Set the log level from the cli if possible
fn parse_log(matches: Option<&ArgMatches>) {
    if env_var("RUST_LOG").is_err() {
        if let Some(m) = matches {
            if let Ok(Some(value)) = m.try_get_one::<String>("RUST_LOG") {
                std::env::set_var("RUST_LOG", value);
            }
        }
    }
}

/// Remove the leading slash from the path
fn parse_path(path: String) -> String {
    let mut path = path.trim().to_string();

    if path.ends_with('/') {
        let _ = path.pop();
    }

    path
}
