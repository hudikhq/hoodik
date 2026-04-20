use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::pin::Pin;
use std::str::FromStr;

use crate::checksum;
use crate::config::{UploadHashOptions, CHUNK_SIZE_BYTES, MAX_UPLOAD_RETRIES};
use crate::error::{Error, HttpError, Result};
use crate::platform::{DataSource, HttpClient, ProgressReporter};
use crate::types::{Auth, ChunkResponse, FileHashes};
use crate::upload::compute_chunk_count;

fn test_auth() -> Auth {
    Auth {
        base_url: "http://localhost:1234".into(),
        jwt_token: Some("test-jwt".into()),
        refresh_token: None,
        cookie: None,
    }
}

fn test_key() -> Vec<u8> {
    // 32 bytes: first 16 = Ascon128a key, last 16 = nonce
    b"test-encryption-key!test-nonce!!" .to_vec()
}

struct MockDataSource {
    data: Vec<u8>,
}

impl MockDataSource {
    fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl DataSource for MockDataSource {
    fn read_chunk(
        &self,
        offset: u64,
        length: u64,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>> {
        let start = offset as usize;
        let end = (offset + length).min(self.data.len() as u64) as usize;
        let chunk = self.data[start..end].to_vec();
        Box::pin(async move { Ok(chunk) })
    }

    fn total_size(&self) -> u64 {
        self.data.len() as u64
    }
}

/// When `scripted_upload_responses` is non-empty, responses are popped FIFO.
/// Otherwise the upload auto-succeeds and stores the chunk in `stored_chunks`
/// so it can be read back during download (roundtrip testing).
struct MockHttpClient {
    scripted_upload_responses: RefCell<std::collections::VecDeque<Result<ChunkResponse>>>,
    stored_chunks: RefCell<HashMap<u64, Vec<u8>>>,
    upload_call_count: Cell<u64>,
    scripted_download_responses: RefCell<std::collections::VecDeque<Result<Vec<u8>>>>,
    received_hashes: RefCell<Option<FileHashes>>,
    scripted_upload_tar_responses: RefCell<std::collections::VecDeque<Result<ChunkResponse>>>,
    last_upload_tar_body: RefCell<Option<Vec<u8>>>,
}

impl MockHttpClient {
    fn new() -> Self {
        Self {
            scripted_upload_responses: RefCell::new(std::collections::VecDeque::new()),
            stored_chunks: RefCell::new(HashMap::new()),
            upload_call_count: Cell::new(0),
            scripted_download_responses: RefCell::new(std::collections::VecDeque::new()),
            received_hashes: RefCell::new(None),
            scripted_upload_tar_responses: RefCell::new(std::collections::VecDeque::new()),
            last_upload_tar_body: RefCell::new(None),
        }
    }

    fn push_upload_error(&self, err: Error) {
        self.scripted_upload_responses
            .borrow_mut()
            .push_back(Err(err));
    }

    fn push_download_error(&self, err: Error) {
        self.scripted_download_responses
            .borrow_mut()
            .push_back(Err(err));
    }

    fn upload_count(&self) -> u64 {
        self.upload_call_count.get()
    }

    fn take_last_tar_body(&self) -> Option<Vec<u8>> {
        self.last_upload_tar_body.borrow_mut().take()
    }
}

impl HttpClient for MockHttpClient {
    fn upload_chunk(
        &self,
        _auth: &Auth,
        _file_id: &str,
        chunk_index: u64,
        _checksum: &str,
        data: &[u8],
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>> {
        self.upload_call_count
            .set(self.upload_call_count.get() + 1);

        let scripted = self.scripted_upload_responses.borrow_mut().pop_front();

        if let Some(result) = scripted {
            if result.is_ok() {
                self.stored_chunks
                    .borrow_mut()
                    .insert(chunk_index, data.to_vec());
            }
            return Box::pin(async move { result });
        }

        self.stored_chunks
            .borrow_mut()
            .insert(chunk_index, data.to_vec());
        let stored = self.stored_chunks.borrow().len() as i64;
        let resp = ChunkResponse {
            chunks_stored: Some(stored),
            finished_upload_at: None,
        };
        Box::pin(async move { Ok(resp) })
    }

    fn download_chunk(
        &self,
        _auth: &Auth,
        _file_id: &str,
        chunk_index: u64,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>> {
        let scripted = self.scripted_download_responses.borrow_mut().pop_front();

        if let Some(result) = scripted {
            return Box::pin(async move { result });
        }

        let data = self.stored_chunks.borrow().get(&chunk_index).cloned();
        Box::pin(async move {
            data.ok_or(Error::Http(HttpError {
                status: 404,
                message: format!("chunk {chunk_index} not found"),
                validation: None,
            }))
        })
    }

    fn download_all_chunks(
        &self,
        _auth: &Auth,
        _file_id: &str,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + '_>> {
        let stored = self.stored_chunks.borrow();
        let mut indices: Vec<u64> = stored.keys().copied().collect();
        indices.sort();

        let entries: Vec<(String, Vec<u8>)> = indices
            .into_iter()
            .filter_map(|idx| stored.get(&idx).map(|d| (format!("{:06}.enc", idx), d.clone())))
            .collect();
        let archive = crate::tar::build_tar(&entries);
        Box::pin(async move { Ok(archive) })
    }

    fn upload_chunks_tar(
        &self,
        _auth: &Auth,
        _file_id: &str,
        tar_body: Vec<u8>,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>> {
        let scripted = self.scripted_upload_tar_responses.borrow_mut().pop_front();
        *self.last_upload_tar_body.borrow_mut() = Some(tar_body.clone());

        if let Some(result) = scripted {
            return Box::pin(async move { result });
        }

        let entries = match crate::tar::extract_tar(&tar_body) {
            Ok(e) => e,
            Err(e) => return Box::pin(async move { Err(e) }),
        };

        for entry in &entries {
            let idx: u64 = entry
                .name
                .strip_suffix(".enc")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);
            self.stored_chunks
                .borrow_mut()
                .insert(idx, entry.data.clone());
        }

        let stored = self.stored_chunks.borrow().len() as i64;
        let resp = ChunkResponse {
            chunks_stored: Some(stored),
            finished_upload_at: None,
        };
        Box::pin(async move { Ok(resp) })
    }

    fn update_hashes(
        &self,
        _auth: &Auth,
        _file_id: &str,
        hashes: &FileHashes,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>> {
        *self.received_hashes.borrow_mut() = Some(hashes.clone());
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum ProgressEvent {
    ChunkUploaded {
        chunk: u64,
        total_chunks: u64,
        is_done: bool,
    },
    ChunkDownloaded {
        bytes: u64,
        total_bytes: u64,
    },
    Error {
        error: String,
    },
    Complete,
}

/// Records all progress callbacks. If `cancel_on_call` is set, `is_cancelled`
/// returns `true` once that many calls have been made (0 = immediate).
struct MockProgressReporter {
    events: RefCell<Vec<ProgressEvent>>,
    cancel_on_call: Option<u32>,
    is_cancelled_calls: Cell<u32>,
}

impl MockProgressReporter {
    fn new() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            cancel_on_call: None,
            is_cancelled_calls: Cell::new(0),
        }
    }

    fn cancelling_immediately() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
            cancel_on_call: Some(0),
            is_cancelled_calls: Cell::new(0),
        }
    }

    fn events(&self) -> Vec<ProgressEvent> {
        self.events.borrow().clone()
    }
}

impl ProgressReporter for MockProgressReporter {
    fn on_chunk_uploaded(&self, _file_id: &str, chunk: u64, total_chunks: u64, is_done: bool) {
        self.events.borrow_mut().push(ProgressEvent::ChunkUploaded {
            chunk,
            total_chunks,
            is_done,
        });
    }

    fn on_chunk_downloaded(&self, _file_id: &str, bytes: u64, total_bytes: u64) {
        self.events
            .borrow_mut()
            .push(ProgressEvent::ChunkDownloaded { bytes, total_bytes });
    }

    fn on_error(&self, _file_id: &str, error: &str) {
        self.events.borrow_mut().push(ProgressEvent::Error {
            error: error.to_string(),
        });
    }

    fn on_complete(&self, _file_id: &str) {
        self.events.borrow_mut().push(ProgressEvent::Complete);
    }

    fn is_cancelled(&self, _file_id: &str) -> bool {
        let n = self.is_cancelled_calls.get();
        self.is_cancelled_calls.set(n + 1);
        self.cancel_on_call.is_some_and(|threshold| n >= threshold)
    }
}

#[test]
fn chunk_count_zero() {
    assert_eq!(compute_chunk_count(0), 1);
}

#[test]
fn chunk_count_exact() {
    assert_eq!(compute_chunk_count(CHUNK_SIZE_BYTES), 1);
}

#[test]
fn chunk_count_remainder() {
    assert_eq!(compute_chunk_count(CHUNK_SIZE_BYTES + 1), 2);
}

#[test]
fn chunk_count_multiple_exact() {
    assert_eq!(compute_chunk_count(CHUNK_SIZE_BYTES * 3), 3);
}

#[test]
fn crc16_deterministic() {
    let data = b"hello world";
    let a = checksum::crc16(data);
    let b = checksum::crc16(data);
    assert_eq!(a, b);
    assert!(!a.is_empty());
}

#[tokio::test(flavor = "current_thread")]
async fn upload_small_file() {
    let data = vec![42u8; 256];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    let hashes = crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f1", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    assert_eq!(http.upload_count(), 1);
    assert_eq!(http.stored_chunks.borrow().len(), 1);
    assert!(!hashes.sha256.is_empty());
    assert!(hashes.md5.as_ref().is_some_and(|s| !s.is_empty()));
    assert!(http.received_hashes.borrow().is_some());

    let events = progress.events();
    assert!(matches!(events.last(), Some(ProgressEvent::Complete)));
    assert!(events
        .iter()
        .any(|e| matches!(e, ProgressEvent::ChunkUploaded { chunk: 0, .. })));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_multi_chunk() {
    let size = CHUNK_SIZE_BYTES as usize + 1000;
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f2", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    assert_eq!(http.upload_count(), 2);
    assert_eq!(http.stored_chunks.borrow().len(), 2);

    let upload_events: Vec<_> = progress
        .events()
        .iter()
        .filter(|e| matches!(e, ProgressEvent::ChunkUploaded { .. }))
        .cloned()
        .collect();
    assert_eq!(upload_events.len(), 2);
    assert!(matches!(
        progress.events().last(),
        Some(ProgressEvent::Complete)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_resume_skips_existing() {
    let size = CHUNK_SIZE_BYTES as usize * 2 + 500;
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    // Chunks 0 and 2 already uploaded — only chunk 1 should be sent
    crate::upload::upload_file(
        &http,
        &source,
        &progress,
        &test_auth(),
        "f3",
        &test_key(),
        &[0, 2],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert_eq!(http.upload_count(), 1);
    assert!(http.stored_chunks.borrow().contains_key(&1));
    assert!(!http.stored_chunks.borrow().contains_key(&0));
    assert!(!http.stored_chunks.borrow().contains_key(&2));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_checksum_retry_succeeds() {
    let data = vec![7u8; 256];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    let mut validation = HashMap::new();
    validation.insert(
        "checksum".into(),
        "checksum_mismatch: 'aaa' != 'bbb'".into(),
    );
    http.push_upload_error(Error::Http(HttpError {
        status: 422,
        message: "Validation error".into(),
        validation: Some(validation),
    }));

    crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f4", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    // First call failed, second succeeded
    assert_eq!(http.upload_count(), 2);
    assert!(matches!(
        progress.events().last(),
        Some(ProgressEvent::Complete)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_checksum_exhausts_retries() {
    let data = vec![7u8; 256];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    // Initial attempt + MAX_UPLOAD_RETRIES retries = MAX_UPLOAD_RETRIES + 1 total
    for _ in 0..=MAX_UPLOAD_RETRIES {
        let mut validation = HashMap::new();
        validation.insert("checksum".into(), "checksum_mismatch".into());
        http.push_upload_error(Error::Http(HttpError {
            status: 422,
            message: "Validation error".into(),
            validation: Some(validation),
        }));
    }

    let result =
        crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f5", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
            .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::Http(_)));
    assert_eq!(http.upload_count(), MAX_UPLOAD_RETRIES as u64 + 1);
    assert!(progress
        .events()
        .iter()
        .any(|e| matches!(e, ProgressEvent::Error { .. })));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_chunk_already_exists_is_not_an_error() {
    let data = vec![42u8; 256];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    let mut validation = HashMap::new();
    validation.insert("chunk".into(), "chunk_already_exists".into());
    http.push_upload_error(Error::Http(HttpError {
        status: 422,
        message: "Validation error".into(),
        validation: Some(validation),
    }));

    crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f6", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    assert_eq!(http.upload_count(), 1);
    assert!(matches!(
        progress.events().last(),
        Some(ProgressEvent::Complete)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn upload_cancelled_before_any_work() {
    let data = vec![42u8; 256];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::cancelling_immediately();

    let result =
        crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f7", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
            .await;

    assert!(matches!(result, Err(Error::Cancelled)));
    assert_eq!(http.upload_count(), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn upload_empty_file() {
    let source = MockDataSource::new(vec![]);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    // Zero-byte file still produces 1 chunk. Ascon128a adds a 16-byte auth tag,
    // so the encrypted output is non-empty and the upload succeeds.
    crate::upload::upload_file(&http, &source, &progress, &test_auth(), "f8", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    assert_eq!(http.upload_count(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn upload_hashes_are_deterministic() {
    let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    // Fresh upload
    let source1 = MockDataSource::new(data.clone());
    let http1 = MockHttpClient::new();
    let progress1 = MockProgressReporter::new();
    let hashes1 = crate::upload::upload_file(
        &http1,
        &source1,
        &progress1,
        &test_auth(),
        "h1",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    // Same data again
    let source2 = MockDataSource::new(data.clone());
    let http2 = MockHttpClient::new();
    let progress2 = MockProgressReporter::new();
    let hashes2 = crate::upload::upload_file(
        &http2,
        &source2,
        &progress2,
        &test_auth(),
        "h2",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert_eq!(hashes1.sha256, hashes2.sha256);
    assert_eq!(hashes1.md5, hashes2.md5);
    assert_eq!(hashes1.sha1, hashes2.sha1);
    assert_eq!(hashes1.blake2b, hashes2.blake2b);
}

#[tokio::test(flavor = "current_thread")]
async fn upload_resume_produces_same_hashes_as_fresh() {
    let size = CHUNK_SIZE_BYTES as usize * 3 + 500;
    let data: Vec<u8> = (0..size).map(|i| (i % 256) as u8).collect();

    // Fresh upload
    let source1 = MockDataSource::new(data.clone());
    let http1 = MockHttpClient::new();
    let progress1 = MockProgressReporter::new();
    let fresh_hashes = crate::upload::upload_file(
        &http1,
        &source1,
        &progress1,
        &test_auth(),
        "r1",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    // Resumed upload (chunks 0 and 2 already uploaded)
    let source2 = MockDataSource::new(data);
    let http2 = MockHttpClient::new();
    let progress2 = MockProgressReporter::new();
    let resume_hashes = crate::upload::upload_file(
        &http2,
        &source2,
        &progress2,
        &test_auth(),
        "r2",
        &test_key(),
        &[0, 2],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert_eq!(fresh_hashes.sha256, resume_hashes.sha256);
    assert_eq!(fresh_hashes.md5, resume_hashes.md5);
    assert_eq!(fresh_hashes.sha1, resume_hashes.sha1);
    assert_eq!(fresh_hashes.blake2b, resume_hashes.blake2b);

    // Resumed upload should only upload chunks 1 and 3
    assert_eq!(http2.upload_count(), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn upload_hash_disable_mask_omits_optional_hashes() {
    let data = vec![9u8; 512];
    let source = MockDataSource::new(data);
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    let hashes = crate::upload::upload_file(
        &http,
        &source,
        &progress,
        &test_auth(),
        "mask",
        &test_key(),
        &[],
        UploadHashOptions::from_disable_mask(
            crate::config::HASH_DISABLE_MD5
                | crate::config::HASH_DISABLE_SHA1
                | crate::config::HASH_DISABLE_BLAKE2B,
        ),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert!(!hashes.sha256.is_empty());
    assert!(hashes.md5.is_none());
    assert!(hashes.sha1.is_none());
    assert!(hashes.blake2b.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn download_roundtrip() {
    let original = b"Hello, this is a roundtrip test with some data!".to_vec();
    let source = MockDataSource::new(original.clone());
    let http = MockHttpClient::new();
    let up_progress = MockProgressReporter::new();

    crate::upload::upload_file(
        &http,
        &source,
        &up_progress,
        &test_auth(),
        "rt",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    let dl_progress = MockProgressReporter::new();
    let chunk_count = compute_chunk_count(original.len() as u64);
    let downloaded = crate::download::download_file(
        &http,
        &dl_progress,
        &test_auth(),
        "rt",
        original.len() as u64,
        chunk_count,
        &test_key(),
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert_eq!(downloaded, original);
    assert!(matches!(
        dl_progress.events().last(),
        Some(ProgressEvent::Complete)
    ));
}

#[tokio::test(flavor = "current_thread")]
async fn download_multi_chunk_ordering() {
    let cs = CHUNK_SIZE_BYTES as usize;
    let mut data = Vec::with_capacity(cs * 2 + 500);
    data.extend(vec![0xAAu8; cs]);
    data.extend(vec![0xBBu8; cs]);
    data.extend(vec![0xCCu8; 500]);

    let source = MockDataSource::new(data.clone());
    let http = MockHttpClient::new();
    let up = MockProgressReporter::new();

    crate::upload::upload_file(&http, &source, &up, &test_auth(), "ord", &test_key(), &[], UploadHashOptions::default(), None, cryptfns::cipher::DEFAULT)
        .await
        .unwrap();

    let dl = MockProgressReporter::new();
    let chunk_count = compute_chunk_count(data.len() as u64);
    let downloaded = crate::download::download_file(
        &http,
        &dl,
        &test_auth(),
        "ord",
        data.len() as u64,
        chunk_count,
        &test_key(),
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    assert_eq!(downloaded, data);
    assert!(downloaded[..cs].iter().all(|&b| b == 0xAA));
    assert!(downloaded[cs..cs * 2].iter().all(|&b| b == 0xBB));
    assert!(downloaded[cs * 2..].iter().all(|&b| b == 0xCC));
}

#[tokio::test(flavor = "current_thread")]
async fn download_cancelled_before_any_work() {
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::cancelling_immediately();

    let result = crate::download::download_file(
        &http,
        &progress,
        &test_auth(),
        "cancel",
        1000,
        1,
        &test_key(),
        cryptfns::cipher::DEFAULT,
    )
    .await;

    assert!(matches!(result, Err(Error::Cancelled)));
}

#[tokio::test(flavor = "current_thread")]
async fn download_http_error_propagates() {
    let http = MockHttpClient::new();
    let progress = MockProgressReporter::new();

    http.push_download_error(Error::Http(HttpError {
        status: 500,
        message: "Internal Server Error".into(),
        validation: None,
    }));

    let result = crate::download::download_file(
        &http,
        &progress,
        &test_auth(),
        "err",
        1000,
        1,
        &test_key(),
        cryptfns::cipher::DEFAULT,
    )
    .await;

    match result {
        Err(Error::Http(e)) => assert_eq!(e.status, 500),
        other => panic!("Expected Http error, got: {other:?}"),
    }
}

#[tokio::test(flavor = "current_thread")]
async fn download_all_chunks_tar_roundtrip() {
    let original = b"Hello, tar download roundtrip test!".to_vec();
    let source = MockDataSource::new(original.clone());
    let http = MockHttpClient::new();
    let up = MockProgressReporter::new();

    crate::upload::upload_file(
        &http,
        &source,
        &up,
        &test_auth(),
        "tar_rt",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    // Download as tar archive and extract entries.
    let tar_data = http
        .download_all_chunks(&test_auth(), "tar_rt")
        .await
        .unwrap();
    let entries = crate::tar::extract_tar(&tar_data).unwrap();

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "000000.enc");
    // Encrypted data should be non-empty (original + auth tag).
    assert!(!entries[0].data.is_empty());

    // Decrypt the chunk and verify it matches the original.
    let cipher = cryptfns::cipher::Cipher::from_str(cryptfns::cipher::DEFAULT).unwrap();
    let decrypted = cipher.decrypt(test_key(), entries[0].data.clone()).unwrap();
    assert_eq!(decrypted, original);
}

#[tokio::test(flavor = "current_thread")]
async fn download_all_chunks_tar_multi_chunk() {
    let cs = CHUNK_SIZE_BYTES as usize;
    let mut data = Vec::with_capacity(cs + 500);
    data.extend(vec![0xAAu8; cs]);
    data.extend(vec![0xBBu8; 500]);

    let source = MockDataSource::new(data.clone());
    let http = MockHttpClient::new();
    let up = MockProgressReporter::new();

    crate::upload::upload_file(
        &http,
        &source,
        &up,
        &test_auth(),
        "tar_mc",
        &test_key(),
        &[],
        UploadHashOptions::default(),
        None,
        cryptfns::cipher::DEFAULT,
    )
    .await
    .unwrap();

    let tar_data = http
        .download_all_chunks(&test_auth(), "tar_mc")
        .await
        .unwrap();
    let entries = crate::tar::extract_tar(&tar_data).unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "000000.enc");
    assert_eq!(entries[1].name, "000001.enc");

    // Decrypt each chunk and reassemble.
    let cipher = cryptfns::cipher::Cipher::from_str(cryptfns::cipher::DEFAULT).unwrap();
    let mut reassembled = Vec::new();
    for entry in &entries {
        let plaintext = cipher.decrypt(test_key(), entry.data.clone()).unwrap();
        reassembled.extend_from_slice(&plaintext);
    }
    assert_eq!(reassembled, data);
}

#[tokio::test(flavor = "current_thread")]
async fn upload_tar_roundtrip_via_mock() {
    // Feed N chunks through the in-memory tar uploader and verify the tar
    // body the mock captured contains the exact same bytes back out.
    let http = MockHttpClient::new();
    let up = MockProgressReporter::new();

    let entries = vec![
        ("000000.enc".to_string(), vec![0xAAu8; 2048]),
        ("000001.enc".to_string(), vec![0xBBu8; 1024]),
        ("000002.enc".to_string(), vec![0xCCu8; 512]),
    ];

    crate::upload_tar::upload_chunks_as_tar_in_memory(
        &http,
        &up,
        &test_auth(),
        "file-roundtrip",
        entries.clone(),
    )
    .await
    .unwrap();

    let captured = http.take_last_tar_body().expect("mock must record the tar body");
    let parsed = crate::tar::extract_tar(&captured).expect("captured body must parse as tar");
    assert_eq!(parsed.len(), entries.len());
    for (expected, actual) in entries.iter().zip(parsed.iter()) {
        assert_eq!(actual.name, expected.0);
        assert_eq!(actual.data, expected.1);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn upload_tar_progress_reported() {
    let http = MockHttpClient::new();
    let up = MockProgressReporter::new();

    let entries = vec![
        ("000000.enc".to_string(), vec![0u8; 1024]),
        ("000001.enc".to_string(), vec![1u8; 1024]),
    ];

    crate::upload_tar::upload_chunks_as_tar_in_memory(
        &http,
        &up,
        &test_auth(),
        "file-progress",
        entries,
    )
    .await
    .unwrap();

    let events = up.events();
    let mut downloaded_bytes: Vec<u64> = Vec::new();
    let mut saw_complete = false;
    for ev in events {
        match ev {
            ProgressEvent::ChunkDownloaded { bytes, .. } => downloaded_bytes.push(bytes),
            ProgressEvent::Complete => saw_complete = true,
            _ => {}
        }
    }
    assert!(!downloaded_bytes.is_empty(), "expected at least one progress tick");
    assert!(
        downloaded_bytes.iter().all(|&b| b > 0),
        "progress must report non-zero bytes"
    );
    assert!(saw_complete, "on_complete must fire once all chunks are stored");
}

#[tokio::test(flavor = "current_thread")]
async fn upload_tar_cancellation() {
    // Immediate cancellation: is_cancelled returns true on its first call,
    // before the HTTP send runs. The mock never sees a request.
    let http = MockHttpClient::new();
    let up = MockProgressReporter::cancelling_immediately();

    let entries = vec![("000000.enc".to_string(), vec![0xFFu8; 256])];
    let err = crate::upload_tar::upload_chunks_as_tar_in_memory(
        &http,
        &up,
        &test_auth(),
        "file-cancel",
        entries,
    )
    .await
    .expect_err("cancellation must surface as an error");

    assert!(matches!(err, Error::Cancelled));
    assert!(
        http.take_last_tar_body().is_none(),
        "cancelled upload must not have hit the HTTP client"
    );
}
