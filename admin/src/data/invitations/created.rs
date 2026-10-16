use entity::invitations;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Created {
    #[serde(flatten)]
    pub invitation: invitations::Model,
    /// False when the server has no email sender, so the client has to share
    /// the registration link itself.
    pub email_sent: bool,
}
