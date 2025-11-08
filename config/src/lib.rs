pub mod app;
pub mod auth;
pub mod config;
pub mod email;
pub(crate) mod helpers;
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

        if self.ssl.disabled {
            println!("-- SSL is disabled");
        } else {
            println!("-- Using ssl cert: {}", self.ssl.cert_file);
            println!("-- Using ssl key: {}", self.ssl.key_file);
        }

        println!("-- RUST_LOG={:?}", std::env::var("RUST_LOG").ok());
        println!("------------------------------------------");
    }

    /// Emit any deprecation warnings after logging has been initialized.
    /// Call this after env_logger::init() to ensure warnings are visible.
    pub fn emit_deprecation_warnings(&self) {
        if let email::EmailConfig::Smtp(smtp) = &self.mailer {
            if smtp.used_deprecated_default_from {
                log::warn!(
                    "SMTP_DEFAULT_FROM is deprecated and will be removed in a future version. \
                    Please use SMTP_DEFAULT_FROM_EMAIL and SMTP_DEFAULT_FROM_NAME instead."
                );
            }
        }
    }
}
