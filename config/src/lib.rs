use clap::{builder::Str, Arg, ArgMatches, Command};
use dotenv::{from_path, vars};
use std::{
    env::{set_var as set_env_var, var as env_var},
    fs::{self, DirBuilder},
};

const APP_CLIENT_URL: &str = "http://localhost:5173";

/// Config struct that holds all the loaded configuration
/// from the env and arguments.
///
/// Arguments will overwrite any env variables
#[derive(Clone, Debug)]
pub struct Config {
    /// HTTP_PORT where the application will listen at
    ///
    /// *optional*
    ///
    /// default: 4554
    pub port: i32,

    /// HTTP_ADDRESS address where the application will address
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

    /// APP_CLIENT_URL this is the URL of the client application
    /// this is used to redirect all traffic downstream to the client server
    ///
    /// *optional*
    ///
    /// default: const APP_CLIENT_URL (http://localhost:5137)
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

    /// USE_COOKIES This tells us if we should use cookies or not.
    /// Turning this on if you wish to use the API only with your custom
    /// frontend application that might benefit from this way of authentication.
    /// But generally, for most of the modern frontend applications JWT is the way to go.
    ///
    /// Note: Even when using cookies, JWT will still be generated, but it will be ignored  
    /// when authenticating requests.
    ///
    /// *optional*
    ///
    /// default: false
    pub use_cookies: bool,

    /// COOKIE_DOMAIN This should be the URL you are entering to view the application
    /// and it will be used as the cookie domain so its scoped only to this
    ///
    /// *optional*
    pub cookie_domain: Option<String>,

    /// COOKIE_NAME This should be the name of the cookie that will be used to store the session
    /// in your browser it is not that important and you probably don't need to set it
    ///
    /// *optional*
    ///
    /// *default: hoodik_session*
    pub cookie_name: String,

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

    /// LONG_TERM_SESSION_DURATION_DAYS: This tells us how long the long term session should last
    /// in days if the user chooses the option to be remembered by the system.
    ///
    /// *optional*
    ///
    /// default: 30
    pub long_term_session_duration_days: i64,

    /// SHORT_TERM_SESSION_DURATION_MINUTES: This is the period of time that the user will be logged in
    /// if he leaves the application (web client).
    /// While the user is browsing the application the session will keep extending for this period of time.
    ///
    /// *optional*
    ///
    /// default: 5
    pub short_term_session_duration_minutes: i64,
}

impl Config {
    #[cfg(feature = "mock")]
    pub fn mock() -> Config {
        if let Ok(e) = env_var("ENV_FILE") {
            dotenv(Some(e));

            return Config::env_only();
        }

        Config {
            port: 4554,
            address: "localhost".to_string(),
            data_dir: "./data".to_string(),
            database_url: None,
            client_url: None,
            jwt_secret: uuid::Uuid::new_v4().to_string(),
            use_cookies: false,
            cookie_domain: None,
            cookie_name: "hoodik_session".to_string(),
            cookie_http_only: false,
            cookie_secure: false,
            cookie_same_site: "None".to_string(),
            long_term_session_duration_days: 30,
            short_term_session_duration_minutes: 5,
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

        parse_log(matches.as_ref());

        let mut errors = vec![];

        let port = Self::parse_port(matches.as_ref(), &mut errors);
        let address = Self::parse_address(matches.as_ref(), &mut errors);
        let data_dir = Self::parse_data_dir(matches.as_ref(), &mut errors);
        let database_url = Self::parse_database_url(matches.as_ref(), &mut errors);
        let client_url = env_var("APP_CLIENT_URL").ok();
        let jwt_secret = env_var("JWT_SECRET").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
        let use_cookies = env_var("USE_COOKIES")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "false".to_string())
            .as_str()
            == "true";
        let cookie_domain = env_var("COOKIE_DOMAIN").ok();
        let cookie_name = env_var("COOKIE_NAME")
            .ok()
            .unwrap_or_else(|| "hoodik_session".to_string());
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
        let short_term_session_duration_minutes = env_var("SHORT_TERM_SESSION_DURATION_MINUTES")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            client_url,
            jwt_secret,
            use_cookies,
            cookie_domain,
            cookie_name,
            cookie_http_only,
            cookie_secure,
            cookie_same_site,
            long_term_session_duration_days,
            short_term_session_duration_minutes,
        }
        .set_env()
        .ensure_data_dir()
    }

    pub fn env_only() -> Config {
        let matches = None;

        dotenv(None);

        parse_log(matches.as_ref());

        let mut errors = vec![];

        let port = Self::parse_port(matches.as_ref(), &mut errors);
        let address = Self::parse_address(matches.as_ref(), &mut errors);
        let data_dir = Self::parse_data_dir(matches.as_ref(), &mut errors);
        let database_url = Self::parse_database_url(matches.as_ref(), &mut errors);
        let client_url = env_var("APP_CLIENT_URL").ok();
        let jwt_secret = env_var("JWT_SECRET").unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());
        let use_cookies = env_var("USE_COOKIES")
            .ok()
            .map(|c| c.to_lowercase())
            .unwrap_or_else(|| "false".to_string())
            .as_str()
            == "true";
        let cookie_domain = env_var("COOKIE_DOMAIN").ok();
        let cookie_name = env_var("COOKIE_NAME")
            .ok()
            .unwrap_or_else(|| "hoodik_session".to_string());
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
        let short_term_session_duration_minutes = env_var("SHORT_TERM_SESSION_DURATION_MINUTES")
            .unwrap_or_else(|_| "5".to_string())
            .parse()
            .unwrap_or(5);

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            client_url,
            jwt_secret,
            use_cookies,
            cookie_domain,
            cookie_name,
            cookie_http_only,
            cookie_secure,
            cookie_same_site,
            long_term_session_duration_days,
            short_term_session_duration_minutes,
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

        value.unwrap_or(4554)
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
                    Ok(v) => v.cloned(),
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

    /// Get the full bind address
    pub fn get_full_bind_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }

    /// Get the cookie name
    pub fn get_cookie_name(&self) -> String {
        self.cookie_name.clone()
    }

    /// Get URL of the client application
    pub fn get_client_url(&self) -> String {
        parse_path(
            self.client_url
                .clone()
                .unwrap_or_else(|| APP_CLIENT_URL.to_string()),
        )
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
                .short('b')
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
