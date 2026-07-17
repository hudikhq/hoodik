use entity::{share_groups, Uuid};
use serde::{Deserialize, Serialize};
use validr::*;

/// Server-side view of one row in `share_groups`. Returned by every group
/// endpoint; the `name` field is bounded to 255 chars at create time so
/// the column never has to be re-validated downstream.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShareGroup {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub created_at: i64,
}

impl From<share_groups::Model> for AppShareGroup {
    fn from(m: share_groups::Model) -> Self {
        Self {
            id: m.id,
            owner_id: m.owner_id,
            name: m.name,
            created_at: m.created_at,
        }
    }
}

/// Body for `POST /api/shares/groups`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateGroupBody {
    pub name: Option<String>,
}

impl Validation for CreateGroupBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(name),
            Rule::new("name", |obj: &CreateGroupBody, error| {
                if let Some(name) = obj.name.as_deref() {
                    let trimmed = name.trim();
                    if trimmed.is_empty() {
                        error.add("required");
                        return;
                    }
                    if trimmed.len() > 255 {
                        error.add("max:255");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![Modifier::new("name", |obj: &mut CreateGroupBody| {
            if let Some(name) = obj.name.as_mut() {
                *name = name.trim().to_string();
            }
        })]
    }
}

/// Body for `POST /api/shares/groups/{id}/members`. Adding a member is a
/// plain roster insert — no file keys move, so there is no crypto payload.
/// The replay nonce + timestamp guard the single remaining group write.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AddGroupMemberBody {
    pub user_id: Option<String>,
    pub pubkey_fingerprint: Option<String>,
    /// The new member's role *in the group* (reader/editor/co-owner).
    pub group_role: Option<String>,
    pub timestamp: Option<i64>,
    /// 16-byte replay nonce, base64-encoded. Bound per `(caller, nonce)`
    /// against the in-memory dedup cache alongside the timestamp window.
    pub nonce: Option<String>,
}

impl Validation for AddGroupMemberBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(user_id),
            rule_required!(pubkey_fingerprint),
            rule_required!(group_role),
            rule_required!(timestamp),
            rule_required!(nonce),
            Rule::new("group_role", |obj: &AddGroupMemberBody, error| {
                match obj.group_role.as_deref() {
                    Some("reader") | Some("editor") | Some("co-owner") => {}
                    Some(_) => error.add("invalid_group_role"),
                    None => {}
                }
            }),
        ]
    }
}

/// Decode a base64 16-byte replay nonce carried by a group write body.
pub(crate) fn decode_nonce(raw: &str) -> Result<[u8; 16], ::error::Error> {
    let bytes = cryptfns::base64::decode(raw)
        .map_err(|_| ::error::Error::BadRequest("nonce_invalid_base64".to_string()))?;
    let arr: [u8; 16] = bytes
        .try_into()
        .map_err(|_| ::error::Error::BadRequest("nonce_wrong_length".to_string()))?;
    Ok(arr)
}

/// Body for `PUT /api/shares/groups/{id}/members/{user_id}/role`. Pure
/// roster metadata — changing a member's *group* role moves no file key,
/// so there is no crypto payload. Authorized by `can_manage_group` plus
/// the privilege-escalation guard in `GroupRole::can_set_role`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SetMemberRoleBody {
    pub group_role: Option<String>,
}

impl Validation for SetMemberRoleBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(group_role),
            Rule::new("group_role", |obj: &SetMemberRoleBody, error| {
                match obj.group_role.as_deref() {
                    Some("reader") | Some("editor") | Some("co-owner") => {}
                    Some(_) => error.add("invalid_group_role"),
                    None => {}
                }
            }),
        ]
    }
}

/// Body for `PATCH /api/shares/groups/{id}`. Renames a group; the
/// `(owner_id, name)` unique index maps a clash to 409.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RenameGroupBody {
    pub name: Option<String>,
}

impl Validation for RenameGroupBody {
    fn rules(&self) -> Vec<Rule<Self>> {
        vec![
            rule_required!(name),
            Rule::new("name", |obj: &RenameGroupBody, error| {
                if let Some(name) = obj.name.as_deref() {
                    let trimmed = name.trim();
                    if trimmed.is_empty() {
                        error.add("required");
                        return;
                    }
                    if trimmed.len() > 255 {
                        error.add("max:255");
                    }
                }
            }),
        ]
    }

    fn modifiers(&self) -> Vec<Modifier<Self>> {
        vec![Modifier::new("name", |obj: &mut RenameGroupBody| {
            if let Some(name) = obj.name.as_mut() {
                *name = name.trim().to_string();
            }
        })]
    }
}

/// `GET /api/shares/groups` response. Owned groups carry the caller's
/// own membership list (so the UI can render rosters without a second
/// round-trip); member-of groups carry only the group identity plus
/// the owner's email — the caller has no business listing peers in a
/// group they don't own.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShareGroupWithMembers {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub created_at: i64,
    pub members: Vec<AppShareGroupMember>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShareGroupMember {
    pub user_id: Uuid,
    pub email: String,
    pub fingerprint: String,
    pub added_at: i64,
    /// The member's role *in the group* (reader/editor/co-owner). Distinct
    /// from any file-level share role those words also name.
    pub group_role: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppShareGroupAsMember {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub owner_email: String,
    pub name: String,
    pub created_at: i64,
    pub added_at: i64,
    /// The caller's own role in this group — drives which actions the UI
    /// offers (share-to-group for editors, manage for co-owners).
    pub group_role: String,
}

/// Composite payload returned by `GET /api/shares/groups`. Splitting
/// owner-of and member-of lets the UI render two columns without a
/// client-side regroup.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupsResponse {
    pub owned: Vec<AppShareGroupWithMembers>,
    pub member_of: Vec<AppShareGroupAsMember>,
}

/// One recipient in the roster returned by
/// `GET /api/shares/groups/{id}/members`: the group owner and every
/// member, each with the pubkey material the client needs to wrap a file
/// key. The client fans a share out to this whole set.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GroupMemberWithKey {
    pub user_id: Uuid,
    pub email: String,
    pub pubkey: String,
    pub key_type: String,
    pub wrapping_pubkey: Option<String>,
    pub fingerprint: String,
    pub group_role: String,
}
