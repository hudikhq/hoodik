use sea_orm_migration::prelude::*;

/// Admin kill switch for sharing.
///
/// Settings in Hoodik live in the file-backed `settings.json` Settings
/// service, not in the database, so this migration has no SQL side
/// effect. The `sharing.enabled` flag is added to `settings::data::Data`
/// in the same release with `Default::default()` returning `true` and
/// `serde(default)` on the field, so existing settings files load
/// unchanged with the new field auto-populated to enabled.
///
/// The migration is registered as a release marker: anyone diffing
/// migrations between Hoodik versions can pinpoint exactly which release
/// introduced the kill-switch field.
#[derive(DeriveMigrationName)]
pub(crate) struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}
