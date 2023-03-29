use clap::{builder::Str, Arg, ArgMatches, Command};
use dotenv::{from_path, vars};
use std::{
    env::{set_var as set_env_var, var as env_var},
    fs::{self, DirBuilder},
};

/// Config struct that holds all the loaded configuration
/// from the env and arguments.
///
/// Arguments will overwrite any env variables
#[derive(Clone, Debug)]
pub struct Config {
    /// HTTP_PORT where the application will listen at
    /// *optional*
    /// default: 4554
    pub port: i32,

    /// HTTP_ADDRESS address where the application will address
    /// *optional*
    /// default: 127.0.0.1
    pub address: String,

    /// DATA_DIR where all the uploaded data will be stored
    /// *required*
    pub data_dir: String,

    /// URL connection info for Postgres database
    /// *optional*
    /// default: uses sqlite instead of postgres
    pub database_url: Option<String>,

    /// COOKIE_DOMAIN This should be the URL you are entering to view the application
    /// and it will be used as the cookie domain so its scoped only to this
    /// *optional*
    pub cookie_domain: Option<String>,

    /// COOKIE_NAME This should be the name of the cookie that will be used to store the session
    /// in your browser it is not that important and you probably don't need to set it
    /// *optional*
    /// *default: hoodik_session*
    pub cookie_name: String,

    /// COOKIE_HTTP_ONLY This tells us if the cookie is supposed to be http only or not. Http only cookie will
    /// only be seen by the browser and not by the javascript frontend. This is okay and its supposed
    /// to work like this.
    /// *optional*
    /// *default: true*
    pub cookie_http_only: bool,

    /// COOKIE_SECURE This tells us if the cookie is supposed to be secure or not. Secure cookie will
    /// only be sent over https.
    /// *optional*
    /// *default: true*
    pub cookie_secure: bool,

    /// COOKIE_SAME_SITE: This tells us if the cookie is supposed to be same site or not. Same site cookie will
    /// only be sent over same site.
    /// *optional*
    /// *default: Lax*
    /// *possible values: Lax, Strict, None*
    pub cookie_same_site: String,
}

impl Config {
    #[cfg(feature = "mock")]
    pub fn mock() -> Config {
        Config {
            port: 4554,
            address: "127.0.0.1".to_string(),
            data_dir: "./data".to_string(),
            database_url: None,
            cookie_domain: None,
            cookie_name: "hoodik_session".to_string(),
            cookie_http_only: false,
            cookie_secure: false,
            cookie_same_site: "None".to_string(),
        }
    }
    /// Read the env and arguments and init the config struct
    /// # panics if required attributes are missing or parsing went wrong
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

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            cookie_domain: None,
            cookie_name: "hoodik_session".to_string(),
            cookie_http_only: false,
            cookie_secure: false,
            cookie_same_site: "None".to_string(),
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

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        Config {
            port,
            address,
            data_dir: parse_path(data_dir.unwrap()),
            database_url,
            cookie_domain,
            cookie_name,
            cookie_http_only,
            cookie_secure,
            cookie_same_site,
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

        value.unwrap_or_else(|| "127.0.0.1".to_string())
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

    pub fn get_cookie_name(&self) -> String {
        self.cookie_name.clone()
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
