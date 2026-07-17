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

/// The KSF parameters that shape OPAQUE's `export_key`. Recorded per user at
/// registration so the work factor can be raised later without a mass lockout:
/// a raised registration stores the new values while existing accounts keep
/// logging in with the ones their envelope was sealed under.
pub struct KsfParams {
    pub algorithm: String,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
}

/// The current default KSF: OWASP's m=64 MiB / t=3 / p=1 — p=1 because WASM
/// computes lanes serially. New registrations and migrations record these, and
/// `login/start` returns them for accounts that hold no record of their own.
#[cfg(not(feature = "fast-test-kdf"))]
pub fn current_ksf_params() -> KsfParams {
    KsfParams { algorithm: "argon2id".to_string(), m_cost: 64 * 1024, t_cost: 3, p_cost: 1 }
}

/// Cheap defaults for the e2e WASM build only, enabled solely by the
/// `fast-test-kdf` cargo feature that the production `wasm`/`build` recipes
/// never set.
#[cfg(feature = "fast-test-kdf")]
pub fn current_ksf_params() -> KsfParams {
    KsfParams { algorithm: "argon2id".to_string(), m_cost: 8 * 1024, t_cost: 1, p_cost: 1 }
}

fn build_argon2(m_cost: u32, t_cost: u32, p_cost: u32) -> CryptoResult<Argon2<'static>> {
    let params = Params::new(m_cost, t_cost, p_cost, None)
        .map_err(|e| Error::KeyEncoding(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// The current-default KSF as an `Argon2`, for callers that carry no per-user
/// parameters (new-account registration, and the legacy test call paths).
fn ksf() -> CryptoResult<Argon2<'static>> {
    let p = current_ksf_params();
    build_argon2(p.m_cost, p.t_cost, p.p_cost)
}

/// Build the KSF for a user's stored parameters. The `fast-test-kdf` build
/// ignores them and always uses the cheap current KSF, so the e2e client stays
/// cheap regardless of what a server returned; only real builds honour them.
fn ksf_for_params(m_cost: u32, t_cost: u32, p_cost: u32) -> CryptoResult<Argon2<'static>> {
    #[cfg(feature = "fast-test-kdf")]
    {
        let _ = (m_cost, t_cost, p_cost);
        ksf()
    }
    #[cfg(not(feature = "fast-test-kdf"))]
    {
        build_argon2(m_cost, t_cost, p_cost)
    }
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
    registration_finish(registration_state, registration_response, password, &ksf()?)
}

/// As [`client_registration_finish`], stretching the password with an explicit
/// per-user KSF instead of the current default.
pub fn client_registration_finish_with_params(
    registration_state: &str,
    registration_response: &str,
    password: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> CryptoResult<ClientExportResult> {
    registration_finish(
        registration_state,
        registration_response,
        password,
        &ksf_for_params(m_cost, t_cost, p_cost)?,
    )
}

fn registration_finish(
    registration_state: &str,
    registration_response: &str,
    password: &[u8],
    ksf: &Argon2<'static>,
) -> CryptoResult<ClientExportResult> {
    let state = de(
        ClientRegistration::<HoodikCipherSuite>::deserialize(&b64(registration_state)?),
        "client reg state",
    )?;
    let response = de(
        RegistrationResponse::<HoodikCipherSuite>::deserialize(&b64(registration_response)?),
        "reg response",
    )?;
    let result = state
        .finish(
            &mut OsRng,
            password,
            response,
            ClientRegistrationFinishParameters::new(Identifiers::default(), Some(ksf)),
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
    login_finish(login_state, credential_response, password, &ksf()?)
}

/// As [`client_login_finish`], stretching the password with an explicit
/// per-user KSF (the values `login/start` returned) instead of the default.
pub fn client_login_finish_with_params(
    login_state: &str,
    credential_response: &str,
    password: &[u8],
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
) -> CryptoResult<ClientLoginResult> {
    login_finish(
        login_state,
        credential_response,
        password,
        &ksf_for_params(m_cost, t_cost, p_cost)?,
    )
}

fn login_finish(
    login_state: &str,
    credential_response: &str,
    password: &[u8],
    ksf: &Argon2<'static>,
) -> CryptoResult<ClientLoginResult> {
    let state = de(
        ClientLogin::<HoodikCipherSuite>::deserialize(&b64(login_state)?),
        "client login state",
    )?;
    let response = de(
        CredentialResponse::<HoodikCipherSuite>::deserialize(&b64(credential_response)?),
        "cred response",
    )?;
    let result = state
        .finish(
            &mut OsRng,
            password,
            response,
            ClientLoginFinishParameters::new(None, Identifiers::default(), Some(ksf)),
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

    /// These Argon2id parameters feed every account's OPAQUE `export_key`, which
    /// unwraps the private-key envelope. Change any of them and no existing
    /// account can open its keys again; this test freezes them.
    #[cfg(not(feature = "fast-test-kdf"))]
    #[test]
    fn production_ksf_parameters_are_frozen() {
        let ksf = super::ksf().unwrap();
        let params = ksf.params();
        assert_eq!(params.m_cost(), 65536);
        assert_eq!(params.t_cost(), 3);
        assert_eq!(params.p_cost(), 1);

        // Argon2 exposes neither its algorithm nor its version, so a
        // known-answer vector pins them (Argon2id, 0x13) alongside the costs:
        // any drift in the KSF config changes these bytes.
        let mut derived = [0u8; 32];
        ksf.hash_password_into(b"hoodik", b"opaque-ksf-vector", &mut derived)
            .unwrap();
        assert_eq!(
            derived,
            [
                205, 0, 205, 154, 129, 227, 22, 118, 38, 116, 158, 172, 124, 237, 168, 106, 255,
                232, 44, 143, 62, 19, 80, 182, 219, 103, 108, 252, 3, 135, 152, 24,
            ]
        );
    }

    #[cfg(feature = "fast-test-kdf")]
    #[test]
    fn fast_test_ksf_parameters_are_reduced() {
        let ksf = super::ksf().unwrap();
        let params = ksf.params();
        assert_eq!(params.m_cost(), 8192);
        assert_eq!(params.t_cost(), 1);
        assert_eq!(params.p_cost(), 1);
    }
}
