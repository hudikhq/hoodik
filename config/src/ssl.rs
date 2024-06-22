use error::{AppResult, Error};
use rcgen::generate_simple_self_signed;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{
    fs::File,
    io::{BufReader, Write},
};

use crate::{app::AppConfig, vars::Vars};

#[derive(Debug, Clone)]
pub struct SslConfig {
    /// SSL_DISABLED: Disable SSL, if this is set to true, the server will not use SSL
    /// even if the cert and key files are provided.
    /// This is useful for development and testing.
    /// 
    /// *optional*
    /// 
    /// default: false
    pub disabled: bool,

    /// SSL_CERT_FILE: Location of the ssl cert file, this will be loaded and setup on to the server
    /// if you don't provide this, the server will generate a self signed certificate
    /// and place them in the /tmp directory. This is not recommended for production.
    ///
    /// *optional*
    ///
    /// default: DATA_DIR/hoodik.crt.pem
    pub cert_file: String,

    /// SSL_KEY_FILE: Location of the ssl key file, this will be loaded and setup on to the server
    /// if you don't provide this, the server will generate a self signed certificate
    /// and place them in the /tmp directory. This is not recommended for production.
    ///
    /// *optional*
    ///
    /// default: DATA_DIR/hoodik.key.pem
    pub key_file: String,
}

impl SslConfig {
    pub(crate) fn new(app: &AppConfig, vars: &mut Vars) -> Self {
        let disabled = vars.var_default("SSL_DISABLED", false);

        let cert_file =
            vars.var_default("SSL_CERT_FILE", format!("{}/hoodik.crt.pem", app.data_dir));
        let key_file = vars.var_default("SSL_KEY_FILE", format!("{}/hoodik.key.pem", app.data_dir));

        vars.panic_if_errors("SslConfig");

        Self {
            disabled: disabled.get(),
            cert_file: cert_file.get(),
            key_file: key_file.get(),
        }
    }

    /// Build a rustls server config from the provided cert and key files through the environment
    pub fn build_rustls_config(&self, names: Vec<String>) -> AppResult<ServerConfig> {
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth();

        let (cert_file, key_file) = self.load_or_generate(names)?;

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

    /// Take the set file names for ssl cert and key, try to load them, if they don't exist, generate them
    fn load_or_generate(&self, names: Vec<String>) -> AppResult<(File, File)> {
        let cert_file = open_file(&self.cert_file);
        let key_file = open_file(&self.key_file);

        match (cert_file, key_file) {
            (Some(cert_file), Some(key_file)) => Ok((cert_file, key_file)),
            (None, None) | (Some(_), None) | (None, Some(_)) => {
                let (cert, key) = self.generate_simple_self_signed(names)?;

                let mut cert_file = File::create(&self.cert_file)?;
                cert_file.write_all(&cert.into_bytes())?;
                cert_file = File::open(&self.cert_file)?;

                let mut key_file = File::create(&self.key_file)?;
                key_file.write_all(&key.into_bytes())?;
                key_file = File::open(&self.key_file)?;

                Ok((cert_file, key_file))
            }
        }
    }

    /// Use rcgen to generate a simple self signed certificate and key
    fn generate_simple_self_signed(
        &self,
        subject_alt_names: Vec<String>,
    ) -> AppResult<(String, String)> {
        let cert = generate_simple_self_signed(subject_alt_names)?;

        Ok((cert.serialize_pem()?, cert.serialize_private_key_pem()))
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
