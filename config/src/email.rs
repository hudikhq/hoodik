#![allow(rustdoc::invalid_html_tags)]

use crate::vars::Vars;

/// TLS mode for SMTP connection
#[derive(Debug, Clone, PartialEq)]
pub enum TlsMode {
    /// STARTTLS - typically used on port 587
    StartTls,
    /// Implicit TLS - typically used on port 465
    ImplicitTls,
    /// No TLS - typically used on port 25 (development only)
    None,
}

impl TlsMode {
    /// Auto-detect TLS mode based on port
    fn from_port(port: u16) -> Self {
        match port {
            587 => TlsMode::StartTls,
            465 => TlsMode::ImplicitTls,
            25 => TlsMode::None,
            _ => TlsMode::ImplicitTls, // Default to implicit TLS for unknown ports
        }
    }

    /// Parse TLS mode from string
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "starttls" | "start_tls" => Some(TlsMode::StartTls),
            "implicit" | "tls" | "ssl" => Some(TlsMode::ImplicitTls),
            "none" | "plain" => Some(TlsMode::None),
            _ => None,
        }
    }
}

/// Email configuration holder,
/// it can be either SMTP or None.
///
/// To use SMTP you need to set the following environment variables:
/// MAILER_TYPE=smtp
/// SMTP_ADDRESS=smtp.example.com:587
/// SMTP_USERNAME=example
/// SMTP_PASSWORD=secret
/// SMTP_PORT=465 # optional (default: 465)
/// SMTP_TLS_MODE=starttls # optional (values: starttls, implicit, none - auto-detected from port if not set)
/// SMTP_DEFAULT_FROM_EMAIL=example@example.com
/// SMTP_DEFAULT_FROM_NAME="Full Name" # optional
/// SMTP_DEFAULT_FROM="example@example.com <Full Name>" # DEPRECATED: Use SMTP_DEFAULT_FROM_EMAIL and SMTP_DEFAULT_FROM_NAME instead
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
/// SMTP_PORT=465 # optional (default: 465)
/// SMTP_TLS_MODE=starttls # optional (values: starttls, implicit, none - auto-detected from port if not set)
/// SMTP_DEFAULT_FROM_EMAIL=example@example.com
/// SMTP_DEFAULT_FROM_NAME="Full Name" # optional
/// SMTP_DEFAULT_FROM="example@example.com <Full Name>" # DEPRECATED: Use SMTP_DEFAULT_FROM_EMAIL and SMTP_DEFAULT_FROM_NAME instead
#[derive(Debug, Clone)]
pub struct SmtpCredentials {
    pub address: String,
    pub username: String,
    pub password: String,
    pub port: u16,
    pub default_from: String,
    pub tls_mode: TlsMode,
    #[allow(dead_code)]
    pub(crate) used_deprecated_default_from: bool,
}

impl SmtpCredentials {
    fn new(vars: &mut Vars) -> Box<dyn FnOnce() -> Self> {
        let address = vars.var::<String>("SMTP_ADDRESS");
        let username = vars.var::<String>("SMTP_USERNAME");
        let password = vars.var::<String>("SMTP_PASSWORD");
        let port = vars.var_default::<u16>("SMTP_PORT", 465);
        
        // New variables (preferred)
        let default_from_email = vars.maybe_var::<String>("SMTP_DEFAULT_FROM_EMAIL");
        let default_from_name = vars.maybe_var::<String>("SMTP_DEFAULT_FROM_NAME");
        
        // Old variable (deprecated)
        let smtp_default_from = vars.maybe_var::<String>("SMTP_DEFAULT_FROM");
        
        let tls_mode_str = vars.var_default::<String>("SMTP_TLS_MODE", String::new());

        Box::new(move || {
            let port_value = port.get();
            let tls_mode_str_value = tls_mode_str.get();
            
            // Determine TLS mode: explicit config overrides auto-detection
            let tls_mode = if !tls_mode_str_value.is_empty() {
                TlsMode::from_str(&tls_mode_str_value)
                    .unwrap_or_else(|| {
                        log::warn!(
                            "Invalid SMTP_TLS_MODE '{}', auto-detecting from port {}",
                            tls_mode_str_value,
                            port_value
                        );
                        TlsMode::from_port(port_value)
                    })
            } else {
                TlsMode::from_port(port_value)
            };

            // Determine default_from based on new or old variables
            let (default_from, used_deprecated_default_from) = match (default_from_email.maybe_get(), default_from_name.maybe_get()) {
                (Some(email), Some(name)) if !email.is_empty() && !name.is_empty() => {
                    // Both email and name provided: format as "Name <email@example.com>"
                    (format!("{} <{}>", name, email), false)
                }
                (Some(email), _) if !email.is_empty() => {
                    // Only email provided
                    (format!("Hoodik <{}>", email), false)
                }
                _ => {
                    // Fall back to deprecated SMTP_DEFAULT_FROM
                    match smtp_default_from.maybe_get() {
                        Some(old_value) if !old_value.is_empty() => {
                            (old_value, true)
                        }
                        _ => {
                            // This will cause an error later when trying to parse the mailbox
                            (String::new(), false)
                        }
                    }
                }
            };

            Self {
                address: address.get(),
                username: username.get(),
                password: password.get(),
                port: port_value,
                default_from,
                tls_mode,
                used_deprecated_default_from,
            }
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
