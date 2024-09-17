use google_authenticator::GoogleAuthenticator;

/// Macro to create a required rule for a field with additional
/// condition to in order to be required
#[macro_export]
macro_rules! rule_required_if {
    ($field:ident, $check:expr) => {
        Rule::new(stringify!($field), |obj: &Self, error| {
            let condition: bool = $check(obj.$field.as_ref(), obj);

            if condition && obj.$field.is_none() {
                error.add("required")
            }
        })
    };
    () => {
        
    };
}

/// Validate password to be certain strength according to zxcvbn
pub fn validate_password(password: &str) -> bool {
    let entropy = match zxcvbn::zxcvbn(password, &[]) {
        Ok(e) => e,
        Err(_) => return false,
    };

    entropy.score() > 3
}

/// Validate the provided token with the provided secret that it matches
pub fn validate_otp(secret: &str, token: Option<&String>) -> bool {
    match token {
        Some(t) => GoogleAuthenticator::new().verify_code(secret, t, 1, 0),
        None => false,
    }
}
