use actix_web::{route, web, HttpResponse};
use auth::data::authenticated::Authenticated;
use context::Context;
use error::AppResult;

use crate::{data::create_link::CreateLink, repository::Repository};

/// Create a shareable link for a file.
///
/// Response: [crate::data::app_link::AppLink]
#[route("/api/links", method = "POST")]
pub(crate) async fn create(
    context: web::Data<Context>,
    authenticated: Authenticated,
    create_link: web::Json<CreateLink>,
) -> AppResult<HttpResponse> {
    let context = context.into_inner();
    let repository = Repository::new(&context);

    let app_link = repository
        .create(create_link.into_inner(), &authenticated.user)
        .await?;

    Ok(HttpResponse::Created().json(app_link))
}
