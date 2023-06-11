#![allow(rustdoc::invalid_html_tags)]

use crate::vars::Vars;

/// Email configuration holder,
/// it can be either SMTP or None.
///
/// To use SMTP you need to set the following environment variables:
/// MAILER_TYPE=smtp
/// SMTP_ADDRESS=smtp.example.com:587
/// SMTP_USERNAME=example
/// SMTP_PASSWORD=secret
/// SMTP_PORT=465 # optional
/// SMTP_DEFAULT_FROM="example@example.com <Full Name>"
#[derive(Debug, Clone)]
pub enum EmailConfig {
    Smtp(SmtpCredentials),
    None,
}

/// SMTP credentials holder.
/// It can be instantiated by using the following environment variables:
/// SMTP_ADDRESS=smtp.example.com:587
/// SMTP_USERNAME=example
/// SMTP_PASSWORD=secret
/// SMTP_PORT=465 # optional
/// SMTP_DEFAULT_FROM="example@example.com <Full Name>"
#[derive(Debug, Clone)]
pub struct SmtpCredentials {
    pub address: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub default_from: String,
}

impl SmtpCredentials {
    fn new(vars: &mut Vars) -> Box<dyn FnOnce() -> Self> {
        let address = vars.var::<String>("SMTP_ADDRESS");
        let username = vars.var::<String>("SMTP_USERNAME");
        let password = vars.var::<String>("SMTP_PASSWORD");
        let port = vars.var_default::<u16>("SMTP_PORT", 465);
        let default_from = vars.var::<String>("SMTP_DEFAULT_FROM");

        Box::new(move || Self {
            address: address.get(),
            username: username.get(),
            password: password.get(),
            port: port.get(),
            default_from: default_from.get(),
        })
    }
}

impl EmailConfig {
    pub(crate) fn new(vars: &mut Vars) -> Self {
        let mailer = vars.var_default("MAILER_TYPE", "".to_string()).get();

        if mailer == "smtp" {
            let credentials = SmtpCredentials::new(vars);

            vars.panic_if_errors("EmailConfig");

            Self::Smtp(credentials())
        } else {
            Self::None
        }
    }
}
