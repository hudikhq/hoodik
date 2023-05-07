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
    inner: Box<dyn contract::SenderContract>,
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.boxed_clone(),
        }
    }
}

impl Sender {
    pub fn new(config: &config::Config) -> AppResult<Option<Self>> {
        Ok(match &config.mailer {
            EmailConfig::Smtp(c) => Some(Self {
                inner: Box::new(SmtpSender::new(
                    &c.address,
                    &c.username,
                    &c.password,
                    &c.default_from,
                )?),
            }),
            EmailConfig::None => None,
        })
    }

    #[cfg(feature = "mock")]
    pub fn mock() -> Self {
        Self {
            inner: Box::new(MockSender::new()),
        }
    }
}

#[async_trait::async_trait]
impl contract::SenderContract for Sender {
    async fn send(&self, emails: Vec<template::Template>) -> error::AppResult<usize> {
        self.inner.send(emails).await
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
