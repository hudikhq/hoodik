use entity::invitations;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Paginated {
    pub invitations: Vec<invitations::Model>,
    pub total: u64,
}
