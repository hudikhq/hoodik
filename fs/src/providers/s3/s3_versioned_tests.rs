//! Integration tests for S3 versioned chunk operations.
//!
//! These tests require MinIO running on `localhost:9000` with a bucket named
//! `hoodik` and the default `minioadmin:minioadmin` credentials — exactly
//! what `just minio-up` provides. The suite intentionally has **no
//! skip-when-unavailable logic**: if MinIO isn't up, the tests fail with a
//! clear "run `just minio-up` first" message. CI operators can ensure MinIO
//! is running; hiding infrastructure gaps behind skipped tests amounts to
//! shipping untested code.
//!
//! Each test carves out its own prefix (`it-{uuid}/`) inside the shared
//! bucket so runs don't collide, and the `TestScope` RAII guard tears the
//! prefix down on drop.

use crate::filename::Filename;
use crate::providers::s3::S3Provider;
use crate::{contract::FsProviderContract, MAX_CHUNK_SIZE_BYTES};
use futures_util::stream::StreamExt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Build a MinIO-backed provider with a unique key prefix. The prefix is
/// torn down by `TestScope::drop` regardless of test outcome.
async fn scope() -> TestScope {
    scope_with_direct_transfer(true).await
}

/// Same, with direct transfer switched off, so a test can observe the
/// provider withholding URLs.
async fn scope_with_direct_transfer(direct_transfer: bool) -> TestScope {
    let run_id = Uuid::new_v4();
    let prefix = format!("it-{}/", run_id);
    let config = config::s3::S3Config {
        bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "hoodik".into()),
        region: std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into()),
        endpoint: Some(
            std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:9000".into()),
        ),
        access_key: std::env::var("S3_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".into()),
        secret_key: std::env::var("S3_SECRET_KEY").unwrap_or_else(|_| "minioadmin".into()),
        path_style: std::env::var("S3_PATH_STYLE")
            .map(|v| v == "true")
            .unwrap_or(true),
        prefix: Some(prefix.clone()),
        direct_transfer,
        direct_expiry_secs: 3600,
        direct_allow_insecure: true,
    };

    // The provider hands out URLs only when the config asks for them *and*
    // startup's transport probe agreed, and nothing here runs startup — so
    // without this every direct test asks for URLs and gets `None`. Standing
    // in for a server that passed its probes is the honest fixture: the
    // probes check the transport, which is exactly what these tests then go
    // over real HTTP to prove. Recording is first-call-wins and process-wide,
    // and the withheld-URL tests still hold: `direct_enabled` needs both, so
    // `direct_transfer: false` withholds regardless of the verdict.
    config::direct::record(config::direct::DirectVerdict {
        enabled: true,
        blockers: vec![],
    });

    let provider = S3Provider::new(&config);

    // Smoke-test the bucket so a fresh developer sees "run `just minio-up`"
    // rather than a cryptic DNS or auth error. Listing this scope's own
    // prefix rather than the whole bucket: it proves reachability and
    // credentials just as well, and the remote bucket CI uses is shared with
    // the dogfood rig, whose objects every scope would otherwise page through.
    let listing = provider.bucket().list(prefix.clone(), None).await;
    assert!(
        listing.is_ok(),
        "no S3 endpoint answered at the configured address: {:?}. \
         Bring MinIO up with `just minio-up`, or set S3_ENDPOINT \
         to point somewhere that speaks S3.",
        listing.err()
    );

    TestScope { provider, prefix }
}

struct TestScope {
    provider: S3Provider,
    prefix: String,
}

impl TestScope {
    fn p(&self) -> &S3Provider {
        &self.provider
    }

    /// Every key under this scope's prefix, so a test can assert which
    /// layout a write chose rather than inferring it from a read.
    async fn keys(&self) -> Vec<String> {
        self.provider
            .bucket()
            .list(self.prefix.clone(), None)
            .await
            .expect("list for keys")
            .into_iter()
            .flat_map(|r| r.contents.into_iter().map(|o| o.key))
            .collect()
    }

    async fn clean(&self) {
        let objects = self
            .provider
            .bucket()
            .list(self.prefix.clone(), None)
            .await
            .expect("list for cleanup");
        for result in objects {
            for obj in result.contents {
                let _ = self.provider.bucket().delete_object(&obj.key).await;
            }
        }
    }
}

impl Drop for TestScope {
    fn drop(&mut self) {
        // Best-effort teardown. If this fails the bucket still holds the
        // prefix, but it's unique per run so it won't pollute the next one.
        let bucket = self.provider.bucket().clone();
        let prefix = self.prefix.clone();
        let rt = tokio::runtime::Handle::try_current();
        if rt.is_err() {
            return;
        }
        // Spawn a detached best-effort cleanup on the current runtime.
        tokio::task::spawn(async move {
            if let Ok(results) = bucket.list(prefix, None).await {
                for result in results {
                    for obj in result.contents {
                        let _ = bucket.delete_object(&obj.key).await;
                    }
                }
            }
        });
    }
}

fn fname() -> Filename {
    Filename::new(Uuid::new_v4().to_string())
}

fn fname_with_timestamp() -> Filename {
    Filename::new(Uuid::new_v4().to_string()).with_timestamp(1_234_567_890_i64.to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn s3_versioned_round_trip() {
    let s = scope().await;
    let filename = fname();

    s.p().push_v(&filename, 2, 0, b"hello").await.unwrap();
    s.p().push_v(&filename, 2, 1, b"world").await.unwrap();
    s.p().push_v(&filename, 2, 2, b"!!!").await.unwrap();

    let chunks = s.p().get_uploaded_chunks_v(&filename, 2).await.unwrap();
    assert_eq!(chunks, vec![0, 1, 2]);

    assert!(s.p().exists_v(&filename, 2, 0).await.unwrap());
    assert!(s.p().exists_v(&filename, 2, 2).await.unwrap());
    assert!(!s.p().exists_v(&filename, 2, 99).await.unwrap());

    let got = s.p().pull_v(&filename, 2, 1).await.unwrap();
    assert_eq!(got, b"world");

    s.clean().await;
}

#[tokio::test]
async fn s3_legacy_fallback_for_v1() {
    let s = scope().await;
    let filename = fname_with_timestamp();

    s.p().push(&filename, 0, b"old-0").await.unwrap();
    s.p().push(&filename, 1, b"old-1").await.unwrap();

    let chunks = s.p().get_uploaded_chunks_v(&filename, 1).await.unwrap();
    assert_eq!(chunks, vec![0, 1]);

    let bytes = s.p().pull_v(&filename, 1, 1).await.unwrap();
    assert_eq!(bytes, b"old-1");

    assert!(s.p().exists_v(&filename, 1, 0).await.unwrap());
    assert!(!s.p().exists_v(&filename, 1, 7).await.unwrap());

    s.clean().await;
}

#[tokio::test]
async fn s3_legacy_fallback_skipped_when_versioned_exists() {
    let s = scope().await;
    let filename = fname_with_timestamp();

    s.p().push(&filename, 0, b"legacy").await.unwrap();

    // Not push_v: it probes, and with v1 empty it would fall back to the very
    // flat key this test needs to see ignored. copy_version always writes the
    // versioned layout, so it can put a chunk where nothing else would.
    s.p().push_v(&filename, 2, 0, b"versioned").await.unwrap();
    s.p()
        .copy_version(&filename, 2, &filename, 1)
        .await
        .unwrap();

    let bytes = s.p().pull_v(&filename, 1, 0).await.unwrap();
    assert_eq!(bytes, b"versioned");
    assert_eq!(s.p().pull(&filename, 0).await.unwrap(), b"legacy");

    s.clean().await;
}

#[tokio::test]
async fn s3_copy_version_in_place() {
    let s = scope().await;
    let filename = fname();

    s.p().push_v(&filename, 3, 0, b"a").await.unwrap();
    s.p().push_v(&filename, 3, 1, b"b").await.unwrap();

    s.p()
        .copy_version(&filename, 3, &filename, 4)
        .await
        .unwrap();

    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 3).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 4).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(s.p().pull_v(&filename, 4, 0).await.unwrap(), b"a");
    assert_eq!(s.p().pull_v(&filename, 4, 1).await.unwrap(), b"b");

    s.clean().await;
}

#[tokio::test]
async fn s3_copy_version_across_files() {
    let s = scope().await;
    let src = fname();
    let dst = fname();

    s.p().push_v(&src, 2, 0, b"hi").await.unwrap();
    s.p().push_v(&src, 2, 1, b"there").await.unwrap();

    s.p().copy_version(&src, 2, &dst, 1).await.unwrap();

    assert_eq!(
        s.p().get_uploaded_chunks_v(&src, 2).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(
        s.p().get_uploaded_chunks_v(&dst, 1).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(s.p().pull_v(&dst, 1, 0).await.unwrap(), b"hi");
    assert_eq!(s.p().pull_v(&dst, 1, 1).await.unwrap(), b"there");

    s.clean().await;
}

#[tokio::test]
async fn s3_copy_version_from_legacy_source() {
    let s = scope().await;
    let filename = fname_with_timestamp();

    s.p().push(&filename, 0, b"x").await.unwrap();
    s.p().push(&filename, 1, b"y").await.unwrap();

    s.p()
        .copy_version(&filename, 1, &filename, 2)
        .await
        .unwrap();

    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 2).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(s.p().pull_v(&filename, 2, 0).await.unwrap(), b"x");
    assert_eq!(s.p().pull_v(&filename, 2, 1).await.unwrap(), b"y");

    s.clean().await;
}

#[tokio::test]
async fn s3_version_one_lands_in_the_flat_layout() {
    let s = scope().await;
    let filename = fname();

    s.p().push_v(&filename, 1, 0, b"v1").await.unwrap();

    let keys = s.keys().await;
    assert!(
        keys.iter().any(|k| k.ends_with(".part.0")),
        "version 1 should be written flat, got {:?}",
        keys
    );
    assert!(
        !keys.iter().any(|k| k.contains("/v1/")),
        "nothing should be under v1/, got {:?}",
        keys
    );

    s.clean().await;
}

#[tokio::test]
async fn s3_purge_version_isolated() {
    let s = scope().await;
    let filename = fname();

    s.p().push_v(&filename, 1, 0, b"v1").await.unwrap();
    s.p().push_v(&filename, 2, 0, b"v2").await.unwrap();

    s.p().purge_version(&filename, 1).await.unwrap();

    assert!(s
        .p()
        .get_uploaded_chunks_v(&filename, 1)
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 2).await.unwrap(),
        vec![0]
    );

    s.clean().await;
}

#[tokio::test]
async fn s3_purge_version_missing_is_ok() {
    let s = scope().await;
    let filename = fname();

    s.p().purge_version(&filename, 99).await.unwrap();
}

#[tokio::test]
async fn s3_purge_all_removes_versions_and_legacy() {
    let s = scope().await;
    let filename = fname_with_timestamp();

    s.p().push(&filename, 0, b"legacy").await.unwrap();
    s.p().push_v(&filename, 2, 0, b"versioned").await.unwrap();

    s.p().purge_all(&filename).await.unwrap();

    assert!(s
        .p()
        .get_uploaded_chunks_v(&filename, 2)
        .await
        .unwrap()
        .is_empty());
    assert!(s
        .p()
        .get_uploaded_chunks(&filename)
        .await
        .unwrap()
        .is_empty());
}

/// `rust-s3` is built without `fail-on-err`, so a `NoSuchKey` arrives as an
/// `Ok` whose body is S3's `<Error>` XML. Streamed straight through, the client
/// hands that XML to the cipher and reports a decryption failure instead of a
/// missing chunk. Both stream entry points have to resolve the absence before
/// they hand back a `Streamer`, because the caller has already committed a 200
/// by the time it polls one.
#[tokio::test]
async fn s3_stream_missing_chunk_is_not_found() {
    let s = scope().await;

    let versioned = fname();
    s.p().push_v(&versioned, 2, 0, b"present").await.unwrap();
    assert_not_found(
        s.p().stream_v(&versioned, 2, Some(9)).await.err(),
        "stream_v",
    );
    assert_not_found(s.p().pull_v(&versioned, 2, 9).await.err(), "pull_v");

    let legacy = fname();
    s.p().push(&legacy, 0, b"present").await.unwrap();
    assert_not_found(s.p().stream(&legacy, Some(9)).await.err(), "stream");
    assert_not_found(s.p().pull(&legacy, 9).await.err(), "pull");

    // A chunk that is there still streams its bytes.
    let streamer = s.p().stream_v(&versioned, 2, Some(0)).await.unwrap();
    let mut stream = Box::pin(streamer.stream());
    let bytes = stream.next().await.unwrap().unwrap();
    assert_eq!(bytes.as_ref(), b"present");

    s.clean().await;
}

fn assert_not_found(err: Option<error::Error>, what: &str) {
    match err {
        Some(e) => assert!(e.is_not_found(), "{what}: expected NotFound, got {e:?}"),
        None => panic!("{what}: missing chunk resolved Ok"),
    }
}

/// Bulk-delete path: stage 1050 tiny versioned chunks, then purge. This
/// exercises pagination of `ListObjects` and confirms `DeleteObjects`
/// batches at the 1000-key boundary.
#[tokio::test]
async fn s3_purge_version_batch_gt_1000_chunks() {
    let s = scope().await;
    let filename = fname();

    // Sanity-assert the chunk size constant exists and is small — the test
    // pushes 1050 tiny bodies, not 1050 × 4 MiB.
    let _ = MAX_CHUNK_SIZE_BYTES;

    const N: i64 = 1050;
    for i in 0..N {
        // One-byte payload keeps the test fast and keeps MinIO disk use
        // negligible. The point is breadth, not depth.
        s.p().push_v(&filename, 7, i, b"x").await.unwrap();
    }

    let chunks = s.p().get_uploaded_chunks_v(&filename, 7).await.unwrap();
    assert_eq!(chunks.len(), N as usize);

    s.p().purge_version(&filename, 7).await.unwrap();

    let after = s.p().get_uploaded_chunks_v(&filename, 7).await.unwrap();
    assert!(
        after.is_empty(),
        "{} chunks survived purge_version",
        after.len()
    );

    s.clean().await;
}

// ---------------------------------------------------------------------------
// Direct transfer
//
// These are the only tests that prove a presigned URL is usable. Everything
// else about the feature can compile and still hand clients URLs that 403,
// because the signature is computed locally and never checked until something
// fetches it. So each of these goes over real HTTP to MinIO.
// ---------------------------------------------------------------------------

/// A signed read URL returns exactly the bytes that were pushed, to a client
/// that presents no credentials of any kind.
#[tokio::test]
async fn presigned_get_returns_the_chunk_to_an_unauthenticated_client() {
    let s = scope().await;
    let filename = fname();
    let body = b"ciphertext-for-the-signed-url";

    s.p().push_v(&filename, 3, 0, body).await.unwrap();

    let urls = s
        .p()
        .direct_get_urls(&filename, 3, &[0])
        .await
        .unwrap()
        .expect("s3 provider should offer direct urls when direct_transfer is on");
    assert_eq!(urls.len(), 1);

    // A bare client: no access key, no session, nothing but the URL. This is
    // the client the feature actually ships to.
    let fetched = reqwest::Client::new()
        .get(&urls[0])
        .send()
        .await
        .expect("presigned GET should reach MinIO");

    assert_eq!(fetched.status().as_u16(), 200, "presigned GET was rejected");
    assert_eq!(fetched.bytes().await.unwrap().as_ref(), body);

    s.clean().await;
}

/// URLs come back aligned with the chunk indices that were asked for, and each
/// one addresses its own chunk. A transposition here would decrypt as garbage.
#[tokio::test]
async fn presigned_get_urls_map_to_the_right_chunks() {
    let s = scope().await;
    let filename = fname();

    for i in 0..4i64 {
        s.p()
            .push_v(&filename, 1, i, format!("chunk-{i}").as_bytes())
            .await
            .unwrap();
    }

    let requested = [3i64, 0, 2, 1];
    let urls = s
        .p()
        .direct_get_urls(&filename, 1, &requested)
        .await
        .unwrap()
        .unwrap();

    let client = reqwest::Client::new();
    for (position, chunk) in requested.iter().enumerate() {
        let body = client
            .get(&urls[position])
            .send()
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(
            body.as_ref(),
            format!("chunk-{chunk}").as_bytes(),
            "url at position {position} did not address chunk {chunk}"
        );
    }

    s.clean().await;
}

/// A signed write URL accepts the exact body it was signed for, and the object
/// it produces is indistinguishable from one pushed through the server.
#[tokio::test]
async fn presigned_put_writes_a_chunk_the_provider_can_read_back() {
    let s = scope().await;
    let filename = fname();
    let body = b"written-straight-into-the-bucket";

    let urls = s
        .p()
        .direct_put_urls(&filename, 5, &[(0, body.len() as u64)])
        .await
        .unwrap()
        .expect("s3 provider should offer direct urls when direct_transfer is on");

    let response = reqwest::Client::new()
        .put(&urls[0])
        .header("content-length", body.len().to_string())
        .body(body.to_vec())
        .send()
        .await
        .expect("presigned PUT should reach MinIO");
    assert!(
        response.status().is_success(),
        "presigned PUT was rejected with {}",
        response.status()
    );

    // The server's own read path has to see it, or finalize would never
    // count it and the upload would hang at 'chunks missing'.
    assert_eq!(s.p().pull_v(&filename, 5, 0).await.unwrap(), body);
    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 5).await.unwrap(),
        vec![0]
    );

    s.clean().await;
}

/// A first upload writes where the non-versioned read path looks.
///
/// This is the shape every ordinary file takes: version 1, nothing in the
/// bucket yet, and `use_versioned_layout` false because the file is not
/// editable — so `finalize` counts chunks with `get_uploaded_chunks`, the flat
/// layout. Signing versioned keys regardless put the bytes somewhere that
/// listing never reaches, and every such upload failed as `chunks_missing`
/// after a PUT that had returned 200.
///
/// The test above uses version 5, where the legacy probe never fires, which is
/// why it did not catch this.
#[tokio::test]
async fn presigned_put_on_a_first_upload_lands_in_the_flat_layout() {
    let s = scope().await;
    let filename = fname();
    let body = b"first-upload-straight-into-the-bucket";

    let urls = s
        .p()
        .direct_put_urls(&filename, 1, &[(0, body.len() as u64)])
        .await
        .unwrap()
        .expect("s3 provider should offer direct urls when direct_transfer is on");

    let response = reqwest::Client::new()
        .put(&urls[0])
        .header("content-length", body.len().to_string())
        .body(body.to_vec())
        .send()
        .await
        .expect("presigned PUT should reach the bucket");
    assert!(
        response.status().is_success(),
        "presigned PUT was rejected with {}",
        response.status()
    );

    // What `finalize` runs for a non-editable file. Empty here means the
    // upload is uncommittable however well the write went.
    assert_eq!(
        s.p().get_uploaded_chunks(&filename).await.unwrap(),
        vec![0],
        "a direct write has to be visible to the flat listing finalize uses"
    );
    assert_eq!(s.p().pull(&filename, 0).await.unwrap(), body);

    // And the read manifest has to point back at the same object, or the file
    // uploads and then cannot be downloaded.
    let read = s
        .p()
        .direct_get_urls(&filename, 1, &[0])
        .await
        .unwrap()
        .expect("read urls");
    let fetched = reqwest::Client::new().get(&read[0]).send().await.unwrap();
    assert_eq!(fetched.status().as_u16(), 200);
    assert_eq!(fetched.bytes().await.unwrap().as_ref(), body);

    s.clean().await;
}

/// The signed `content-length` is a limit, not a hint. This is what replaces
/// the per-chunk size cap the relaying upload route applies — without it a
/// presigned write would be an unbounded one.
#[tokio::test]
async fn presigned_put_rejects_a_body_of_the_wrong_length() {
    let s = scope().await;
    let filename = fname();

    let urls = s
        .p()
        .direct_put_urls(&filename, 1, &[(0, 8)])
        .await
        .unwrap()
        .unwrap();

    let oversized = vec![b'x'; 4096];
    let response = reqwest::Client::new()
        .put(&urls[0])
        .header("content-length", oversized.len().to_string())
        .body(oversized)
        .send()
        .await
        .expect("request should reach MinIO even when it is refused");

    assert!(
        !response.status().is_success(),
        "a body longer than the signed content-length was accepted ({}); \
         presigned writes would be unbounded",
        response.status()
    );

    s.clean().await;
}

/// Reading a pre-migration file goes through the legacy flat layout, so the
/// signed URL has to address the legacy key rather than a versioned one that
/// holds nothing.
#[tokio::test]
async fn presigned_get_addresses_legacy_chunks_for_unmigrated_files() {
    let s = scope().await;
    let filename = fname_with_timestamp();
    let body = b"stored-before-versioning-existed";

    s.p().push(&filename, 0, body).await.unwrap();

    let urls = s
        .p()
        .direct_get_urls(&filename, 1, &[0])
        .await
        .unwrap()
        .unwrap();

    let fetched = reqwest::Client::new().get(&urls[0]).send().await.unwrap();
    assert_eq!(
        fetched.status().as_u16(),
        200,
        "legacy chunk was not reachable through its signed url"
    );
    assert_eq!(fetched.bytes().await.unwrap().as_ref(), body);

    s.clean().await;
}

/// With the flag off the provider offers nothing, whatever else is configured.
/// The routes turn that `None` into a 400 rather than a broken transfer.
#[tokio::test]
async fn direct_urls_are_withheld_when_the_flag_is_off() {
    let s = scope_with_direct_transfer(false).await;
    let filename = fname();
    s.p().push_v(&filename, 1, 0, b"x").await.unwrap();

    assert!(s
        .p()
        .direct_get_urls(&filename, 1, &[0])
        .await
        .unwrap()
        .is_none());
    assert!(s
        .p()
        .direct_put_urls(&filename, 1, &[(0, 1)])
        .await
        .unwrap()
        .is_none());

    s.clean().await;
}
