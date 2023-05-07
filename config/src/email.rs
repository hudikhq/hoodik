/// Email configuration holder,
/// it can be either SMTP or None.
///
/// To use SMTP you need to set the following environment variables:
/// MAILER_TYPE=smtp
/// SMTP_ADDRESS=smtp.example.com:587
/// SMTP_USERNAME=example
/// SMTP_PASSWORD=secret
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
/// SMTP_DEFAULT_FROM="example@example.com <Full Name>"
#[derive(Debug, Clone)]
pub struct SmtpCredentials {
    pub address: String,
    pub username: String,
    pub password: String,
    pub default_from: String,
}

impl SmtpCredentials {
    pub fn from_env(errors: &mut Vec<String>) -> Option<Self> {
        let address = std::env::var("SMTP_ADDRESS")
            .map(Some)
            .unwrap_or_else(|_| {
                errors.push("SMTP_ADDRESS is not set".to_string());
                None
            })?;

        let username = std::env::var("SMTP_USERNAME")
            .map(Some)
            .unwrap_or_else(|_| {
                errors.push("SMTP_USERNAME is not set".to_string());
                None
            })?;

        let password = std::env::var("SMTP_PASSWORD")
            .map(Some)
            .unwrap_or_else(|_| {
                errors.push("SMTP_USERNAME is not set".to_string());
                None
            })?;

        let default_from = std::env::var("SMTP_DEFAULT_FROM")
            .map(Some)
            .unwrap_or_else(|_| {
                errors.push("SMTP_DEFAULT_FROM is not set".to_string());
                None
            })?;

        Some(Self {
            address,
            username,
            password,
            default_from,
        })
    }
}

impl EmailConfig {
    pub fn new(errors: &mut Vec<String>) -> Self {
        let mailer = std::env::var("MAILER_TYPE")
            .ok()
            .unwrap_or_default()
            .to_lowercase();

        if mailer == "smtp" {
            let credentials = SmtpCredentials::from_env(errors);

            if let Some(c) = credentials {
                return Self::Smtp(c);
            }
        }

        log::warn!("Using mock email sender");

        Self::None
    }
}
