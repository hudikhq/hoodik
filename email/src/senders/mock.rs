use std::sync::{Arc, Mutex};

use crate::contract::SenderContract;
use crate::template::Template;
use error::AppResult;

#[derive(Clone)]
pub struct MockSender {
    sent_subjects: Arc<Mutex<Vec<String>>>,
}

impl MockSender {
    pub fn new() -> Self {
        Self {
            sent_subjects: Arc::new(Mutex::new(vec![])),
        }
    }
}

#[async_trait::async_trait]
impl SenderContract for MockSender {
    async fn send(&self, emails: Vec<Template>) -> AppResult<usize> {
        let len = emails.len();

        for mut email in emails {
            if !email.has_from() {
                email = email.from("Mock Test <mock@test.com>")?;
            }

            let email = email.message()?;

            let message = String::from_utf8(email.formatted())
                .unwrap()
                .split("\r\n")
                .filter(|s| s.contains("Subject"))
                .collect::<Vec<&str>>()
                .join("");

            let subject = message.replace("Subject: ", "");

            self.sent_subjects.lock().unwrap().push(subject);
        }

        Ok(len)
    }

    fn has(&self, subject: &str) -> bool {
        return self
            .sent_subjects
            .lock()
            .unwrap()
            .contains(&subject.to_owned());
    }

    #[cfg(feature = "mock")]
    fn find(&self, pat: &str) -> Option<String> {
        let sent_subjects = self.sent_subjects.lock().unwrap();

        for subject in sent_subjects.iter() {
            if subject.contains(pat) {
                return Some(subject.to_string());
            }
        }

        None
    }

    fn boxed_clone(&self) -> Box<dyn SenderContract> {
        Box::new(self.clone())
    }
}
