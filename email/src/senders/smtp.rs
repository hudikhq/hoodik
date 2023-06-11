use std::str::FromStr;

use crate::template::Template;
use error::{AppResult, Error};
use lettre::message::Mailbox;
use lettre::Transport as _;
use lettre::{transport::smtp::authentication::Credentials, SmtpTransport};

use crate::contract::SenderContract;

#[derive(Clone)]
pub struct SmtpSender {
    smtp: SmtpTransport,
    default_from: Mailbox,
}

impl SmtpSender {
    pub fn new(
        address: &str,
        username: &str,
        password: &str,
        port: u16,
        default_from: &str,
    ) -> AppResult<Self> {
        let smtp = SmtpTransport::relay(address)?.port(port);

        let smtp = smtp
            .credentials(Credentials::new(username.to_string(), password.to_string()))
            .build();

        smtp.test_connection()?;

        Ok(Self {
            smtp,
            default_from: Mailbox::from_str(default_from)?,
        })
    }
}

#[async_trait::async_trait]
impl SenderContract for SmtpSender {
    async fn send(&self, emails: Vec<Template>) -> AppResult<usize> {
        let mut sent = 0;

        for mut email in emails {
            if !email.has_from() {
                email = email.from_mailbox(&self.default_from);
            }

            if email.skip_send() {
                sent += 1;

                continue;
            }

            let message = email.message()?;

            match self.smtp.send(&message) {
                Ok(response) => {
                    if response.is_positive() {
                        sent += 1;
                    } else {
                        log::error!(
                            "Negative response sending email in Smtp: {:?}, message: {:?}",
                            response,
                            message
                        );
                    }
                }
                Err(e) => {
                    log::error!("Error sending email in Smtp: {}, message: {:?}", e, message);

                    return Err(Error::from(e));
                }
            }
        }

        Ok(sent)
    }

    fn boxed_clone(&self) -> Box<dyn SenderContract> {
        Box::new(self.clone())
    }
}
