pub mod capabilities;
pub mod create;
pub mod delete;
pub mod discover;
pub mod events;
pub mod evict_from_folder;
pub mod folder_members;
pub mod fork;
pub mod groups;
pub mod list;
pub mod mine;
pub mod move_into_shared;
pub mod move_out_of_shared;
pub mod upload_multikey;

pub fn configure(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(capabilities::capabilities);
    cfg.service(create::create);
    cfg.service(discover::discover);
    cfg.service(folder_members::folder_members);
    cfg.service(upload_multikey::upload_multikey);
    cfg.service(evict_from_folder::evict_from_folder);
    cfg.service(move_into_shared::move_into_shared);
    cfg.service(move_out_of_shared::move_out_of_shared);
    // Group routes must register before the catch-all `DELETE
    // /api/shares/{file_id}/{user_id}` revoke route — actix-web's
    // matcher walks services in registration order and the revoke
    // pattern happily eats `groups/{id}` as `(file_id=groups,
    // user_id={id})` otherwise. Within the group surface, more-specific
    // paths register before their shorter prefixes for the same reason.
    cfg.service(groups::create_group);
    cfg.service(groups::list_groups);
    cfg.service(groups::group_members);
    cfg.service(groups::set_member_role);
    cfg.service(groups::add_group_member);
    cfg.service(groups::remove_group_member);
    cfg.service(groups::rename_group);
    cfg.service(groups::delete_group);
    cfg.service(fork::fork);
    cfg.service(delete::delete);
    // More-specific paths must register before the catch-all
    // `/api/shares/{file_id}` route, or actix-web routes `/mine` and
    // `/events` into the path-param handler.
    cfg.service(mine::mine_by);
    cfg.service(mine::mine);
    cfg.service(events::events);
    cfg.service(list::list);
}

pub(crate) mod gate {
    use error::{AppResult, Error};

    /// Reject every share-API call when the admin kill switch has flipped
    /// `Settings.sharing.enabled` to `false`. The public capability route
    /// remains accessible so clients can detect the disabled state and
    /// hide their UI fail-closed.
    pub(crate) async fn ensure_enabled(context: &context::Context) -> AppResult<()> {
        if context.settings.inner().await.sharing.enabled() {
            Ok(())
        } else {
            Err(Error::ServiceUnavailable("sharing_disabled".to_string()))
        }
    }
}
