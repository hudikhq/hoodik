use config::email::EmailConfig;
use error::AppResult;
use senders::smtp::SmtpSender;

#[cfg(feature = "mock")]
use crate::senders::mock::MockSender;

pub mod contract;
pub mod senders;
pub mod template;

/// Email sender that can be instantiated by using the application
/// config, it will automatically extract all the needed parts
/// to set the inner into proper sending mode.
pub struct Sender {
    app_name: String,
    app_version: String,
    inner: Box<dyn contract::SenderContract>,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            app_name: self.app_name.clone(),
            app_version: self.app_version.clone(),
            inner: self.inner.boxed_clone(),
        }
    }
}

impl Sender {
    pub fn new(config: &config::Config) -> AppResult<Option<Self>> {
        let app_name = config.get_app_name();
        let app_version = config.get_app_version();

        Ok(match &config.mailer {
            EmailConfig::Smtp(c) => Some(Self {
                app_name,
                app_version,
                inner: Box::new(SmtpSender::new(
                    &c.address,
                    &c.username,
                    &c.password,
                    c.port,
                    &c.tls_mode,
                    &c.default_from,
                )?),
            }),
            EmailConfig::None => None,
        })
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self {
            app_name: "Mock Hoodik".to_string(),
            app_version: "0.1.0".to_string(),
            inner: Box::new(MockSender::new()),
        }
    }
}

#[async_trait::async_trait]
impl contract::SenderContract for Sender {
    async fn send(&self, emails: Vec<template::Template>) -> error::AppResult<usize> {
        self.inner.send(emails).await
    }

    /// We will override the default behavior here because we want it to always have
    /// app version and name in production settings, and this will enable that.
    fn template(&self, subject: &str, pre_header: &str) -> AppResult<template::Template> {
        let mut template = template::Template::new(subject, pre_header)?;

        template.add_template_var("base_app_name", self.app_name.as_str());
        template.add_template_var("base_app_version", self.app_version.as_str());

        Ok(template)
    }

    fn boxed_clone(&self) -> Box<dyn contract::SenderContract> {
        self.inner.boxed_clone()
    }

    #[cfg(feature = "mock")]
    fn has(&self, subject: &str) -> bool {
        self.inner.has(subject)
    }

    #[cfg(feature = "mock")]
    fn find(&self, pat: &str) -> Option<String> {
        self.inner.find(pat)
    }
}
