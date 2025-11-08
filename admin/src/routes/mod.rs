pub mod files;
pub mod invitations;
pub mod sessions;
pub mod settings;
pub mod users;

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(files::index)
        .service(invitations::create)
        .service(invitations::expire)
        .service(invitations::index)
        .service(sessions::index)
        .service(sessions::kill)
        .service(sessions::kill_for_user)
        .service(users::get)
        .service(users::index)
        .service(users::update)
        .service(users::remove)
        .service(settings::index)
        .service(settings::update)
        .service(settings::test_email)
        .service(users::remove_tfa);
}
