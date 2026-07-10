use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use error::{AppResult, Error};

use crate::{
    auth::Auth,
    contracts::{cookies::Cookies, migration::Migration, opaque::Opaque, repository::Repository},
    data::{
        claims::Claims,
        opaque::{
            MigrationComplete, MigrationKeysQuery, OpaqueLoginFinish, OpaqueLoginStart,
            OpaqueRegisterFinish, OpaqueRegisterStart, RewrapBatch, SignupRegisterStart,
        },
    },
};
use entity::Uuid;

/// Begin registering (or re-registering) the OPAQUE password file for the
/// authenticated user.
///
/// Request: [crate::data::opaque::OpaqueRegisterStart]
///
/// Response: [crate::data::opaque::OpaqueRegisterStartResponse]
#[route("/api/auth/pake/register/start", method = "POST")]
pub(crate) async fn register_start(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<OpaqueRegisterStart>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let response = auth
        .opaque_register_start(claims.sub, &data.registration_request)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

/// Change the password of an OPAQUE (v2) account: store the new password file
/// and the private-key envelope re-sealed under the new password's `export_key`
/// in one transaction. A session cookie alone is not enough — the request must
/// carry a signature over the new registration upload, proving possession of
/// the account's identity private key (which never lives in the session), plus
/// a TOTP token when 2FA is enabled.
///
/// Request: [crate::data::opaque::OpaqueRegisterFinish]
#[route("/api/auth/pake/register/finish", method = "POST")]
pub(crate) async fn register_finish(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<OpaqueRegisterFinish>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    auth.opaque_register_finish(claims.sub, &data.into_inner())
        .await?;

    Ok(HttpResponse::NoContent().finish())
}

/// Begin OPAQUE registration for a brand-new (v2) signup. Unauthenticated —
/// keyed by the email in the body, which becomes the account's OPAQUE
/// credential identifier. The account itself is created by `/api/auth/register`
/// with the resulting registration upload.
///
/// Request: [crate::data::opaque::SignupRegisterStart]
///
/// Response: [crate::data::opaque::OpaqueRegisterStartResponse]
#[route("/api/auth/register/pake/start", method = "POST")]
pub(crate) async fn signup_register_start(
    context: web::Data<Context>,
    data: web::Json<SignupRegisterStart>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let response = auth
        .opaque_signup_register_start(&data.email, &data.registration_request)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

/// Begin an OPAQUE login. The password never crosses the wire.
///
/// Request: [crate::data::opaque::OpaqueLoginStart]
///
/// Response: [crate::data::opaque::OpaqueLoginStartResponse]
#[route("/api/auth/login/start", method = "POST")]
pub(crate) async fn login_start(
    context: web::Data<Context>,
    data: web::Json<OpaqueLoginStart>,
) -> AppResult<HttpResponse> {
    // Deliberately unthrottled: start carries no secret to guess and always
    // "succeeds" (unknown emails get a decoy), so a failures-only limiter has
    // nothing to count. The password-guess signal is the failed `login/finish`,
    // which is where the throttle lives.
    let auth = Auth::new(&context);
    let response = auth
        .opaque_login_start(&data.email, &data.credential_request)
        .await?;

    Ok(HttpResponse::Ok().json(response))
}

/// Finish an OPAQUE login, issuing a session on success.
///
/// Request: [crate::data::opaque::OpaqueLoginFinish]
///
/// Response: [crate::data::authenticated::Authenticated]
#[route("/api/auth/login/finish", method = "POST")]
pub(crate) async fn login_finish(
    req: HttpRequest,
    context: web::Data<Context>,
    data: web::Json<OpaqueLoginFinish>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let (user_agent, ip) = util::actix::extract_ip_ua(&req);
    let data = data.into_inner();

    // A wrong password fails here, so this is where OPAQUE guessing is throttled.
    // The wire carries only a fresh `login_id`, never the email, so the identity
    // window is unavailable — a failed finish is charged to the source IP alone.
    let now = chrono::Utc::now().timestamp();
    crate::rate_limit::check(None, &ip, now)?;

    let authenticated = match auth
        .opaque_login_finish(data.login_id, &data.credential_finalization, data.token, &user_agent, &ip)
        .await
    {
        Ok(authenticated) => authenticated,
        Err(e) => {
            crate::rate_limit::charge_failure(None, &ip, now);
            return Err(e);
        }
    };

    let mut response = HttpResponse::Ok();
    let (jwt, refresh) = auth.manage_cookies(&authenticated, module_path!())?;

    if !context.config.auth.use_headers_for_auth {
        response.cookie(jwt);
        response.cookie(refresh);
    } else {
        response.append_header(("x-auth-jwt".to_string(), jwt.value()));
        response.append_header(("x-auth-refresh".to_string(), refresh.value()));
    }

    Ok(response.json(authenticated))
}

/// One page of the file keys the authenticated user holds and the public link
/// keys they own, so the client can re-wrap each one during migration. Paged
/// (`?offset=&limit=`) so a large account's key set is never held whole in
/// memory; the response's `next_offset` is the cursor for the next page.
///
/// Response: [crate::data::opaque::MigrationKeys]
#[route("/api/auth/migration/keys", method = "GET")]
pub(crate) async fn migration_keys(
    context: web::Data<Context>,
    claims: Claims,
    q: web::Query<MigrationKeysQuery>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let keys = auth.migration_keys(claims.sub, q.offset, q.limit).await?;

    Ok(HttpResponse::Ok().json(keys))
}

/// Stage one batch of re-wrapped file and link keys for the authenticated
/// legacy account. The client submits its whole re-wrap across several of these
/// before calling `migration/complete`, which applies the accumulated set
/// atomically. Idempotent per key, so a retried batch replaces rather than
/// duplicates.
///
/// Request: [crate::data::opaque::RewrapBatch]
///
/// Registered with an explicit `JsonConfig` in [`crate::routes::configure`], so
/// the route path lives there rather than in a `#[route]` attribute here.
pub(crate) async fn migration_rewrap(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<RewrapBatch>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    auth.stage_rewrap(claims.sub, data.into_inner()).await?;

    Ok(HttpResponse::NoContent().finish())
}

/// Complete the one-shot migration onto Curve25519 + OPAQUE in a single
/// transaction. Runs from the legacy (bcrypt) session the user just opened.
///
/// Request: [crate::data::opaque::MigrationComplete]
///
/// Response: the migrated [entity::users::Model]
///
/// Registered with an explicit `JsonConfig` in [`crate::routes::configure`], so
/// the route path lives there rather than in a `#[route]` attribute here.
pub(crate) async fn migration_complete(
    context: web::Data<Context>,
    claims: Claims,
    data: web::Json<MigrationComplete>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let user = auth.migration_complete(claims.sub, data.into_inner()).await?;

    Ok(HttpResponse::Ok().json(user))
}

/// Return the append-only key transition chain for a user. This allows clients
/// to verify historical fingerprints (TOFU, share grants signed with pre-migration
/// keys, audit events, etc.).
///
/// Query: `?user_id=<uuid>` (optional; defaults to caller)
/// Authenticated.
#[route("/api/auth/key-transitions", method = "GET")]
pub(crate) async fn key_transitions(
    context: web::Data<Context>,
    claims: Claims,
    q: web::Query<std::collections::HashMap<String, String>>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let target = match q.get("user_id") {
        Some(s) => Uuid::parse_str(s).map_err(|_| Error::BadRequest("invalid_user_id".to_string()))?,
        None => claims.sub,
    };
    let rows = auth.list_key_transitions(target).await?;
    Ok(HttpResponse::Ok().json(rows))
}
