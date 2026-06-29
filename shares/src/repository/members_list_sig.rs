//! Folder member-list signature: reconstruct the canonical
//! `FolderMemberListV1` from the live DB, verify the client's
//! signature against it, and stamp the file row inside the caller's
//! transaction.
//!
//! A separate module so every membership-mutating route (initial share,
//! Co-owner re-share, role change, revoke, group-add cascade) goes
//! through the same canonicalisation pass. The reconstruction reads
//! the post-mutation state directly from `user_files` so the server's
//! signing input cannot disagree with the bytes the client signed —
//! supplying a list that omits or rearranges members breaks verification
//! deterministically.

use std::collections::HashMap;

use cryptfns::asn1::{
    encode_folder_member_list_v1, FolderListMember, FolderMemberListV1, ShareRoleEnum,
    FOLDER_LIST_V1_PREFIX,
};
use entity::{
    user_files, users, ActiveValue, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Uuid,
};
use error::{AppResult, Error};

const REPLAY_WINDOW_SECONDS: i64 = 300;

/// Client-supplied snapshot accompanying every membership mutation on a
/// folder. The fields mirror `members_list_signature`,
/// `members_signed_at`, `members_list_signed_by_user_id` on the
/// `files` row.
#[derive(Debug, Clone)]
pub(crate) struct MembersListSig {
    pub signature_b64: String,
    pub signed_at: i64,
    pub signed_by_user_id: Uuid,
}

/// Verify that the supplied `members_list_signature` covers the
/// post-mutation member set the transaction is about to commit. Caller
/// passes the prospective member set (so reconstruction can run before
/// or during the mutation transaction); we sort, DER-encode, and
/// RSA-PSS-verify against the named signer's pubkey. Returns the
/// canonical DER bytes on success so the route can immediately stamp
/// the file row without re-encoding.
pub(crate) async fn verify_post_mutation_signature<C: ConnectionTrait>(
    tx: &C,
    folder_id: Uuid,
    folder_owner_id: Uuid,
    members_after: &[ProspectiveMember],
    sig: &MembersListSig,
    now: i64,
) -> AppResult<Vec<u8>> {
    if (now - sig.signed_at).abs() > REPLAY_WINDOW_SECONDS {
        return Err(Error::BadRequest(
            "members_list_signature_skew".to_string(),
        ));
    }

    let signer = users::Entity::find_by_id(sig.signed_by_user_id)
        .one(tx)
        .await?
        .ok_or_else(|| {
            Error::BadRequest("members_list_signer_not_found".to_string())
        })?;

    // Only the folder owner or one of the post-mutation Co-owners is
    // authorized to sign. We accept the owner regardless of role
    // (the owner row carries `is_owner=true` even with `share_role`
    // values left over from legacy data) and any Co-owner that will
    // be present after the mutation commits.
    let authorized = members_after.iter().any(|m| {
        m.user_id == sig.signed_by_user_id
            && (m.is_owner || matches!(m.share_role, ShareRoleEnum::CoOwner))
    });
    if !authorized {
        return Err(Error::BadRequest(
            "members_list_signer_not_authorized".to_string(),
        ));
    }

    let payload = FolderMemberListV1 {
        folder_id: folder_id.into_bytes(),
        folder_owner_id: folder_owner_id.into_bytes(),
        members: members_after.iter().map(ProspectiveMember::to_list).collect(),
        members_signed_at: sig.signed_at,
    };
    let der = encode_folder_member_list_v1(&payload)
        .map_err(|e| Error::CryptoError(Box::new(e)))?;

    let mut signing_input = Vec::with_capacity(FOLDER_LIST_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(FOLDER_LIST_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    cryptfns::rsa::public::verify_bytes(&signing_input, &sig.signature_b64, &signer.pubkey)
        .map_err(|_| Error::BadRequest("members_list_signature_invalid".to_string()))?;

    Ok(der)
}

/// Write the verified signature onto the folder's `files` row inside
/// the caller's transaction. Stored as raw bytes so reads round-trip
/// through `base64::encode` on the way out.
pub(crate) async fn store_signature<C: ConnectionTrait>(
    tx: &C,
    folder_id: Uuid,
    sig: &MembersListSig,
) -> AppResult<()> {
    let signature_bytes = cryptfns::base64::decode(&sig.signature_b64)
        .map_err(|_| Error::BadRequest("members_list_signature_invalid_base64".to_string()))?;
    entity::files::Entity::update(entity::files::ActiveModel {
        id: ActiveValue::Unchanged(folder_id),
        members_list_signature: ActiveValue::Set(Some(signature_bytes)),
        members_list_signed_at: ActiveValue::Set(Some(sig.signed_at)),
        members_list_signed_by_user_id: ActiveValue::Set(Some(sig.signed_by_user_id)),
        ..Default::default()
    })
    .exec(tx)
    .await?;
    Ok(())
}

/// Single prospective member of the post-mutation list. The canonical
/// fingerprint comes from the matched `users` row (hex column → 32
/// bytes); the per-member σ signer comes from `user_files`. Helper is
/// `pub(crate)` so the route-level reconstruction code can build it
/// either from already-loaded `user_files` rows or by walking the DB.
#[derive(Clone, Debug)]
pub(crate) struct ProspectiveMember {
    pub user_id: Uuid,
    pub pubkey_fingerprint: [u8; 32],
    pub share_role: ShareRoleEnum,
    pub is_owner: bool,
    pub signed_by_user_id: Uuid,
}

impl ProspectiveMember {
    fn to_list(&self) -> FolderListMember {
        FolderListMember {
            user_id: self.user_id.into_bytes(),
            pubkey_fingerprint: self.pubkey_fingerprint,
            share_role: self.share_role,
            is_owner: self.is_owner,
            signed_by_user_id: self.signed_by_user_id.into_bytes(),
        }
    }
}

/// Reconstruct the prospective list from the live `user_files` rows for
/// `folder_id`. Used after the mutation commits' in-memory effect is
/// already represented by the rows we read.
pub(crate) async fn prospective_from_db<C: ConnectionTrait>(
    tx: &C,
    folder_id: Uuid,
) -> AppResult<(Uuid, Vec<ProspectiveMember>)> {
    let rows = user_files::Entity::find()
        .filter(user_files::Column::FileId.eq(folder_id))
        .all(tx)
        .await?;
    if rows.is_empty() {
        return Err(Error::NotFound("folder_not_found".to_string()));
    }
    let owner_id = rows
        .iter()
        .find(|r| r.is_owner)
        .map(|r| r.user_id)
        .ok_or_else(|| Error::InternalError("folder_has_no_owner_row".to_string()))?;

    let user_ids: Vec<Uuid> = rows.iter().map(|r| r.user_id).collect();
    let users_by_id: HashMap<Uuid, users::Model> = users::Entity::find()
        .filter(users::Column::Id.is_in(user_ids))
        .all(tx)
        .await?
        .into_iter()
        .map(|u| (u.id, u))
        .collect();

    let mut members = Vec::with_capacity(rows.len());
    for row in &rows {
        let user = users_by_id
            .get(&row.user_id)
            .ok_or_else(|| Error::InternalError("member_user_missing".to_string()))?;
        let fingerprint = parse_fingerprint(&user.fingerprint)?;
        // The owner's per-member σ is self-attested by construction; we
        // record their own id as the `signed_by_user_id` so the
        // FolderListMember encoding is deterministic for the owner row.
        let signed_by = row.shared_by_user_id.unwrap_or(owner_id);
        members.push(ProspectiveMember {
            user_id: row.user_id,
            pubkey_fingerprint: fingerprint,
            share_role: parse_share_role(&row.share_role)?,
            is_owner: row.is_owner,
            signed_by_user_id: signed_by,
        });
    }
    Ok((owner_id, members))
}

fn parse_fingerprint(hex: &str) -> AppResult<[u8; 32]> {
    let bytes = cryptfns::hex::decode(hex)
        .map_err(|_| Error::InternalError("fingerprint_not_hex".to_string()))?;
    if bytes.len() != 32 {
        return Err(Error::InternalError("fingerprint_wrong_length".to_string()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn parse_share_role(role: &str) -> AppResult<ShareRoleEnum> {
    match role {
        "reader" => Ok(ShareRoleEnum::Reader),
        "editor" => Ok(ShareRoleEnum::Editor),
        "co-owner" => Ok(ShareRoleEnum::CoOwner),
        other => Err(Error::InternalError(format!("unknown_share_role:{other}"))),
    }
}

