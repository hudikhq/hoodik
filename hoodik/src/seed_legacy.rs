//! Development-only seeder for the browser migration test.
//!
//! Registration is Curve25519 + OPAQUE only, so a legacy (RSA + bcrypt) account
//! can no longer be created through any API — yet real deployments still hold
//! pre-migration accounts, and the login-time migration ceremony must keep
//! working for them. This binary writes one such account, plus a file and a
//! public link whose keys are wrapped under the old RSA scheme, straight into
//! the e2e SQLite database so Playwright can drive the ceremony through the
//! real UI.
//!
//! Gated behind the `e2e-seed` feature and excluded from every release build —
//! a shipped "seed a legacy account" binary would be indefensible in a public
//! repo. The blob encodings below mirror exactly what the web client writes and
//! reads (`web/services/cryptfns/aes.ts`, `storage/meta.ts`, `links/crypto.ts`);
//! if either side drifts, the browser can no longer open the seeded blobs and
//! the spec fails rather than passing on a migration that never happened.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use entity::{files, links, user_files, users, ActiveValue, EntityTrait, PaginatorTrait, Uuid};
use sea_orm::Database;

/// Name the seeded file and its public link carry once decrypted. The spec
/// asserts on this exact value, so the two must stay in step.
const FILE_NAME: &str = "legacy-photo.png";

/// The file's chunks are encrypted with Ascon-128a; its `files.cipher` is set to
/// match so the browser decrypts with the same algorithm. Ascon keys are 32
/// bytes, which is also what `cryptfns::aes` (the primitive behind link names
/// and the legacy private-key blob) consumes, so one key shape covers the whole
/// fixture.
const CIPHER: &str = "ascon128a";

#[actix_web::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [db_path, email, password, image_path] = match &args[1..] {
        [a, b, c, d] => [a, b, c, d],
        _ => {
            eprintln!("usage: seed_legacy <sqlite_path> <email> <password> <image_path>");
            std::process::exit(2);
        }
    };

    let db = Database::connect(format!("sqlite:{db_path}?mode=rwc"))
        .await
        .expect("connect to e2e sqlite");

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let private = cryptfns::rsa::private::generate().unwrap();
    let rsa_private = cryptfns::rsa::private::to_string(&private).unwrap();
    let public = cryptfns::rsa::public::from_private(&private).unwrap();
    let rsa_public = cryptfns::rsa::public::to_string(&public).unwrap();
    let fingerprint = cryptfns::rsa::fingerprint(public).unwrap();

    // `cryptfns.rsa.decryptPrivateKey` = `aes.decryptString(blob, password)`:
    // the password padded to a 32-byte Ascon key, Ascon-128a, hex-encoded. This
    // is the one blob the browser must open with the typed password before it
    // can even begin the migration.
    let encrypted_private_key = cryptfns::hex::encode(
        cryptfns::aes::encrypt(password_key(password), rsa_private.clone().into_bytes()).unwrap(),
    );

    let user_id = Uuid::new_v4();
    // The register contract makes the first account the instance admin; mirror
    // that so seeding before the Playwright run leaves the same one-admin state
    // the suite already assumes (and never relies on).
    let role = if users::Entity::find().count(&db).await.unwrap() == 0 {
        ActiveValue::Set(Some("admin".to_string()))
    } else {
        ActiveValue::NotSet
    };
    users::Entity::insert(users::ActiveModel {
        id: ActiveValue::Set(user_id),
        role,
        quota: ActiveValue::NotSet,
        email: ActiveValue::Set(email.to_string()),
        password: ActiveValue::Set(Some(util::password::hash(password))),
        secret: ActiveValue::NotSet,
        pubkey: ActiveValue::Set(rsa_public.clone()),
        fingerprint: ActiveValue::Set(fingerprint),
        key_type: ActiveValue::Set("rsa".to_string()),
        wrapping_pubkey: ActiveValue::NotSet,
        security_version: ActiveValue::Set(0),
        opaque_password_file: ActiveValue::NotSet,
        encrypted_private_key: ActiveValue::Set(Some(encrypted_private_key)),
        email_verified_at: ActiveValue::Set(Some(now)),
        created_at: ActiveValue::Set(now),
        updated_at: ActiveValue::Set(now),
        share_notifications_enabled: ActiveValue::Set(true),
    })
    .exec_without_returning(&db)
    .await
    .unwrap();

    // File key: 32 raw bytes. The name is encrypted under it (Ascon-128a, hex);
    // `user_files.encrypted_key` is the RSA encryption of its hex, exactly what
    // the migration ceremony fetches and re-wraps under the new X25519 key.
    let file_key = cryptfns::aes::generate_key().unwrap();
    let file_key_hex = cryptfns::hex::encode(&file_key);
    let file_id = Uuid::new_v4();
    let image = std::fs::read(image_path).expect("read seed image");

    files::Entity::insert(files::ActiveModel {
        id: ActiveValue::Set(file_id),
        name_hash: ActiveValue::Set(cryptfns::sha256::digest(FILE_NAME.as_bytes())),
        encrypted_name: ActiveValue::Set(encrypt_hex(&file_key, FILE_NAME.as_bytes())),
        encrypted_thumbnail: ActiveValue::Set(None),
        mime: ActiveValue::Set("image/png".to_string()),
        size: ActiveValue::Set(Some(image.len() as i64)),
        chunks: ActiveValue::Set(Some(1)),
        chunks_stored: ActiveValue::Set(Some(1)),
        file_id: ActiveValue::Set(None),
        md5: ActiveValue::NotSet,
        sha1: ActiveValue::NotSet,
        sha256: ActiveValue::NotSet,
        blake2b: ActiveValue::NotSet,
        cipher: ActiveValue::Set(CIPHER.to_string()),
        editable: ActiveValue::Set(false),
        file_modified_at: ActiveValue::Set(now),
        created_at: ActiveValue::Set(now),
        finished_upload_at: ActiveValue::Set(Some(now)),
        active_version: ActiveValue::Set(1),
        pending_version: ActiveValue::NotSet,
        pending_chunks: ActiveValue::NotSet,
        pending_size: ActiveValue::NotSet,
        last_membership_change_at: ActiveValue::NotSet,
        members_list_signature: ActiveValue::NotSet,
        members_list_signed_at: ActiveValue::NotSet,
        members_list_signed_by_user_id: ActiveValue::NotSet,
    })
    .exec_without_returning(&db)
    .await
    .unwrap();

    user_files::Entity::insert(user_files::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        file_id: ActiveValue::Set(file_id),
        user_id: ActiveValue::Set(user_id),
        encrypted_key: ActiveValue::Set(
            cryptfns::rsa::public::encrypt(&file_key_hex, &rsa_public).unwrap(),
        ),
        is_owner: ActiveValue::Set(true),
        created_at: ActiveValue::Set(now),
        expires_at: ActiveValue::NotSet,
        share_role: ActiveValue::Set("co-owner".to_string()),
        shared_at: ActiveValue::NotSet,
        shared_by_user_id: ActiveValue::NotSet,
        member_signature: ActiveValue::NotSet,
        member_signed_at: ActiveValue::NotSet,
    })
    .exec_without_returning(&db)
    .await
    .unwrap();

    // One non-versioned chunk on disk, next to the DB, in the flat layout the
    // link download resolves: `{data_dir}/{created_at}-{file_id}.part.0`.
    let data_dir = Path::new(db_path).parent().unwrap();
    let chunk = cryptfns::aes::encrypt(file_key.clone(), image).unwrap();
    std::fs::write(data_dir.join(format!("{now}-{file_id}.part.0")), chunk).unwrap();

    // Public link. The link key wraps the name and the file key (both Ascon-128a
    // under the raw link key); the link key itself is RSA-wrapped under the
    // owner's pubkey — the exact shape the ceremony re-wraps to X25519 and the
    // regression that shipped without it.
    let link_key = cryptfns::aes::generate_key().unwrap();
    let link_id = Uuid::new_v4();
    links::Entity::insert(links::ActiveModel {
        id: ActiveValue::Set(link_id),
        user_id: ActiveValue::Set(user_id),
        file_id: ActiveValue::Set(file_id),
        signature: ActiveValue::Set(
            cryptfns::rsa::private::sign(&file_id.to_string(), &rsa_private).unwrap(),
        ),
        downloads: ActiveValue::Set(0),
        encrypted_name: ActiveValue::Set(encrypt_hex(&link_key, FILE_NAME.as_bytes())),
        encrypted_link_key: ActiveValue::Set(
            cryptfns::rsa::public::encrypt(&cryptfns::hex::encode(&link_key), &rsa_public).unwrap(),
        ),
        encrypted_thumbnail: ActiveValue::Set(None),
        encrypted_file_key: ActiveValue::Set(Some(encrypt_hex(&link_key, file_key_hex.as_bytes()))),
        created_at: ActiveValue::Set(now),
        expires_at: ActiveValue::Set(None),
    })
    .exec_without_returning(&db)
    .await
    .unwrap();

    println!("seeded legacy account {email} with file + public link {link_id}");
}

/// Ascon-128a encrypt `plaintext` under `key`, hex-encoded — the encoding
/// `aes.encryptString` / `cipher.encryptString` produce for names and keys.
fn encrypt_hex(key: &[u8], plaintext: &[u8]) -> String {
    cryptfns::hex::encode(cryptfns::aes::encrypt(key.to_vec(), plaintext.to_vec()).unwrap())
}

/// Reproduce `keyFromSimpleString`: pad to 32 chars with `'0'` (or truncate),
/// then UTF-8. Passwords here are ASCII, so char count and byte count agree.
fn password_key(password: &str) -> Vec<u8> {
    let mut s = password.to_string();
    if s.len() < 32 {
        s.push_str(&"0".repeat(32 - s.len()));
    } else {
        s.truncate(32);
    }
    s.into_bytes()
}
