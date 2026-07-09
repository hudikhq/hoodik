//! Audit-log integrity tests. Each test exercises one property of the
//! per-sender hash chain or the per-row sender signature in
//! `share_events`.

#[macro_use]
#[path = "./shares_common.rs"]
mod shares_common;

use actix_web::{http::StatusCode, test};
use cryptfns::asn1::{
    encode_audit_event_sig_input_v1, AuditEventActionEnum, AuditEventSigInputV1, ShareRoleEnum,
    AUDIT_EVENT_SIG_V1_PREFIX,
};
use entity::{
    share_events, ActiveValue, ColumnTrait, EntityTrait, QueryFilter,
};
use hoodik::server;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::shares_common::*;

#[actix_web::test]
async fn test_audit_row_inserted_on_grant() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "audit-grant");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let rows = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(alice.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.sender_id, Some(alice.user_id));
    assert_eq!(row.recipient_id, Some(bob.user_id));
    assert_eq!(row.file_id, file.id);
    assert_eq!(row.action, "grant");
    assert_eq!(row.share_role_before, None);
    assert_eq!(row.share_role_after.as_deref(), Some("reader"));
    assert!(
        row.sender_signature.as_ref().map(|v| !v.is_empty()).unwrap_or(false),
        "grant rows must carry a non-empty sender signature"
    );
    assert_eq!(row.this_event_hash.len(), 32);
}

#[actix_web::test]
async fn test_audit_row_sender_signature_verifies() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "audit-sig-verify");
    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let row = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(alice.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("grant row exists");

    let signature_b64 = cryptfns::base64::encode(row.sender_signature.as_ref().unwrap());
    let input = AuditEventSigInputV1 {
        sender_id: alice.user_id.into_bytes(),
        recipient_id: Some(bob.user_id.into_bytes()),
        file_id: file.id.into_bytes(),
        action: AuditEventActionEnum::Grant,
        share_role_before: None,
        share_role_after: Some(ShareRoleEnum::Editor),
        timestamp: row.created_at,
    };
    let der = encode_audit_event_sig_input_v1(&input).unwrap();
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    cryptfns::rsa::public::verify_bytes(&signing_input, &signature_b64, &alice.public_pem)
        .expect("recomputed signature must verify against alice's pubkey");
}

#[actix_web::test]
async fn test_event_signature_mismatch_rejected_400() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "audit-sig-mismatch");

    let envelope = build_share_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    // Corrupt the audit signature without touching the rest of the envelope.
    let mut tampered = envelope.clone();
    let sig_b64 = tampered["event_signature"].as_str().unwrap().to_string();
    let mut sig_bytes = cryptfns::base64::decode(&sig_b64).unwrap();
    let last_idx = sig_bytes.len() - 1;
    sig_bytes[last_idx] ^= 0x01;
    tampered["event_signature"] =
        Value::String(cryptfns::base64::encode(&sig_bytes));

    let resp = post_share!(app, alice, tampered);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "event_signature_invalid");

    let rows = share_events::Entity::find()
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        rows.is_empty(),
        "rolled-back transaction must leave share_events untouched"
    );
    let touched = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::UserId.eq(bob.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert!(touched.is_empty(), "rolled-back transaction must leave user_files untouched");
}

#[actix_web::test]
async fn test_per_sender_chain_continuous_across_multiple_events() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    register_user!(app, context, dave, "dave@example.com");
    let file_a = create_file!(app, alice, "chain-a");
    let file_b = create_file!(app, alice, "chain-b");
    let file_c = create_file!(app, alice, "chain-c");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file_a.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file_b.id);
    grant!(app, alice, dave, ShareRoleEnum::Reader, file_c.id);

    let rows = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(alice.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 3);

    // Walk the chain by following prev_event_hash — created_at ties give an
    // ambiguous read order, but the chain itself is a linked list with one
    // head (prev_event_hash IS NULL) and unique back-pointers.
    let by_hash: std::collections::HashMap<Vec<u8>, &share_events::Model> =
        rows.iter().map(|r| (r.this_event_hash.clone(), r)).collect();
    let head = rows
        .iter()
        .find(|r| r.prev_event_hash.is_none())
        .expect("exactly one chain head");
    let mut walked = vec![head];
    let mut next_prev = head.this_event_hash.clone();
    while let Some(row) = rows
        .iter()
        .find(|r| r.prev_event_hash.as_ref() == Some(&next_prev))
    {
        walked.push(row);
        next_prev = row.this_event_hash.clone();
    }
    assert_eq!(walked.len(), rows.len(), "chain walk must cover every row");
    assert_eq!(by_hash.len(), rows.len(), "this_event_hash values must be unique");

    // Walking the chain forward and recomputing each row's hash must match.
    let mut prev: Vec<u8> = vec![0u8; 32];
    for row in &walked {
        let row_input = cryptfns::asn1::AuditEventRowV1 {
            sender_id: row.sender_id.unwrap_or(entity::Uuid::nil()).into_bytes(),
            recipient_id: row.recipient_id.unwrap_or(entity::Uuid::nil()).into_bytes(),
            file_id: row.file_id.into_bytes(),
            action: row.action.clone(),
            share_role: row
                .share_role_after
                .as_deref()
                .and_then(|s| match s {
                    "reader" => Some(ShareRoleEnum::Reader),
                    "editor" => Some(ShareRoleEnum::Editor),
                    "co-owner" => Some(ShareRoleEnum::CoOwner),
                    _ => None,
                }),
            created_at: row.created_at,
        };
        let der = cryptfns::asn1::encode_audit_event_v1(&row_input).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(cryptfns::asn1::AUDIT_EVENT_V1_PREFIX);
        hasher.update(&prev);
        hasher.update(&der);
        let computed = hasher.finalize();
        assert_eq!(
            computed.as_slice(),
            row.this_event_hash.as_slice(),
            "this_event_hash must equal recomputed chain hash"
        );
        prev = row.this_event_hash.clone();
    }
}

#[actix_web::test]
async fn test_chain_break_detectable_by_walking() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file_a = create_file!(app, alice, "break-a");
    let file_b = create_file!(app, alice, "break-b");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file_a.id);
    grant!(app, alice, carol, ShareRoleEnum::Reader, file_b.id);

    // Find the chain head (the row with prev_event_hash NULL) and tamper
    // with its this_event_hash directly in the database.
    let rows = share_events::Entity::find()
        .filter(share_events::Column::SenderId.eq(alice.user_id))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let head = rows
        .iter()
        .find(|r| r.prev_event_hash.is_none())
        .expect("chain head exists")
        .clone();
    let downstream = rows
        .iter()
        .find(|r| r.id != head.id)
        .expect("second row exists")
        .clone();
    assert_eq!(
        downstream.prev_event_hash.as_ref().unwrap(),
        &head.this_event_hash,
        "downstream must already chain off the head before tampering"
    );

    let mut bad_hash = head.this_event_hash.clone();
    bad_hash[0] ^= 0x01;
    let mut updated: share_events::ActiveModel = head.clone().into();
    updated.this_event_hash = ActiveValue::Set(bad_hash.clone());
    share_events::Entity::update(updated)
        .exec(&context.db)
        .await
        .unwrap();

    // After tampering, the downstream row's prev_event_hash diverges from
    // the head's stored this_event_hash — a chain walker surfaces this.
    let head_after = share_events::Entity::find_by_id(head.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    let downstream_after = share_events::Entity::find_by_id(downstream.id)
        .one(&context.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(head_after.this_event_hash, bad_hash);
    assert_ne!(
        downstream_after.prev_event_hash.as_ref().unwrap(),
        &head_after.this_event_hash,
        "chain walker must see the divergence"
    );
}

/// Role-change audit rows carry a `sender_signature` that the SPA's
/// verifier can recompute against the granter's pubkey using the
/// `role_change` canonical input (action + share_role_before + share_role_after
/// + timestamp). The bug this pins: previously the server discarded the
/// client-supplied signature whenever the entry was a role_change rather
/// than a fresh grant, leaving a NULL `sender_signature` on every legitimate
/// upgrade/downgrade — a forensic gap on the most-sensitive audit action
/// in the chain.
#[actix_web::test]
async fn test_role_change_persists_sender_signature_and_verifies() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "role-change-audit");

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);

    let timestamp = now_secs();
    let envelope = build_role_change_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Editor,
        ShareRoleEnum::CoOwner,
        file.id,
        vec![(file.id, b"wrapped-coowner".to_vec())],
        random_nonce(),
        timestamp,
    );
    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let role_change_rows = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("role_change"))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(
        role_change_rows.len(),
        1,
        "exactly one role_change row written"
    );
    let row = &role_change_rows[0];
    assert_eq!(row.sender_id, Some(alice.user_id));
    assert_eq!(row.recipient_id, Some(bob.user_id));
    assert_eq!(row.share_role_before.as_deref(), Some("editor"));
    assert_eq!(row.share_role_after.as_deref(), Some("co-owner"));
    assert_eq!(row.created_at, timestamp);

    let sig_bytes = row
        .sender_signature
        .as_ref()
        .expect("role_change row must carry a sender signature");
    assert!(!sig_bytes.is_empty(), "signature must not be empty bytes");

    let sig_b64 = cryptfns::base64::encode(sig_bytes);
    let input = AuditEventSigInputV1 {
        sender_id: alice.user_id.into_bytes(),
        recipient_id: Some(bob.user_id.into_bytes()),
        file_id: file.id.into_bytes(),
        action: AuditEventActionEnum::RoleChange,
        share_role_before: Some(ShareRoleEnum::Editor),
        share_role_after: Some(ShareRoleEnum::CoOwner),
        timestamp: row.created_at,
    };
    let der = encode_audit_event_sig_input_v1(&input).unwrap();
    let mut signing_input = Vec::with_capacity(AUDIT_EVENT_SIG_V1_PREFIX.len() + der.len());
    signing_input.extend_from_slice(AUDIT_EVENT_SIG_V1_PREFIX);
    signing_input.extend_from_slice(&der);
    cryptfns::rsa::public::verify_bytes(&signing_input, &sig_b64, &alice.public_pem)
        .expect("role_change signature must verify against the granter's pubkey");
}

/// A submission that claims to be a role_change but whose `event_signature`
/// covers a different action (e.g. the legacy `grant` canonical, mismatched
/// roles, or another signer entirely) must be rejected before any row is
/// written. Mirrors the existing grant-path tamper test so the role_change
/// path has the same forensic floor.
#[actix_web::test]
async fn test_role_change_with_wrong_signature_rejected_400() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "role-change-tamper");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    let timestamp = now_secs();
    // A correctly-formed envelope for the upgrade, then we swap the
    // audit signature for one over the legacy `grant` canonical — what
    // an out-of-date client would emit. The verifier must reject it.
    let mut envelope = build_role_change_envelope(
        &alice,
        &bob,
        ShareRoleEnum::Reader,
        ShareRoleEnum::Editor,
        file.id,
        vec![(file.id, b"wrapped-editor".to_vec())],
        random_nonce(),
        timestamp,
    );
    let wrong_signature = sign_audit_event(
        &alice,
        &bob,
        file.id,
        AuditEventActionEnum::Grant,
        None,
        Some(ShareRoleEnum::Editor),
        timestamp,
    );
    envelope["event_signature"] = Value::String(wrong_signature);

    let resp = post_share!(app, alice, envelope);
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["message"], "event_signature_invalid");

    // The recipient's existing row is untouched and no role_change audit
    // row was appended.
    let recipient_row = entity::user_files::Entity::find()
        .filter(entity::user_files::Column::FileId.eq(file.id))
        .filter(entity::user_files::Column::UserId.eq(bob.user_id))
        .one(&context.db)
        .await
        .unwrap()
        .expect("bob still has the reader row");
    assert_eq!(recipient_row.share_role, "reader");
    let role_change_rows = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("role_change"))
        .all(&context.db)
        .await
        .unwrap();
    assert!(
        role_change_rows.is_empty(),
        "rejected role_change must not append an audit row"
    );
}

#[actix_web::test]
async fn test_cascade_revoke_audit_rows_have_null_sender_signature() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "cascade-audit");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);

    let revoke = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::CoOwner, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let cascade_rows = share_events::Entity::find()
        .filter(share_events::Column::Action.eq("shared_by_co_owner_revoked"))
        .all(&context.db)
        .await
        .unwrap();
    assert_eq!(cascade_rows.len(), 1);
    let cascade = &cascade_rows[0];
    assert!(cascade.sender_id.is_none(), "cascade rows have NULL sender_id");
    assert!(
        cascade.sender_signature.is_none(),
        "cascade rows have NULL sender_signature"
    );
    assert_eq!(cascade.recipient_id, Some(carol.user_id));
}

/// Client-style chain walk over the `GET /api/shares/events` response. The
/// JS verifier in `web/services/shares/crypto.ts::verifyChain` buckets rows
/// by `sender_id` (including the NULL bucket for system-cascade rows),
/// orders each bucket oldest-first, and recomputes
/// `sha256(prefix || prev_event_hash || encode_audit_event_v1(row))` against
/// the row's stored `this_event_hash`. The shape of the response and the
/// encoder inputs must match the client's expectations exactly — a drift
/// here surfaces as a "chain mismatch" badge in the audit UI even though
/// neither the database nor any user has been tampered with. This test
/// drives a real grant + co-owner reshare + cascade revoke, then walks the
/// events the way the client does and asserts every row's hash matches.
#[actix_web::test]
async fn test_chain_walk_over_events_route_matches_client_verifier() {
    use std::collections::BTreeMap;

    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    register_user!(app, context, carol, "carol@example.com");
    let file = create_file!(app, alice, "chain-walk-events");

    grant!(app, alice, bob, ShareRoleEnum::CoOwner, file.id);
    let envelope = build_co_owner_share_envelope(
        &bob,
        &carol,
        ShareRoleEnum::Reader,
        file.id,
        vec![(file.id, b"wrapped".to_vec())],
        random_nonce(),
        now_secs(),
    );
    let resp = post_share!(app, bob, envelope);
    assert_eq!(resp.status(), StatusCode::CREATED);
    let revoke = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::CoOwner, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Fetch via the public events endpoint — same surface the SPA reads.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let events = body["events"].as_array().expect("events array").clone();
    assert!(events.len() >= 3, "alice sees grant + reshare + cascade rows");

    // Bucket by sender_id (string or "__system__" for NULL) and walk
    // oldest-first inside each bucket, mirroring verifyChain().
    let mut buckets: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for row in &events {
        let key = row["sender_id"]
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "__system__".to_string());
        buckets.entry(key).or_default().push(row);
    }
    for rows in buckets.values_mut() {
        rows.sort_by(|a, b| {
            let ac = a["created_at"].as_i64().unwrap();
            let bc = b["created_at"].as_i64().unwrap();
            if ac == bc {
                a["id"].as_str().unwrap().cmp(b["id"].as_str().unwrap())
            } else {
                ac.cmp(&bc)
            }
        });
    }

    fn role_enum(value: &Value) -> Option<ShareRoleEnum> {
        match value.as_str() {
            Some("reader") => Some(ShareRoleEnum::Reader),
            Some("editor") => Some(ShareRoleEnum::Editor),
            Some("co-owner") => Some(ShareRoleEnum::CoOwner),
            _ => None,
        }
    }
    fn uuid_bytes(value: &Value) -> [u8; 16] {
        match value.as_str().and_then(|s| entity::Uuid::parse_str(s).ok()) {
            Some(uuid) => uuid.into_bytes(),
            None => [0u8; 16],
        }
    }

    let mut total_walked = 0usize;
    for rows in buckets.values() {
        let mut prev_hash: Option<Vec<u8>> = None;
        for row in rows {
            let row_input = cryptfns::asn1::AuditEventRowV1 {
                sender_id: uuid_bytes(&row["sender_id"]),
                recipient_id: uuid_bytes(&row["recipient_id"]),
                file_id: uuid_bytes(&row["file_id"]),
                action: row["action"].as_str().unwrap().to_string(),
                share_role: role_enum(&row["share_role_after"]),
                created_at: row["created_at"].as_i64().unwrap(),
            };
            let der = cryptfns::asn1::encode_audit_event_v1(&row_input).unwrap();
            let recompute_against = match prev_hash.as_deref() {
                Some(value) => value.to_vec(),
                None => match row["prev_event_hash"].as_str() {
                    Some(prev_b64) => cryptfns::base64::decode(prev_b64).unwrap(),
                    None => vec![0u8; 32],
                },
            };
            let mut hasher = Sha256::new();
            hasher.update(cryptfns::asn1::AUDIT_EVENT_V1_PREFIX);
            hasher.update(&recompute_against);
            hasher.update(&der);
            let computed = hasher.finalize().to_vec();
            let stored = cryptfns::base64::decode(row["this_event_hash"].as_str().unwrap())
                .unwrap();
            assert_eq!(
                computed, stored,
                "row {} (action={}) failed client-style chain verification",
                row["id"], row["action"]
            );
            prev_hash = Some(stored);
            total_walked += 1;
        }
    }
    assert_eq!(total_walked, events.len(), "every row must be walked");
}

/// Every event row carries enough material for the caller to decrypt the
/// file's name client-side, when the caller still has access. Alice (owner)
/// and Bob (recipient) both see `encrypted_name` + `encrypted_key` non-null
/// on the grant row — the name is held by `files`, the wrap is held by
/// `user_files`, both join on `file_id`. The bare-id fallback in
/// `rowSentence` is reserved for the cases the next two tests cover.
#[actix_web::test]
async fn test_events_carry_encrypted_name_and_key_for_authorized_caller() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "events-name-grant");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);

    for user in [&alice, &bob] {
        let req = test::TestRequest::get()
            .uri("/api/shares/events?limit=100&offset=0")
            .cookie(user.jwt.clone())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body: Value = test::read_body_json(resp).await;
        let events = body["events"].as_array().expect("events array");
        let row = events
            .iter()
            .find(|r| r["file_id"].as_str() == Some(&file.id.to_string()))
            .unwrap_or_else(|| panic!("{} sees a row for the granted file", user.email));
        assert_eq!(
            row["encrypted_name"].as_str(),
            Some("encrypted-name"),
            "{} sees the file's encrypted_name",
            user.email,
        );
        assert!(
            row["encrypted_key"].as_str().is_some_and(|s| !s.is_empty()),
            "{} sees their own wrapped file key",
            user.email,
        );
        assert!(
            row["cipher"].as_str().is_some_and(|s| !s.is_empty()),
            "{} sees the file's cipher so decrypt picks the right algorithm",
            user.email,
        );
    }
}

/// A revoked recipient still sees the audit rows that mention them — but
/// their `user_files` row is gone, so `encrypted_key` LEFT-JOINs to null.
/// `encrypted_name` is still surfaced because the file row itself survives.
/// The owner's view of the same rows keeps both fields non-null (she still
/// owns the file). Client renders the bare-id fallback whenever
/// `encrypted_key` is null — no second round-trip, no per-row fetch.
#[actix_web::test]
async fn test_revoked_recipient_loses_encrypted_key_but_keeps_encrypted_name() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "events-name-revoke");

    grant!(app, alice, bob, ShareRoleEnum::Editor, file.id);
    let revoke = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::Editor, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Owner view: she still owns the file, so both grant and revoke rows
    // carry the encrypted name and her own wrapped key.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(alice.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let alice_rows = body["events"].as_array().expect("alice events").clone();
    assert!(alice_rows.len() >= 2, "alice sees grant + revoke");
    for row in &alice_rows {
        assert_eq!(row["encrypted_name"].as_str(), Some("encrypted-name"));
        assert!(row["encrypted_key"].as_str().is_some_and(|s| !s.is_empty()));
    }

    // Recipient view: bob's user_files row is gone after the revoke. The
    // revoke row is still visible (he's the recipient_id), but his wrapped
    // key isn't joinable anymore — the LEFT JOIN returns null.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let bob_rows = body["events"].as_array().expect("bob events");
    let revoke_row = bob_rows
        .iter()
        .find(|r| r["action"].as_str() == Some("revoke"))
        .expect("bob sees the revoke targeting him");
    assert_eq!(
        revoke_row["encrypted_name"].as_str(),
        Some("encrypted-name"),
        "file row is intact, so encrypted_name survives",
    );
    assert!(
        revoke_row["encrypted_key"].is_null(),
        "bob lost access, so his wrapped key LEFT-JOINs to null",
    );
    assert!(
        revoke_row["cipher"].as_str().is_some_and(|s| !s.is_empty()),
        "cipher is carried by the file row, not by user_files",
    );
}

/// A bystander who can see the audit row but has no `user_files` row of
/// their own for the file gets `encrypted_key = null` on every action.
/// Carol owns the file, Alice grants Bob a co-owner role on Carol's behalf
/// (cascade), then Carol revokes Bob. Bob's audit view
/// surfaces the revoke (he was the recipient) but his wrap is gone — the
/// LEFT JOIN nulls `encrypted_key` while `encrypted_name` survives. The
/// shape mirrors `test_revoked_recipient_loses_encrypted_key_but_keeps_
/// encrypted_name`; this one pins the bystander angle so an accidental
/// `INNER JOIN user_files` regression on the query path surfaces here.
#[actix_web::test]
async fn test_bystander_without_user_files_row_sees_null_encrypted_key() {
    let context = context::Context::mock_sqlite().await;
    let app = test::init_service(server::app(context.clone())).await;

    register_user!(app, context, alice, "alice@example.com");
    register_user!(app, context, bob, "bob@example.com");
    let file = create_file!(app, alice, "events-name-bystander");

    grant!(app, alice, bob, ShareRoleEnum::Reader, file.id);
    let revoke = build_revoke_body(&alice, &bob, file.id, ShareRoleEnum::Reader, now_secs());
    let resp = delete_share!(app, alice, file.id, bob.user_id, revoke);
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Bob's grant row + revoke row both reference a file he no longer
    // has a user_files row for. The visibility filter still surfaces
    // them (he's the recipient_id), but his wrap is gone — the bare-id
    // fallback fires on every row this view would render.
    let req = test::TestRequest::get()
        .uri("/api/shares/events?limit=100&offset=0")
        .cookie(bob.jwt.clone())
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body: Value = test::read_body_json(resp).await;
    let bob_rows = body["events"].as_array().expect("bob events");
    assert!(!bob_rows.is_empty(), "bob still sees rows after revoke");
    for row in bob_rows {
        assert_eq!(
            row["encrypted_name"].as_str(),
            Some("encrypted-name"),
            "file is intact, so encrypted_name survives for {}",
            row["action"],
        );
        assert!(
            row["encrypted_key"].is_null(),
            "bob has no user_files row, so encrypted_key LEFT-JOINs to null on {}",
            row["action"],
        );
    }
}
