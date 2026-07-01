//! Deterministic inputs for the cross-runtime ASN.1 fixture suite. The same
//! constants are committed in their DER-encoded form alongside this file
//! (`share_request_v1.der`, `member_sig_v1.der`, `audit_event_v1.der`) so
//! the native, WASM, and FFI encoders can each be asserted byte-identical
//! against the committed bytes.

#![allow(dead_code)]

use cryptfns::asn1::{
    AuditEventActionEnum, AuditEventRowV1, AuditEventSigInputV1, FolderListMember,
    FolderMemberListV1, MemberSigPayloadV1, ShareEntry, ShareRequestPayloadV1, ShareRoleEnum,
};

pub const SHARE_REQUEST_DER: &[u8] = include_bytes!("share_request_v1.der");
pub const MEMBER_SIG_DER: &[u8] = include_bytes!("member_sig_v1.der");
pub const AUDIT_EVENT_DER: &[u8] = include_bytes!("audit_event_v1.der");
pub const AUDIT_EVENT_SIG_INPUT_DER: &[u8] = include_bytes!("audit_event_sig_input_v1.der");
pub const FOLDER_MEMBER_LIST_DER: &[u8] = include_bytes!("folder_member_list_v1.der");

pub const SHARE_REQUEST_SENDER_ID: [u8; 16] = [0x11; 16];
pub const SHARE_REQUEST_RECIPIENT_ID: [u8; 16] = [0x22; 16];
pub const SHARE_REQUEST_RECIPIENT_FINGERPRINT: [u8; 32] = [0x33; 32];
pub const SHARE_REQUEST_ROOT_FILE_ID: [u8; 16] = [0x44; 16];
pub const SHARE_REQUEST_ENTRIES_HASH: [u8; 32] = [0x55; 32];
pub const SHARE_REQUEST_TIMESTAMP: i64 = 1_735_689_600;
pub const SHARE_REQUEST_NONCE: [u8; 16] = [0x66; 16];

pub const MEMBER_SIG_USER_ID: [u8; 16] = [0x77; 16];
pub const MEMBER_SIG_FINGERPRINT: [u8; 32] = [0x88; 32];
pub const MEMBER_SIG_PUBKEY_BYTE: u8 = 0xAA;
pub const MEMBER_SIG_PUBKEY_LEN: usize = 270;
pub const MEMBER_SIG_SIGNED_AT: i64 = 1_735_689_700;

pub const AUDIT_EVENT_SENDER_ID: [u8; 16] = [0xAA; 16];
pub const AUDIT_EVENT_RECIPIENT_ID: [u8; 16] = [0xBB; 16];
pub const AUDIT_EVENT_FILE_ID: [u8; 16] = [0xCC; 16];
pub const AUDIT_EVENT_ACTION: &str = "grant";
pub const AUDIT_EVENT_CREATED_AT: i64 = 1_735_689_800;

pub const AUDIT_EVENT_SIG_SENDER_ID: [u8; 16] = [0xAA; 16];
pub const AUDIT_EVENT_SIG_RECIPIENT_ID: [u8; 16] = [0xBB; 16];
pub const AUDIT_EVENT_SIG_FILE_ID: [u8; 16] = [0xCC; 16];
pub const AUDIT_EVENT_SIG_TIMESTAMP: i64 = 1_735_689_900;

pub const SHARE_ENTRY_FILE_ID_A: [u8; 16] = [0xDD; 16];
pub const SHARE_ENTRY_FILE_ID_B: [u8; 16] = [0x99; 16];
pub const SHARE_ENTRY_KEY_BYTE_A: u8 = 0x11;
pub const SHARE_ENTRY_KEY_BYTE_B: u8 = 0x22;
pub const SHARE_ENTRY_KEY_LEN: usize = 256;

pub const FOLDER_MEMBER_LIST_FOLDER_ID: [u8; 16] = [0xF0; 16];
pub const FOLDER_MEMBER_LIST_OWNER_ID: [u8; 16] = [0x11; 16];
pub const FOLDER_MEMBER_LIST_SIGNED_AT: i64 = 1_736_000_000;

pub fn share_request_v1() -> ShareRequestPayloadV1 {
    ShareRequestPayloadV1 {
        sender_id: SHARE_REQUEST_SENDER_ID,
        recipient_id: SHARE_REQUEST_RECIPIENT_ID,
        recipient_pubkey_fingerprint: SHARE_REQUEST_RECIPIENT_FINGERPRINT,
        share_role: ShareRoleEnum::Editor,
        root_file_id: SHARE_REQUEST_ROOT_FILE_ID,
        entries_hash: SHARE_REQUEST_ENTRIES_HASH,
        timestamp: SHARE_REQUEST_TIMESTAMP,
        nonce: SHARE_REQUEST_NONCE,
    }
}

pub fn member_sig_v1() -> MemberSigPayloadV1 {
    MemberSigPayloadV1 {
        user_id: MEMBER_SIG_USER_ID,
        pubkey_der: vec![MEMBER_SIG_PUBKEY_BYTE; MEMBER_SIG_PUBKEY_LEN],
        fingerprint: MEMBER_SIG_FINGERPRINT,
        share_role: ShareRoleEnum::CoOwner,
        signed_at: MEMBER_SIG_SIGNED_AT,
    }
}

pub fn audit_event_v1() -> AuditEventRowV1 {
    AuditEventRowV1 {
        sender_id: AUDIT_EVENT_SENDER_ID,
        recipient_id: AUDIT_EVENT_RECIPIENT_ID,
        file_id: AUDIT_EVENT_FILE_ID,
        action: AUDIT_EVENT_ACTION.to_string(),
        share_role: Some(ShareRoleEnum::Reader),
        created_at: AUDIT_EVENT_CREATED_AT,
    }
}

pub fn audit_event_sig_input_v1() -> AuditEventSigInputV1 {
    AuditEventSigInputV1 {
        sender_id: AUDIT_EVENT_SIG_SENDER_ID,
        recipient_id: Some(AUDIT_EVENT_SIG_RECIPIENT_ID),
        file_id: AUDIT_EVENT_SIG_FILE_ID,
        action: AuditEventActionEnum::RoleChange,
        share_role_before: Some(ShareRoleEnum::Reader),
        share_role_after: Some(ShareRoleEnum::Editor),
        timestamp: AUDIT_EVENT_SIG_TIMESTAMP,
    }
}

pub fn folder_member_list_v1() -> FolderMemberListV1 {
    FolderMemberListV1 {
        folder_id: FOLDER_MEMBER_LIST_FOLDER_ID,
        folder_owner_id: FOLDER_MEMBER_LIST_OWNER_ID,
        members: vec![
            FolderListMember {
                user_id: [0x11; 16],
                pubkey_fingerprint: [0xA1; 32],
                share_role: ShareRoleEnum::Reader,
                is_owner: true,
                signed_by_user_id: [0x11; 16],
            },
            FolderListMember {
                user_id: [0x22; 16],
                pubkey_fingerprint: [0xB2; 32],
                share_role: ShareRoleEnum::CoOwner,
                is_owner: false,
                signed_by_user_id: [0x11; 16],
            },
            FolderListMember {
                user_id: [0x33; 16],
                pubkey_fingerprint: [0xC3; 32],
                share_role: ShareRoleEnum::Editor,
                is_owner: false,
                signed_by_user_id: [0x22; 16],
            },
        ],
        members_signed_at: FOLDER_MEMBER_LIST_SIGNED_AT,
    }
}

pub fn share_entries_v1() -> Vec<ShareEntry> {
    vec![
        ShareEntry {
            file_id: SHARE_ENTRY_FILE_ID_A,
            encrypted_key: vec![SHARE_ENTRY_KEY_BYTE_A; SHARE_ENTRY_KEY_LEN],
        },
        ShareEntry {
            file_id: SHARE_ENTRY_FILE_ID_B,
            encrypted_key: vec![SHARE_ENTRY_KEY_BYTE_B; SHARE_ENTRY_KEY_LEN],
        },
    ]
}
