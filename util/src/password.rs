/// Helper method to create hash of a password
pub fn hash<T: AsRef<[u8]>>(password: T) -> String {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap()
}

/// Helper method to verify password hash
pub fn verify(password: &str, hashed_password: &str) -> bool {
    bcrypt::verify(password, hashed_password).unwrap_or(false)
}
