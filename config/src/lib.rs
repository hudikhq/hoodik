use clap::{builder::Str, Arg, ArgMatches, Command};
use dotenv::{from_path, vars};
use std::env::var as env_var;

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

    /// HTTP_BIND address where the application will bind
    /// *optional*
    /// default: 127.0.0.1
    pub bind: String,

    /// DATA_DIR where all the uploaded data will be stored
    /// *required*
    pub data_dir: String,

    /// URL connection info for Postgres database
    /// *optional*
    /// default: uses sqlite instead of postgres
    pub pg_url: Option<String>,
}

impl Config {
    /// Read the env and arguments and init the config struct
    /// # panics if required attributes are missing or parsing went wrong
    pub fn new(name: &str, version: &str, about: &str) -> Config {
        let matches = arguments(name, version, about);

        dotenv(matches.try_get_one("CONFIG_PATH").unwrap_or(None).cloned());

        parse_log(&matches);

        let mut errors = vec![];

        let port = Self::parse_port(&matches, &mut errors);
        let bind = Self::parse_bind(&matches, &mut errors);
        let data_dir = Self::parse_data_dir(&matches, &mut errors);
        let pg_url = Self::parse_pg_url(&matches, &mut errors);

        if !errors.is_empty() {
            panic!("Failed loading configuration:\n{:#?}", errors);
        }

        let config = Config {
            port,
            bind,
            data_dir: parse_path(data_dir.unwrap()),
            pg_url,
        };

        config
    }

    /// Try loading the port from env or arguments
    fn parse_port(matches: &ArgMatches, errors: &mut Vec<String>) -> i32 {
        let value = match env_var("HTTP_PORT") {
            Ok(v) => match v.parse::<i32>() {
                Ok(v) => Some(v),
                Err(e) => {
                    errors.push(e.to_string());

                    None
                }
            },
            Err(_) => match matches.try_get_one::<i32>("HTTP_PORT") {
                Ok(v) => v.cloned(),
                Err(_) => None,
            },
        };

        value.unwrap_or(4554)
    }

    /// Try loading the bind address from env or arguments
    fn parse_bind(matches: &ArgMatches, _errors: &mut Vec<String>) -> String {
        let value = match env_var("HTTP_BIND") {
            Ok(v) => Some(v),
            Err(_) => match matches.try_get_one::<String>("HTTP_BIND") {
                Ok(v) => v.cloned(),
                Err(_) => None,
            },
        };

        value.unwrap_or("127.0.0.1".to_string())
    }

    /// Try loading the data_dir from env or arguments
    fn parse_data_dir(matches: &ArgMatches, errors: &mut Vec<String>) -> Option<String> {
        match env_var("DATA_DIR") {
            Ok(v) => Some(v),
            Err(_) => match matches.get_one::<String>("DATA_DIR") {
                Some(v) => Some(v.clone()),
                None => {
                    println!("HERE");
                    errors.push("Required attribute 'data_dir' not specified, please provide 'DATA_DIR' environment variable or '--data-dir' cli argument when starting the application".to_string());

                    None
                }
            },
        }
    }

    /// Try loading the pg_url from the arguments or env
    fn parse_pg_url(matches: &ArgMatches, _errors: &mut Vec<String>) -> Option<String> {
        let value = match env_var("PG_URL") {
            Ok(v) => Some(v),
            Err(_) => match matches.try_get_one::<String>("PG_URL") {
                Ok(v) => v.cloned(),
                Err(_) => None,
            },
        };

        value
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
            Arg::new("bind")
                .id("HTTP_BIND")
                .short('b')
                .long("bind")
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
            Arg::new("pg_url")
                .id("PG_URL")
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
        std::env::set_var(&key, &value);
    }
}

/// Set the log level from the cli if possible
fn parse_log(matches: &ArgMatches) {
    if env_var("RUST_LOG").is_err() {
        if let Ok(value) = matches.try_get_one::<String>("RUST_LOG") {
            if let Some(v) = value {
                std::env::set_var("RUST_LOG", v);
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
