//! OPAQUE (RFC 9807) augmented PAKE — the login handshake that never puts the
//! password on the wire and yields a client-only `export_key` used to derive
//! the private-key envelope KEK ([`crate::envelope::derive_kek`]).
//!
//! The suite is ristretto255 + SHA-512 + TripleDH, with Argon2id (m=64 MiB,
//! t=3, p=1) as the key-stretching function. The KSF parameters are frozen:
//! they are baked into `export_key`, so every client and the server must use
//! byte-identical values forever. All messages cross the API as base64.
//!
//! The server persists one [`opaque_ke::ServerSetup`] (the OPRF seed) and one
//! password file per user; neither reveals the password. The client drives
//! registration once (at signup or migration) and the login KE on every
//! sign-in.

use crate::error::{CryptoResult, Error};

use argon2::{Algorithm, Argon2, Params, Version};
use opaque_ke::rand::rngs::OsRng;
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, Identifiers, RegistrationRequest, RegistrationResponse,
    RegistrationUpload, Ristretto255, ServerLogin, ServerLoginParameters, ServerRegistration,
    ServerSetup, TripleDh,
};

/// Frozen OPAQUE cipher suite. Changing any associated type is a
/// protocol-version break that invalidates every stored registration.
pub struct HoodikCipherSuite;

impl opaque_ke::CipherSuite for HoodikCipherSuite {
    type OprfCs = Ristretto255;
    type KeyExchange = TripleDh<Ristretto255, sha2::Sha512>;
    type Ksf = Argon2<'static>;
}

/// The frozen Argon2id KSF. OWASP's m=64 MiB / t=3 / p=1 — p=1 because WASM
/// computes lanes serially, and these bytes are locked forever by `export_key`.
#[cfg(not(feature = "fast-test-kdf"))]
fn ksf() -> CryptoResult<Argon2<'static>> {
    let params = Params::new(64 * 1024, 3, 1, None)
        .map_err(|e| Error::KeyEncoding(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Cheap Argon2id parameters for the e2e WASM build only. Enabled solely by the
/// `fast-test-kdf` cargo feature, which the production `wasm`/`build` recipes
/// never set — those keep the secure m=64 MiB KSF above.
#[cfg(feature = "fast-test-kdf")]
fn ksf() -> CryptoResult<Argon2<'static>> {
    let params = Params::new(8 * 1024, 1, 1, None)
        .map_err(|e| Error::KeyEncoding(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

fn b64(input: &str) -> CryptoResult<Vec<u8>> {
    crate::base64::decode(input)
}

fn de<T>(r: Result<T, opaque_ke::errors::ProtocolError>, what: &'static str) -> CryptoResult<T> {
    r.map_err(|_| Error::KeyEncoding(format!("invalid opaque {what}")))
}

/// Generate a fresh server OPRF seed. Persist this like a signing key —
/// losing it invalidates every registration.
pub fn server_setup_new() -> String {
    let setup = ServerSetup::<HoodikCipherSuite>::new(&mut OsRng);
    crate::base64::encode(setup.serialize().as_slice())
}

/// Server step 1 of registration: answer the client's request.
pub fn server_registration_start(
    server_setup: &str,
    registration_request: &str,
    username: &[u8],
) -> CryptoResult<String> {
    let setup = de(ServerSetup::<HoodikCipherSuite>::deserialize(&b64(server_setup)?), "server setup")?;
    let request = de(
        RegistrationRequest::<HoodikCipherSuite>::deserialize(&b64(registration_request)?),
        "reg request",
    )?;
    let result = ServerRegistration::<HoodikCipherSuite>::start(&setup, request, username)
        .map_err(|_| Error::KeyEncoding("opaque reg start".into()))?;

    Ok(crate::base64::encode(result.message.serialize().as_slice()))
}

/// Server step 2 of registration: fold the client's upload into the password
/// file to persist for this user.
pub fn server_registration_finish(registration_upload: &str) -> CryptoResult<String> {
    let upload = de(
        RegistrationUpload::<HoodikCipherSuite>::deserialize(&b64(registration_upload)?),
        "reg upload",
    )?;
    let file = ServerRegistration::<HoodikCipherSuite>::finish(upload);

    Ok(crate::base64::encode(file.serialize().as_slice()))
}

/// Server step 1 of login: produce the KE2 response and the login state to
/// carry into [`server_login_finish`].
pub fn server_login_start(
    server_setup: &str,
    password_file: &str,
    credential_request: &str,
    username: &[u8],
) -> CryptoResult<ServerLoginStart> {
    let setup = de(ServerSetup::<HoodikCipherSuite>::deserialize(&b64(server_setup)?), "server setup")?;
    let file = de(
        ServerRegistration::<HoodikCipherSuite>::deserialize(&b64(password_file)?),
        "password file",
    )?;
    let request = de(
        CredentialRequest::<HoodikCipherSuite>::deserialize(&b64(credential_request)?),
        "cred request",
    )?;

    let result = ServerLogin::start(
        &mut OsRng,
        &setup,
        Some(file),
        request,
        username,
        ServerLoginParameters::default(),
    )
    .map_err(|_| Error::KeyEncoding("opaque login start".into()))?;

    Ok(ServerLoginStart {
        state: crate::base64::encode(result.state.serialize().as_slice()),
        response: crate::base64::encode(result.message.serialize().as_slice()),
    })
}

/// Server step 2 of login: verify the client's finalization and recover the
/// shared session key (authentication proof).
pub fn server_login_finish(
    login_state: &str,
    credential_finalization: &str,
) -> CryptoResult<String> {
    let state = de(ServerLogin::<HoodikCipherSuite>::deserialize(&b64(login_state)?), "login state")?;
    let finalization = de(
        CredentialFinalization::<HoodikCipherSuite>::deserialize(&b64(credential_finalization)?),
        "cred finalization",
    )?;
    let result = state
        .finish(finalization, ServerLoginParameters::default())
        .map_err(|_| Error::KeyEncoding("opaque login finish".into()))?;

    Ok(crate::base64::encode(result.session_key.as_slice()))
}

/// Server login state + KE2 response, both base64.
pub struct ServerLoginStart {
    pub state: String,
    pub response: String,
}

/// Client step 1 of registration: blind the password.
pub fn client_registration_start(password: &[u8]) -> CryptoResult<ClientStart> {
    let result = ClientRegistration::<HoodikCipherSuite>::start(&mut OsRng, password)
        .map_err(|_| Error::KeyEncoding("opaque client reg start".into()))?;

    Ok(ClientStart {
        state: crate::base64::encode(result.state.serialize().as_slice()),
        message: crate::base64::encode(result.message.serialize().as_slice()),
    })
}

/// Client step 2 of registration: run the KSF and produce the upload plus the
/// `export_key` (base64) that seeds the envelope KEK.
pub fn client_registration_finish(
    registration_state: &str,
    registration_response: &str,
    password: &[u8],
) -> CryptoResult<ClientExportResult> {
    let state = de(
        ClientRegistration::<HoodikCipherSuite>::deserialize(&b64(registration_state)?),
        "client reg state",
    )?;
    let response = de(
        RegistrationResponse::<HoodikCipherSuite>::deserialize(&b64(registration_response)?),
        "reg response",
    )?;
    let ksf = ksf()?;
    let result = state
        .finish(
            &mut OsRng,
            password,
            response,
            ClientRegistrationFinishParameters::new(Identifiers::default(), Some(&ksf)),
        )
        .map_err(|_| Error::KeyEncoding("opaque client reg finish".into()))?;

    Ok(ClientExportResult {
        message: crate::base64::encode(result.message.serialize().as_slice()),
        export_key: crate::base64::encode(result.export_key.as_slice()),
    })
}

/// Client step 1 of login: blind the password for the KE.
pub fn client_login_start(password: &[u8]) -> CryptoResult<ClientStart> {
    let result = ClientLogin::<HoodikCipherSuite>::start(&mut OsRng, password)
        .map_err(|_| Error::KeyEncoding("opaque client login start".into()))?;

    Ok(ClientStart {
        state: crate::base64::encode(result.state.serialize().as_slice()),
        message: crate::base64::encode(result.message.serialize().as_slice()),
    })
}

/// Client step 2 of login: run the KSF, verify the server, and recover the
/// session key plus the same `export_key` produced at registration. A wrong
/// password fails here.
pub fn client_login_finish(
    login_state: &str,
    credential_response: &str,
    password: &[u8],
) -> CryptoResult<ClientLoginResult> {
    let state = de(
        ClientLogin::<HoodikCipherSuite>::deserialize(&b64(login_state)?),
        "client login state",
    )?;
    let response = de(
        CredentialResponse::<HoodikCipherSuite>::deserialize(&b64(credential_response)?),
        "cred response",
    )?;
    let ksf = ksf()?;
    let result = state
        .finish(
            &mut OsRng,
            password,
            response,
            ClientLoginFinishParameters::new(None, Identifiers::default(), Some(&ksf)),
        )
        .map_err(|_| Error::KeyEncoding("opaque client login finish".into()))?;

    Ok(ClientLoginResult {
        finalization: crate::base64::encode(result.message.serialize().as_slice()),
        session_key: crate::base64::encode(result.session_key.as_slice()),
        export_key: crate::base64::encode(result.export_key.as_slice()),
    })
}

/// Client registration/login start: state to persist + message to send.
#[derive(serde::Serialize)]
pub struct ClientStart {
    pub state: String,
    pub message: String,
}

/// Client registration finish: upload to send + `export_key`.
#[derive(serde::Serialize)]
pub struct ClientExportResult {
    pub message: String,
    pub export_key: String,
}

/// Client login finish: finalization to send + session key + `export_key`.
#[derive(serde::Serialize)]
pub struct ClientLoginResult {
    pub finalization: String,
    pub session_key: String,
    pub export_key: String,
}

#[cfg(test)]
mod tests {
    const USERNAME: &[u8] = b"alice@example.com";
    const PASSWORD: &[u8] = b"correct horse battery staple";

    fn register(setup: &str) -> (String, String) {
        let start = super::client_registration_start(PASSWORD).unwrap();
        let response =
            super::server_registration_start(setup, &start.message, USERNAME).unwrap();
        let finish =
            super::client_registration_finish(&start.state, &response, PASSWORD).unwrap();
        let password_file = super::server_registration_finish(&finish.message).unwrap();
        (password_file, finish.export_key)
    }

    fn login(setup: &str, password_file: &str, password: &[u8]) -> super::CryptoResult<String> {
        let start = super::client_login_start(password)?;
        let server = super::server_login_start(setup, password_file, &start.message, USERNAME)?;
        let finish = super::client_login_finish(&start.state, &server.response, password)?;
        let server_session = super::server_login_finish(&server.state, &finish.finalization)?;
        assert_eq!(finish.session_key, server_session, "KE session keys must agree");
        Ok(finish.export_key)
    }

    #[test]
    fn export_key_is_stable_across_registration_and_login() {
        let setup = super::server_setup_new();
        let (password_file, reg_export) = register(&setup);
        let login_export = login(&setup, &password_file, PASSWORD).unwrap();
        assert_eq!(reg_export, login_export, "export_key must match registration");
    }

    #[test]
    fn wrong_password_fails_login() {
        let setup = super::server_setup_new();
        let (password_file, _) = register(&setup);
        assert!(login(&setup, &password_file, b"wrong password").is_err());
    }

    #[test]
    fn export_key_derives_a_working_envelope_kek() {
        let setup = super::server_setup_new();
        let (password_file, _) = register(&setup);
        let export = login(&setup, &password_file, PASSWORD).unwrap();

        let kek =
            crate::envelope::derive_kek(&crate::base64::decode(&export).unwrap()).unwrap();
        let bundle = b"rsa-pem||ed25519-pem||x25519-pem";
        let sealed = crate::envelope::seal(&kek, bundle).unwrap();
        assert_eq!(crate::envelope::open(&kek, &sealed).unwrap(), bundle);
    }

    #[test]
    fn password_is_absent_from_every_message() {
        let setup = super::server_setup_new();
        let start = super::client_registration_start(PASSWORD).unwrap();
        let response =
            super::server_registration_start(&setup, &start.message, USERNAME).unwrap();
        let finish =
            super::client_registration_finish(&start.state, &response, PASSWORD).unwrap();

        for wire in [&start.message, &response, &finish.message] {
            let bytes = crate::base64::decode(wire).unwrap();
            assert!(
                bytes.windows(PASSWORD.len()).all(|w| w != PASSWORD),
                "password bytes must never appear on the wire"
            );
        }
    }
}
