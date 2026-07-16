//! `GET /api/shares/folder/{folder_id}/members` — return the signed
//! member list for an editable-folder share.

use std::collections::HashMap;
use std::str::FromStr;

use cryptfns::identity::KeyType;
use entity::{
    files, key_transitions,
    permission::{permission, SharePermission},
    user_files, users, ColumnTrait, EntityTrait, QueryFilter, Uuid,
};
use error::{AppResult, Error};

use crate::{
    data::folder_members::{FolderMember, FolderMembersResponse},
    data::key_transition::KeyTransitionRef,
    repository::Repository,
};

impl Repository<'_> {
    pub(crate) async fn folder_members(
        &self,
        caller: &users::Model,
        folder_id: Uuid,
    ) -> AppResult<FolderMembersResponse> {
        let perm = permission(&self.context.db, folder_id, caller.id).await?;
        if matches!(perm, SharePermission::None) {
            return Err(Error::NotFound("folder_not_found".to_string()));
        }

        let folder = files::Entity::find_by_id(folder_id)
            .one(&self.context.db)
            .await?
            .ok_or_else(|| Error::NotFound("folder_not_found".to_string()))?;
        if folder.mime != "dir" {
            return Err(Error::BadRequest("not_a_folder".to_string()));
        }

        let rows = user_files::Entity::find()
            .filter(user_files::Column::FileId.eq(folder_id))
            .all(&self.context.db)
            .await?;
        if rows.is_empty() {
            return Err(Error::NotFound("folder_not_found".to_string()));
        }

        let user_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
        let users_by_id: HashMap<Uuid, users::Model> = users::Entity::find()
            .filter(users::Column::Id.is_in(user_ids.clone()))
            .all(&self.context.db)
            .await?
            .into_iter()
            .map(|u| (u.id, u))
            .collect();

        let transitions_by_user: HashMap<Uuid, KeyTransitionRef> = key_transitions::Entity::find()
            .filter(key_transitions::Column::UserId.is_in(user_ids))
            .all(&self.context.db)
            .await?
            .iter()
            .filter_map(|row| KeyTransitionRef::from_row(row).map(|t| (row.user_id, t)))
            .collect();

        let owner_id = rows
            .iter()
            .find(|r| r.is_owner)
            .map(|r| r.user_id)
            .ok_or_else(|| Error::InternalError("folder_has_no_owner_row".to_string()))?;
        let owner_fingerprint = users_by_id
            .get(&owner_id)
            .map(|u| u.fingerprint.clone())
            .unwrap_or_default();

        // Sharing a file shares the roster: every member sees every other
        // member's email + fingerprint. The previous owner-only opt-in
        // toggle was retired in favour of uniform exposure.
        let mut members: Vec<FolderMember> = rows
            .iter()
            .map(|row| {
                let user = users_by_id.get(&row.user_id);
                FolderMember {
                    user_id: row.user_id,
                    email: user.map(|u| u.email.clone()),
                    pubkey: user.map(|u| u.pubkey.clone()).unwrap_or_default(),
                    key_type: user.map(|u| u.key_type.clone()).unwrap_or_default(),
                    wrapping_pubkey: user.and_then(|u| u.wrapping_pubkey.clone()),
                    pubkey_fingerprint: user.map(|u| u.fingerprint.clone()).unwrap_or_default(),
                    share_role: row.share_role.clone(),
                    is_owner: row.is_owner,
                    // The client re-verifies the member σ against the timestamp it
                    // signed. That's `member_signed_at`; legacy rows written before
                    // the column existed fall back to `shared_at`.
                    added_at: row.member_signed_at.or(row.shared_at),
                    signed_by_user_id: row.shared_by_user_id,
                    member_signature: row
                        .member_signature
                        .as_deref()
                        .map(cryptfns::base64::encode),
                    key_transition: transitions_by_user.get(&row.user_id).cloned(),
                }
            })
            .collect();
        // Stable order so the client can compare against a previously
        // verified snapshot byte-for-byte: by user_id ascending.
        members.sort_by_key(|m| m.user_id);

        let members_list_signature = folder
            .members_list_signature
            .as_deref()
            .map(cryptfns::base64::encode);

        let signer_id = folder.members_list_signed_by_user_id.unwrap_or(owner_id);
        let signature_algorithm = signature_algorithm_for(
            users_by_id.get(&signer_id),
            transitions_by_user.get(&signer_id),
            folder.members_list_signed_at,
        );

        Ok(FolderMembersResponse {
            folder_id,
            folder_owner_id: owner_id,
            folder_owner_pubkey_fingerprint: owner_fingerprint,
            signature_algorithm,
            members,
            members_signed_at: folder.members_list_signed_at,
            members_list_signature,
            members_list_signed_by_user_id: folder.members_list_signed_by_user_id,
        })
    }
}

/// The algorithm `members_list_signature` was actually produced with — the key
/// the signer held when they signed, so a rotated account still reports the
/// truth the way `files.cipher` records the cipher a file was encrypted with. A
/// roster signed before the signer migrated is under their old key, so the
/// signer's *current* key type would misreport it.
fn signature_algorithm_for(
    signer: Option<&users::Model>,
    transition: Option<&KeyTransitionRef>,
    signed_at: Option<i64>,
) -> &'static str {
    let key_type = match (transition, signed_at) {
        (Some(t), Some(at)) if at < t.issued_at => KeyType::from_str(&t.old_key_type).ok(),
        _ => signer.and_then(|u| KeyType::from_str(&u.key_type).ok()),
    }
    .unwrap_or(KeyType::Rsa);
    match key_type {
        KeyType::Rsa => "rsa-pss-sha256",
        KeyType::Curve25519 => "ed25519",
    }
}
