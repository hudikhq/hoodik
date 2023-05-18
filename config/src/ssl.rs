use error::{AppResult, Error};
use rcgen::generate_simple_self_signed;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    fs::File,
    io::{BufReader, Write},
};

use crate::Config;

pub trait SslConfig {
    /// Build a rustls server config from the provided cert and key files through the environment
    fn build_rustls_config(&self) -> AppResult<ServerConfig>;

    /// Attempt to load the cert and key files from the environment, if they don't exist, generate them
    fn load_or_generate(&self) -> AppResult<(File, File)>;

    /// Use rcgen to generate a simple self signed certificate and key
    fn generate_simple_self_signed(
        &self,
        subject_alt_names: Vec<String>,
    ) -> AppResult<(String, String)> {
        let cert = generate_simple_self_signed(subject_alt_names)?;

        Ok((cert.serialize_pem()?, cert.serialize_private_key_pem()))
    }
}

impl SslConfig for Config {
    fn build_rustls_config(&self) -> AppResult<ServerConfig> {
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        let (cert_file, key_file) = self.load_or_generate()?;

        let cert_file = &mut BufReader::new(cert_file);
        let key_file = &mut BufReader::new(key_file);

        let cert_chain = certs(cert_file)?.into_iter().map(Certificate).collect();
        let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)?
            .into_iter()
            .map(PrivateKey)
            .collect();

        config
            .with_single_cert(cert_chain, keys.remove(0))
            .map_err(Error::from)
    }

    fn load_or_generate(&self) -> AppResult<(File, File)> {
        let cert_file = open_file(&self.ssl_cert_file);
        let key_file = open_file(&self.ssl_key_file);

        match (cert_file, key_file) {
            (Some(cert_file), Some(key_file)) => Ok((cert_file, key_file)),
            (None, None) | (Some(_), None) | (None, Some(_)) => {
                let (cert, key) = self.generate_simple_self_signed(vec![self.address.clone()])?;

                let mut cert_file = File::create(&self.ssl_cert_file)?;
                cert_file.write_all(&cert.into_bytes())?;
                cert_file = File::open(&self.ssl_cert_file)?;

                let mut key_file = File::create(&self.ssl_key_file)?;
                key_file.write_all(&key.into_bytes())?;
                key_file = File::open(&self.ssl_key_file)?;

                Ok((cert_file, key_file))
            }
        }
    }
}

/// Try to open a file, log an error if it happens while trying to open a file
fn open_file(path: &str) -> Option<File> {
    match File::open(path) {
        Ok(f) => Some(f),
        Err(e) => {
            log::error!("Error while trying to open a file '{}': {}", path, e);

            None
        }
    }
}
