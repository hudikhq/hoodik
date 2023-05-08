use crate::template::Template;
use error::AppResult;

/// Sender contract that will be setup on the
/// context in order to enable sending emails
#[async_trait::async_trait]
pub trait SenderContract
where
    Self: Send + Sync,
{
    /// Send generated emails
    async fn send(&self, emails: Vec<Template>) -> AppResult<usize>;

    /// Create a new email template with the base HTML
    fn template(&self, subject: &str, pre_header: &str) -> AppResult<Template> {
        Template::new(subject, pre_header)
    }

    /// Clone the inner sender
    fn boxed_clone(&self) -> Box<dyn SenderContract>;

    #[cfg(feature = "mock")]
    /// Check if the mock sender has a specific subject
    /// this can be used when testing if an email has been sent
    fn has(&self, _subject: &str) -> bool {
        false
    }

    #[cfg(feature = "mock")]
    /// Find a specific subject in the mock sender
    fn find(&self, _pat: &str) -> Option<String> {
        None
    }
}
