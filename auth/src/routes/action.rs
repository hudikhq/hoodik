use actix_web::{route, web, HttpRequest, HttpResponse};
use context::Context;
use entity::Uuid;
use error::AppResult;

use crate::{auth::Auth, contracts::register::Register};

/// Activation link in the email will point towards frontend application.
///
/// The frontend, once the link has been opened will make a HTTP post call to the
/// backend with the action `activate-email` and the id of the action,
/// which will verify users account.
///
/// Response: [entity::users::Model]
#[route("/api/auth/action/{action}/{id}", method = "POST")]
pub(crate) async fn action(
    req: HttpRequest,
    context: web::Data<Context>,
) -> AppResult<HttpResponse> {
    let auth = Auth::new(&context);
    let action: String = util::actix::path_var(&req, "action")?;
    let id: Uuid = util::actix::path_var(&req, "id")?;

    match action.as_str() {
        "activate-email" => {
            let user = auth.activate(id).await?;

            Ok(HttpResponse::Ok().json(user))
        }
        _ => Err(error::Error::BadRequest(format!("unknown_action:{action}"))),
    }
}
