use google_authenticator::GoogleAuthenticator;

/// Generate two factor secret
pub fn generate_secret() -> String {
    GoogleAuthenticator::new().create_secret(32)
}
