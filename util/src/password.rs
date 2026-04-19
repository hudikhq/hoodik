pub fn hash<T: AsRef<[u8]>>(password: T) -> String {
    bcrypt::hash(password, bcrypt::DEFAULT_COST).unwrap()
}

pub fn verify(password: &str, hashed_password: &str) -> bool {
    bcrypt::verify(password, hashed_password).unwrap_or(false)
}
