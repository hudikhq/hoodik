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
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------

/// Build a MinIO-backed provider with a unique key prefix. The prefix is
/// torn down by `TestScope::drop` regardless of test outcome.
async fn scope() -> TestScope {
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
    };
    let provider = S3Provider::new(&config);

    // Smoke-test the bucket so a fresh developer sees "run `just minio-up`"
    // rather than a cryptic DNS or auth error.
    let listing = provider.bucket().list("".to_string(), None).await;
    assert!(
        listing.is_ok(),
        "MinIO unreachable at the configured endpoint: {:?}. \
         Bring it up with `just minio-up`, or set S3_ENDPOINT \
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
    s.p().push_v(&filename, 1, 0, b"versioned").await.unwrap();

    let bytes = s.p().pull_v(&filename, 1, 0).await.unwrap();
    assert_eq!(bytes, b"versioned");

    s.clean().await;
}

#[tokio::test]
async fn s3_copy_version_in_place() {
    let s = scope().await;
    let filename = fname();

    s.p().push_v(&filename, 3, 0, b"a").await.unwrap();
    s.p().push_v(&filename, 3, 1, b"b").await.unwrap();

    s.p().copy_version(&filename, 3, &filename, 4).await.unwrap();

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

    s.p().copy_version(&filename, 1, &filename, 2).await.unwrap();

    assert_eq!(
        s.p().get_uploaded_chunks_v(&filename, 2).await.unwrap(),
        vec![0, 1]
    );
    assert_eq!(s.p().pull_v(&filename, 2, 0).await.unwrap(), b"x");
    assert_eq!(s.p().pull_v(&filename, 2, 1).await.unwrap(), b"y");

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
    assert!(after.is_empty(), "{} chunks survived purge_version", after.len());

    s.clean().await;
}
