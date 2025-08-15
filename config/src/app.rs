use std::fs::{self, DirBuilder};

use url::Url;

use crate::{
    helpers::{absolute_path, remove_trailing_slash},
    vars::Vars,
};

#[derive(Debug, Clone)]
pub struct AppConfig {
    /// DATA_DIR where all the uploaded data will be stored
    ///
    /// *required*
    pub data_dir: String,

    /// DATABASE_URL connection info for Postgres database
    ///
    /// *optional*
    ///
    /// default: uses sqlite instead of postgres
    pub database_url: Option<String>,

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

    /// APP_URL, this is the URL of the application.
    /// When you are running in production this should be the URL
    /// to your application.
    ///
    /// *optional*
    ///
    /// default: https://{HTTP_ADDRESS}:{HTTP_PORT}
    pub app_url: Url,

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
    pub client_url: Url,

    /// APP_NAME
    /// This is the name of the application, it will be automatically
    /// filled if it hasn't been provided in the env to be something else then Hoodik
    pub name: String,

    /// APP_VERSION
    /// if this is left empty it will be automatically filled with the version
    /// from the Cargo.toml file.
    pub version: String,
}

impl AppConfig {
    pub(crate) fn new(vars: &mut Vars) -> Self {
        let data_dir = vars.var_default::<String>("DATA_DIR", "".to_string());
        let database_url = vars.maybe_var("DATABASE_URL");
        let address = vars
            .var_default("HTTP_ADDRESS", "localhost".to_string())
            .get();
        let port = vars.var_default("HTTP_PORT", 5443).get();
        let name = vars.var_default("APP_NAME", "Hoodik".to_string());
        let version = vars.get_version();
        let app_url = vars
            .var_default::<Url>(
                "APP_URL",
                Url::parse(&format!("https://{address}:{port}")).unwrap(),
            )
            .get();

        let client_url = vars.var_default("APP_CLIENT_URL", app_url.clone()).get();

        vars.panic_if_errors("AppConfig");

        Self {
            port,
            address,
            data_dir: data_dir.get(),
            database_url: database_url.maybe_get(),
            name: name.get(),
            version,
            app_url,
            client_url,
        }
        .set_env()
    }

    /// Cleanup the data directory
    #[cfg(feature = "mock")]
    pub fn cleanup(&self) {
        log::debug!("Trying to cleanup the data directory: {}", &self.data_dir);

        match fs::remove_dir_all(&self.data_dir) {
            Ok(_) => (),
            Err(_e) => (),
        }
    }

    /// Once the application starts, this will make sure the data_dir actually
    /// exists and its writeable, it will try to create it if possible, but if
    /// it fails it will panic.
    pub fn ensure_data_dir(&mut self, data_dir: Option<String>) {
        let data_dir = data_dir.unwrap_or_else(|| self.data_dir.clone());

        let data_dir = absolute_path(&data_dir)
            .unwrap_or_else(|| panic!("Couldn't get absolute path for '{}'", &data_dir));

        match DirBuilder::new().recursive(true).create(&data_dir) {
            Ok(_) => (),
            Err(e) => println!("Error creating directory: {e:?}"),
        };

        let metadata = fs::metadata(&data_dir).unwrap_or_else(|e| {
            panic!("Got error when attempting to get metadata of a data dir '{data_dir}': {e}")
        });

        if metadata.permissions().readonly() {
            panic!("DATA_DIR is not writeable to the application, aborting...")
        }

        self.data_dir = remove_trailing_slash(data_dir);
    }

    /// Set database url in the env if it wasn't already for the migration
    fn set_env(self) -> Self {
        if let Some(db) = &self.database_url {
            std::env::set_var("DATABASE_URL", db);
        } else {
            std::env::set_var(
                "DATABASE_URL",
                format!("sqlite:{}/sqlite.db?mode=rwc", &self.data_dir),
            );
        }

        self
    }
}
