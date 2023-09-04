use crate::rule::RuleValidate;

use super::{whitelist::Whitelist, Blacklist};
use error::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Users {
    quota_bytes: Option<u64>,
    allow_register: bool,
    enforce_email_activation: bool,
    email_whitelist: Option<Whitelist>,
    email_blacklist: Option<Blacklist>,
}

impl Default for Users {
    fn default() -> Self {
        Self {
            quota_bytes: None,
            allow_register: true,
            enforce_email_activation: false,
            email_whitelist: None,
            email_blacklist: None,
        }
    }
}

impl Users {
    /// Default per user quota in bytes.
    pub fn quota_bytes(&self) -> Option<u64> {
        self.quota_bytes
    }

    /// Allow users to register freely. If false, only whitelisted emails can register,
    /// or the ones that were invited.
    pub fn allow_register(&self) -> bool {
        self.allow_register
    }

    /// Should the application enforce email activation.
    /// This will prevent users from logging in until they activate their email.
    ///
    /// In case the activation is not possible (no sender, or something else) it will be skipped
    pub fn enforce_email_activation(&self) -> bool {
        self.enforce_email_activation
    }

    /// Validate users email if its allowed to register.
    pub fn email_whitelist_valid(&self, input: &str) -> bool {
        if let Some(ref whitelist) = self.email_whitelist {
            return whitelist.valid(input);
        }

        false
    }

    /// Validate users email if its not blacklisted.
    pub fn email_blacklist_valid(&self, input: &str) -> bool {
        if let Some(ref blacklist) = self.email_blacklist {
            return blacklist.valid(input);
        }

        true
    }

    /// Check if user can register.
    pub fn can_register(&self, email: &str) -> bool {
        (self.allow_register || self.email_whitelist_valid(email))
            && self.email_blacklist_valid(email)
    }

    /// Throw an error if the user can't register.
    pub fn can_register_or_else<T: FnOnce() -> AppResult<()>>(
        &self,
        email: &str,
        el: T,
    ) -> AppResult<()> {
        if !self.can_register(email) {
            return el();
        }

        Ok(())
    }
}
