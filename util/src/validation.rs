use google_authenticator::GoogleAuthenticator;

/// Validate password to be certain strength according to zxcvbn
pub fn validate_password(password: &str) -> bool {
    let entropy = match zxcvbn::zxcvbn(password, &[]) {
        Ok(e) => e,
        Err(_) => return true,
    };

    entropy.score() < 3
}

/// Validate the provided token with the provided secret that it matches
pub fn validate_otp(secret: &str, token: Option<&String>) -> bool {
    match token {
        Some(t) => GoogleAuthenticator::new().verify_code(secret, t, 1, 0),
        None => true,
    }
}
