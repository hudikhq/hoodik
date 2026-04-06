pub mod app;
pub mod auth;
pub mod config;
pub mod email;
pub(crate) mod helpers;
pub mod s3;
pub mod ssl;
pub mod vars;

use helpers::remove_trailing_slash;

pub use crate::config::Config;

impl Config {
    pub fn get_app_name(&self) -> String {
        self.app.name.clone()
    }

    pub fn get_app_version(&self) -> String {
        self.app.version.clone()
    }

    pub fn get_app_url(&self) -> String {
        remove_trailing_slash(self.app.app_url.to_string())
    }

    pub fn get_client_url(&self) -> String {
        remove_trailing_slash(self.app.client_url.to_string())
    }

    pub fn get_full_bind_address(&self) -> String {
        format!("{}:{}", self.app.address, self.app.port)
    }

    pub fn announce(&self) {
        println!(
            "Starting {} v{} on {}",
            self.get_app_name(),
            self.get_app_version(),
            self.get_full_bind_address()
        );

        println!("-- Using data_dir: {}", self.app.data_dir);

        match self.app.storage_provider.as_str() {
            "s3" => {
                if let Some(ref s3) = self.s3 {
                    println!(
                        "-- Storage provider: s3 (bucket: {}, region: {})",
                        s3.bucket, s3.region
                    );
                }
            }
            _ => {
                println!("-- Storage provider: local");
            }
        }

        if self.ssl.disabled {
            println!("-- SSL is disabled");
        } else {
            println!("-- Using ssl cert: {}", self.ssl.cert_file);
            println!("-- Using ssl key: {}", self.ssl.key_file);
        }

        println!("-- RUST_LOG={:?}", std::env::var("RUST_LOG").ok());
        println!("------------------------------------------");
    }

    /// Emit any warnings collected during configuration initialization.
    /// Call this after env_logger::init() to ensure warnings are visible.
    pub fn emit_warnings(&self) {
        for warning in &self.warnings {
            log::warn!("{}", warning);
        }
    }
}
