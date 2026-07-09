pub use sea_orm_migration::prelude::*;

pub(crate) mod m20220101_000001_create_users;
pub(crate) mod m20220101_000002_create_user_actions;
pub(crate) mod m20230114_091730_create_sessions;
pub(crate) mod m20230409_091730_create_files;
pub(crate) mod m20230409_101730_create_user_files;
pub(crate) mod m20230429_091730_create_tokens;
pub(crate) mod m20230429_101730_create_file_tokens;
pub(crate) mod m20230521_074334_create_links;
pub(crate) mod m20230612_074334_create_invitations;
pub(crate) mod m20240915_074334_alter_files_add_hashes;
pub(crate) mod m20260326_000001_alter_files_add_cipher;
pub(crate) mod m20260406_000001_alter_files_add_editable;
pub(crate) mod m20260418_000001_alter_files_add_versioning;
pub(crate) mod m20260418_000002_create_file_versions;
pub(crate) mod m20260601_000001_alter_user_files_share_role;
pub(crate) mod m20260601_000002_alter_user_files_share_metadata;
pub(crate) mod m20260601_000003_add_user_files_indexes;
pub(crate) mod m20260601_000004_create_share_events;
pub(crate) mod m20260601_000005_alter_users_share_notifications;
pub(crate) mod m20260601_000006_create_share_groups;
pub(crate) mod m20260601_000007_create_share_group_members;
pub(crate) mod m20260601_000008_alter_settings_sharing_kill_switch;
pub(crate) mod m20260601_000009_alter_files_folder_member_list;
pub(crate) mod m20260601_000010_alter_share_group_members_role;
pub(crate) mod m20260705_000001_alter_users_key_type;
pub(crate) mod m20260705_000002_create_key_transitions;
pub(crate) mod m20260705_000003_create_opaque_tables;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_users::Migration),
            Box::new(m20220101_000002_create_user_actions::Migration),
            Box::new(m20230114_091730_create_sessions::Migration),
            Box::new(m20230409_091730_create_files::Migration),
            Box::new(m20230409_101730_create_user_files::Migration),
            Box::new(m20230429_091730_create_tokens::Migration),
            Box::new(m20230429_101730_create_file_tokens::Migration),
            Box::new(m20230521_074334_create_links::Migration),
            Box::new(m20230612_074334_create_invitations::Migration),
            Box::new(m20240915_074334_alter_files_add_hashes::Migration),
            Box::new(m20260326_000001_alter_files_add_cipher::Migration),
            Box::new(m20260406_000001_alter_files_add_editable::Migration),
            Box::new(m20260418_000001_alter_files_add_versioning::Migration),
            Box::new(m20260418_000002_create_file_versions::Migration),
            Box::new(m20260601_000001_alter_user_files_share_role::Migration),
            Box::new(m20260601_000002_alter_user_files_share_metadata::Migration),
            Box::new(m20260601_000003_add_user_files_indexes::Migration),
            Box::new(m20260601_000004_create_share_events::Migration),
            Box::new(m20260601_000005_alter_users_share_notifications::Migration),
            Box::new(m20260601_000006_create_share_groups::Migration),
            Box::new(m20260601_000007_create_share_group_members::Migration),
            Box::new(m20260601_000008_alter_settings_sharing_kill_switch::Migration),
            Box::new(m20260601_000009_alter_files_folder_member_list::Migration),
            Box::new(m20260601_000010_alter_share_group_members_role::Migration),
            Box::new(m20260705_000001_alter_users_key_type::Migration),
            Box::new(m20260705_000002_create_key_transitions::Migration),
            Box::new(m20260705_000003_create_opaque_tables::Migration),
        ]
    }
}
