/// Helper method to create hash of a password
pub fn hash(password: &str) -> String {
    bcrypt::hash(&password, bcrypt::DEFAULT_COST).unwrap()
}

/// Helper method to verify password hash
pub fn verify(password: &str, hashed_password: &str) -> bool {
    match bcrypt::verify(password, hashed_password) {
        Ok(res) => res,
        Err(_) => false,
    }
}
