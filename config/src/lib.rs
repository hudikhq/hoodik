pub mod app;
pub mod auth;
pub mod config;
pub mod email;
pub mod ssl;
pub mod vars;

pub use crate::config::Config;

impl Config {
    pub fn get_app_name(&self) -> String {
        self.app.name.clone()
    }

    pub fn get_app_version(&self) -> String {
        self.app.version.clone()
    }

    pub fn get_app_url(&self) -> String {
        self.app.app_url.to_string()
    }

    pub fn get_client_url(&self) -> String {
        self.app.client_url.to_string()
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
        println!("-- Using ssl cert: {}", self.ssl.cert_file);
        println!("-- Using ssl key: {}", self.ssl.key_file);
        println!("-- RUST_LOG={:?}", std::env::var("RUST_LOG").ok());
        println!("------------------------------------------");
    }
}
