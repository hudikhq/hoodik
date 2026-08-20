use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};
use futures::stream::{StreamExt, TryStreamExt};
use s3::error::S3Error;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::{
    contract::FsProviderContract,
    filename::{Filename, IntoFilename},
    streamer::Streamer,
    tar,
};

mod bulk_delete;

thread_local! {
    /// Buckets already built on this thread, keyed by the settings that
    /// identify the connection.
    ///
    /// `Bucket::new` reads the whole operating-system trust store to build a
    /// TLS client before it has been asked to talk to anything, and
    /// `Fs::provider()` builds a provider for every storage call — so a single
    /// download used to re-parse every system certificate several times.
    ///
    /// Per thread rather than per process on purpose. The HTTP client inside a
    /// `Bucket` owns a connection pool tied to the runtime it was built on, and
    /// actix gives each worker its own runtime; one shared pool across all of
    /// them is a question this cache has no need to raise. A worker builds one
    /// client and reuses it, which is the whole win.
    static BUCKETS: RefCell<HashMap<BucketKey, s3::Bucket>> = RefCell::new(HashMap::new());
}

/// What distinguishes one bucket connection from another. The key prefix is
/// deliberately absent: it is applied per request when building object keys,
/// never baked into the connection.
#[derive(Clone, PartialEq, Eq, Hash)]
struct BucketKey {
    bucket: String,
    region: String,
    endpoint: Option<String>,
    access_key: String,
    path_style: bool,
}

/// Serialises TLS client construction across the whole process.
///
/// Building an S3 client reads the operating system's trust store. On macOS
/// that goes through Security.framework, which returns `errSecIO` (-36) when
/// several threads reach it at once, and sometimes on the very first call in a
/// process. It surfaces as `io: I/O error` from `Bucket::new`, which reads as
/// an unreachable bucket and is nothing of the sort — the endpoint,
/// credentials and network are all fine.
///
/// Holding a lock here costs nothing in practice: construction is cached per
/// thread, so a worker takes this once and never again.
static BUCKET_CONSTRUCTION: Mutex<()> = Mutex::new(());

/// Backoff between attempts at building the client. Bounded and short: a
/// genuinely broken configuration — wrong region, malformed endpoint — fails
/// the same way every time and should surface in well under a second rather
/// than being retried into a slow death.
const CONSTRUCTION_BACKOFF: [std::time::Duration; 3] = [
    std::time::Duration::from_millis(50),
    std::time::Duration::from_millis(150),
    std::time::Duration::from_millis(400),
];

fn build_bucket(config: &config::s3::S3Config) -> s3::Bucket {
    let region = match &config.endpoint {
        Some(endpoint) => s3::Region::Custom {
            region: config.region.clone(),
            endpoint: endpoint.clone(),
        },
        None => config
            .region
            .parse()
            .expect("Invalid S3 region. Check S3_REGION configuration."),
    };

    let credentials = s3::creds::Credentials::new(
        Some(&config.access_key),
        Some(&config.secret_key),
        None,
        None,
        None,
    )
    .expect("Invalid S3 credentials. Check S3_ACCESS_KEY and S3_SECRET_KEY.");

    let _serialised = BUCKET_CONSTRUCTION
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut last_error = None;
    let mut built = None;

    for (attempt, backoff) in CONSTRUCTION_BACKOFF.iter().enumerate() {
        match s3::Bucket::new(&config.bucket, region.clone(), credentials.clone()) {
            Ok(bucket) => {
                built = Some(bucket);
                break;
            }
            Err(e) => {
                log::debug!(
                    "S3 client construction attempt {} failed ({e:?})",
                    attempt + 1
                );
                last_error = Some(e);
                std::thread::sleep(*backoff);
            }
        }
    }

    // Debug, not Display: `S3Error`'s Display renders an underlying `io::Error`
    // as the bare string "I/O error", which tells an operator nothing about
    // whether their endpoint, credentials or trust store is at fault.
    let mut bucket = built.unwrap_or_else(|| {
        panic!(
            "Failed to create S3 bucket handle for '{}' after {} attempts: {:?}",
            &config.bucket,
            CONSTRUCTION_BACKOFF.len(),
            last_error
        )
    });

    if config.path_style {
        bucket.set_path_style();
    }

    *bucket
}

pub struct S3Provider {
    bucket: s3::Bucket,
    prefix: String,
    direct_transfer: bool,
    direct_expiry_secs: u32,
}

impl S3Provider {
    pub fn new(config: &config::s3::S3Config) -> Self {
        let key = BucketKey {
            bucket: config.bucket.clone(),
            region: config.region.clone(),
            endpoint: config.endpoint.clone(),
            access_key: config.access_key.clone(),
            path_style: config.path_style,
        };

        let bucket = BUCKETS.with(|buckets| {
            if let Some(bucket) = buckets.borrow().get(&key) {
                return bucket.clone();
            }

            let bucket = build_bucket(config);
            buckets.borrow_mut().insert(key, bucket.clone());
            bucket
        });

        Self {
            bucket,
            prefix: config.prefix.clone().unwrap_or_default(),
            direct_transfer: config.direct_transfer,
            direct_expiry_secs: config.direct_expiry_secs,
        }
    }

    /// Whether a URL handed to a client would actually work.
    ///
    /// The flag says the operator asked for direct transfer; the startup probe
    /// says whether the bucket can serve it — reachable over a transport a
    /// client accepts, with a CORS policy that lets the page read the answer.
    /// Signing on the flag alone minted URLs for a bucket no client could use,
    /// which the app then failed every chunk against with nothing to fall back
    /// to. Both have to agree, and the routes 400 when they do not, exactly as
    /// their documentation promises.
    fn direct_enabled(&self) -> bool {
        self.direct_transfer && config::direct::verdict().enabled
    }

    pub fn bucket(&self) -> &s3::Bucket {
        &self.bucket
    }

    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    fn object_key(&self, filename: &Filename) -> String {
        format!("{}{}", self.prefix, filename)
    }

    /// Key prefix used to list *legacy* chunks for a file. Matches the
    /// timestamped flat layout (`{prefix}{timestamp}-{uuid}.part.N`).
    fn chunk_prefix(&self, filename: &Filename) -> String {
        format!("{}{}.part.", self.prefix, filename)
    }

    /// Prefix of all keys belonging to a single version:
    /// `{prefix}{inner_name}/v{N}/`.
    fn version_prefix(&self, filename: &Filename, version: i32) -> String {
        format!("{}{}/v{}/", self.prefix, filename.inner_name(), version)
    }

    /// Full key of one versioned chunk:
    /// `{prefix}{inner_name}/v{N}/{chunk:06}.chunk`.
    fn versioned_chunk_key(&self, filename: &Filename, version: i32, chunk: i64) -> String {
        format!("{}{:06}.chunk", self.version_prefix(filename, version), chunk)
    }

    /// Prefix covering every version and legacy-versioned key for a file:
    /// `{prefix}{inner_name}/`. Used to nuke the full versioned tree on
    /// `purge_all`.
    fn file_root_prefix(&self, filename: &Filename) -> String {
        format!("{}{}/", self.prefix, filename.inner_name())
    }

    /// Parse chunk index from a legacy S3 key of the form
    /// `{prefix}{timestamp}-{uuid}.part.{chunk}`.
    fn parse_chunk_index(key: &str) -> AppResult<i64> {
        let cleaned = key.replace(".part", "");
        cleaned
            .rsplit('.')
            .next()
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| {
                Error::InternalError(format!(
                    "Failed to parse chunk number from S3 key: {}",
                    key
                ))
            })
    }

    /// Parse chunk index from a versioned S3 key ending in `{idx:06}.chunk`.
    /// The leading path portion is ignored; only the trailing filename
    /// matters, so pagination/prefix tricks don't affect parsing.
    fn parse_versioned_chunk_index(key: &str) -> AppResult<i64> {
        let tail = key.rsplit('/').next().unwrap_or(key);
        let stem = tail.strip_suffix(".chunk").ok_or_else(|| {
            Error::InternalError(format!("Unexpected versioned chunk key: {}", key))
        })?;
        stem.parse::<i64>().map_err(|_| {
            Error::InternalError(format!(
                "Failed to parse chunk number from versioned key: {}",
                key
            ))
        })
    }

    async fn list_objects(&self, prefix: &str) -> AppResult<Vec<s3::serde_types::Object>> {
        let results = self
            .bucket
            .list(prefix.to_string(), None)
            .await
            .map_err(|e| Error::StorageError(format!("S3 list objects failed: {}", e)))?;

        let mut objects = Vec::new();
        for result in results {
            objects.extend(result.contents);
        }

        Ok(objects)
    }

    /// Consolidated 404/NoSuchKey detection. `rust-s3` surfaces these as
    /// stringly-typed errors, so every versioned read path needs the same
    /// check — keep it in one place.
    fn is_not_found(err: &S3Error) -> bool {
        let s = err.to_string();
        s.contains("404") || s.contains("NoSuchKey") || s.contains("Not Found")
    }

    /// Sign a URL that reads one object. Pure local HMAC — no round trip to
    /// the store — so signing a whole file's worth costs microseconds.
    pub(crate) async fn presign_get(&self, key: &str) -> AppResult<String> {
        self.bucket
            .presign_get(key, self.direct_expiry_secs, None)
            .await
            .map_err(|e| {
                Error::StorageError(format!("S3 presign_get failed for '{}': {}", key, e))
            })
    }

    /// Sign a URL that writes one object of exactly `len` bytes.
    ///
    /// `content-length` goes in as a signed header, which binds it into the
    /// signature: a client that sends a different number of bytes fails the
    /// signature check at the store. Without it a presigned write would be
    /// an unbounded one, since nothing of ours sits in front of it.
    async fn presign_put(&self, key: &str, len: u64) -> AppResult<String> {
        let mut headers = http::HeaderMap::new();
        headers.insert(
            http::header::CONTENT_LENGTH,
            http::HeaderValue::from_str(&len.to_string()).map_err(|e| {
                Error::InternalError(format!("Invalid content-length for '{}': {}", key, e))
            })?,
        );

        self.bucket
            .presign_put(key, self.direct_expiry_secs, Some(headers), None)
            .await
            .map_err(|e| {
                Error::StorageError(format!("S3 presign_put failed for '{}': {}", key, e))
            })
    }

    /// True when `version == 1` and the versioned directory is empty. Used
    /// by every read-side `_v` method to transparently fall back to the
    /// legacy flat layout for pre-migration files.
    async fn should_use_legacy(&self, filename: &Filename, version: i32) -> AppResult<bool> {
        if version != 1 {
            return Ok(false);
        }
        let prefix = self.version_prefix(filename, 1);
        let objects = self.list_objects(&prefix).await?;
        Ok(!objects.iter().any(|o| o.key.ends_with(".chunk")))
    }
}

#[async_trait]
impl FsProviderContract for S3Provider {
    async fn available_space(&self) -> AppResult<u64> {
        Ok(u64::MAX)
    }

    async fn health_check(&self) -> AppResult<()> {
        // A HEAD on any key is one cheap round-trip that exercises the
        // credentials and bucket reachability; a 404 (key absent) is still a
        // successful response, only an auth/connectivity failure surfaces as
        // an error.
        let key = format!("{}.hoodik-readiness", self.prefix);
        head_size(&self.bucket, &key).await.map(|_| ())
    }

    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>> {
        let key = self.object_key(&filename.filename()?);
        get_object_bytes(&self.bucket, &key).await
    }

    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()> {
        let key = self.object_key(&filename.filename()?);
        put_object_checked(&self.bucket, &key, data).await
    }

    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool> {
        let key = self.object_key(&filename.filename()?.with_chunk(chunk));
        head_size(&self.bucket, &key).await.map(|s| s.is_some())
    }

    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()> {
        let key = self.object_key(&filename.filename()?.with_chunk(chunk));
        put_object_checked(&self.bucket, &key, data).await
    }

    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<Vec<u8>> {
        let key = self.object_key(&filename.filename()?.with_chunk(chunk));
        get_object_bytes(&self.bucket, &key).await
    }

    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        let prefix = self.chunk_prefix(&filename.filename()?);
        let objects = self.list_objects(&prefix).await?;

        if objects.is_empty() {
            return Ok(());
        }

        let keys: Vec<String> = objects.into_iter().map(|o| o.key).collect();
        bulk_delete::delete_keys(&self.bucket, keys).await
    }

    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i64>> {
        let prefix = self.chunk_prefix(&filename.filename()?);
        let objects = self.list_objects(&prefix).await?;

        let mut chunks = Vec::new();
        for object in objects {
            chunks.push(Self::parse_chunk_index(&object.key)?);
        }

        chunks.sort();
        Ok(chunks)
    }

    async fn stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> AppResult<Streamer> {
        let filename = filename.filename()?;

        let chunks_to_stream: Vec<i64> = match chunk {
            Some(c) => {
                if !self.exists(&filename, c).await? {
                    return Err(Error::NotFound("chunk_not_found".to_string()));
                }
                vec![c]
            }
            None => self.get_uploaded_chunks(&filename).await?,
        };

        let mut keys: Vec<String> = chunks_to_stream
            .into_iter()
            .map(|c| self.object_key(&filename.clone().with_chunk(c)))
            .collect();

        keys.reverse();

        Ok(Streamer::new(chunk_key_stream(self.bucket.clone(), keys)))
    }

    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer> {
        let filename = filename.filename()?;
        let chunks = self.get_uploaded_chunks(&filename).await?;

        let entries: Vec<(String, String)> = chunks
            .iter()
            .map(|idx| {
                let key = self.object_key(&filename.clone().with_chunk(*idx));
                let name = format!("{:06}.enc", idx);
                (name, key)
            })
            .collect();

        Ok(Streamer::new(tar_entry_stream(self.bucket.clone(), entries)))
    }

    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        let filename = filename.filename()?;
        let chunks = self.get_uploaded_chunks(&filename).await?;

        let keys: Vec<String> = chunks
            .iter()
            .map(|idx| self.object_key(&filename.clone().with_chunk(*idx)))
            .collect();

        tar_total_length(&self.bucket, keys).await
    }

    // ── Versioned chunk operations ──────────────────────────────────────

    async fn push_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
        data: &[u8],
    ) -> AppResult<()> {
        let key = self.versioned_chunk_key(&filename.filename()?, version, chunk);
        put_object_checked(&self.bucket, &key, data).await
    }

    async fn pull_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
    ) -> AppResult<Vec<u8>> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.pull(&filename, chunk).await;
        }

        let key = self.versioned_chunk_key(&filename, version, chunk);
        get_object_bytes(&self.bucket, &key).await
    }

    async fn exists_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
    ) -> AppResult<bool> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.exists(&filename, chunk).await;
        }

        let key = self.versioned_chunk_key(&filename, version, chunk);
        head_size(&self.bucket, &key).await.map(|s| s.is_some())
    }

    async fn get_uploaded_chunks_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<Vec<i64>> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.get_uploaded_chunks(&filename).await;
        }

        let prefix = self.version_prefix(&filename, version);
        let objects = self.list_objects(&prefix).await?;

        let mut chunks = Vec::with_capacity(objects.len());
        for o in objects {
            if o.key.ends_with(".chunk") {
                chunks.push(Self::parse_versioned_chunk_index(&o.key)?);
            }
        }
        chunks.sort();
        Ok(chunks)
    }

    async fn stream_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: Option<i64>,
    ) -> AppResult<Streamer> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.stream(&filename, chunk).await;
        }

        // Probe the key directly rather than via `exists_v`: the legacy
        // question is already settled above, and `exists_v` would re-ask it
        // with another LIST.
        let chunk_indices: Vec<i64> = match chunk {
            Some(c) => {
                let key = self.versioned_chunk_key(&filename, version, c);
                if head_size(&self.bucket, &key).await?.is_none() {
                    return Err(Error::NotFound("chunk_not_found".to_string()));
                }
                vec![c]
            }
            None => self.get_uploaded_chunks_v(&filename, version).await?,
        };

        let mut keys: Vec<String> = chunk_indices
            .into_iter()
            .map(|c| self.versioned_chunk_key(&filename, version, c))
            .collect();
        keys.reverse();

        Ok(Streamer::new(chunk_key_stream(self.bucket.clone(), keys)))
    }

    async fn stream_tar_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<Streamer> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.stream_tar(&filename).await;
        }

        let chunks = self.get_uploaded_chunks_v(&filename, version).await?;
        let entries: Vec<(String, String)> = chunks
            .iter()
            .map(|idx| {
                let key = self.versioned_chunk_key(&filename, version, *idx);
                let name = format!("{:06}.enc", idx);
                (name, key)
            })
            .collect();

        Ok(Streamer::new(tar_entry_stream(self.bucket.clone(), entries)))
    }

    async fn tar_content_length_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<u64> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await? {
            return self.tar_content_length(&filename).await;
        }

        let chunks = self.get_uploaded_chunks_v(&filename, version).await?;
        let keys: Vec<String> = chunks
            .iter()
            .map(|idx| self.versioned_chunk_key(&filename, version, *idx))
            .collect();

        tar_total_length(&self.bucket, keys).await
    }

    async fn purge_version<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<()> {
        let prefix = self.version_prefix(&filename.filename()?, version);
        let objects = self.list_objects(&prefix).await?;

        if objects.is_empty() {
            return Ok(());
        }

        let keys: Vec<String> = objects.into_iter().map(|o| o.key).collect();
        bulk_delete::delete_keys(&self.bucket, keys).await
    }

    async fn copy_version<S: IntoFilename, D: IntoFilename>(
        &self,
        src: &S,
        src_version: i32,
        dst: &D,
        dst_version: i32,
    ) -> AppResult<()> {
        let src = src.filename()?;
        let dst = dst.filename()?;

        let src_is_legacy = self.should_use_legacy(&src, src_version).await?;

        // Enumerate source chunks in the right layout.
        let (src_keys, src_indices): (Vec<String>, Vec<i64>) = if src_is_legacy {
            let prefix = self.chunk_prefix(&src);
            let mut pairs: Vec<(String, i64)> = self
                .list_objects(&prefix)
                .await?
                .into_iter()
                .map(|o| {
                    let idx = Self::parse_chunk_index(&o.key)?;
                    Ok::<_, Error>((o.key, idx))
                })
                .collect::<AppResult<Vec<_>>>()?;
            pairs.sort_by_key(|(_, i)| *i);
            pairs.into_iter().unzip()
        } else {
            let prefix = self.version_prefix(&src, src_version);
            let mut pairs: Vec<(String, i64)> = self
                .list_objects(&prefix)
                .await?
                .into_iter()
                .filter(|o| o.key.ends_with(".chunk"))
                .map(|o| {
                    let idx = Self::parse_versioned_chunk_index(&o.key)?;
                    Ok::<_, Error>((o.key, idx))
                })
                .collect::<AppResult<Vec<_>>>()?;
            pairs.sort_by_key(|(_, i)| *i);
            pairs.into_iter().unzip()
        };

        if src_keys.is_empty() {
            return Ok(());
        }

        // Defensive: refuse to copy a chunk larger than the CopyObject
        // single-op limit. MAX_CHUNK_SIZE_BYTES is statically under this
        // at the library level, but data written by an older server —
        // or manually prepared fixtures — could in theory exceed it.
        // Multipart-copy isn't wired up, so surface a clear error.
        const COPY_OBJECT_MAX_BYTES: u64 = 5 * 1024 * 1024 * 1024;
        for key in &src_keys {
            let size = required_chunk_size(&self.bucket, key).await?;
            if size > COPY_OBJECT_MAX_BYTES {
                return Err(Error::InternalError(format!(
                    "S3 copy_version source chunk '{}' is {} bytes, \
                     exceeding the CopyObject single-op limit of 5 GiB",
                    key, size
                )));
            }
        }

        let dst_keys: Vec<String> = src_indices
            .iter()
            .map(|idx| self.versioned_chunk_key(&dst, dst_version, *idx))
            .collect();

        let bucket = self.bucket.clone();
        futures::stream::iter(src_keys.into_iter().zip(dst_keys))
            .map(|(src_key, dst_key)| {
                let bucket = bucket.clone();
                async move {
                    let status = bucket
                        .copy_object_internal(&src_key, &dst_key)
                        .await
                        .map_err(|e| {
                            Error::StorageError(format!(
                                "S3 copy_object failed for '{}' -> '{}': {}",
                                src_key, dst_key, e
                            ))
                        })?;
                    if !(200..300).contains(&status) {
                        return Err(Error::StorageError(format!(
                            "S3 copy_object for '{}' -> '{}' returned status {}",
                            src_key, dst_key, status
                        )));
                    }
                    Ok::<(), Error>(())
                }
            })
            .buffer_unordered(8)
            .try_collect::<Vec<()>>()
            .await?;

        Ok(())
    }

    async fn purge_all<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        let filename = filename.filename()?;

        // Drop the whole versioned tree under `{prefix}{inner_name}/` first,
        // then fall through to the legacy flat-key purge so pre-migration
        // files with no `v{N}/` objects still get cleaned up.
        let versioned = self.list_objects(&self.file_root_prefix(&filename)).await?;
        if !versioned.is_empty() {
            let keys: Vec<String> = versioned.into_iter().map(|o| o.key).collect();
            bulk_delete::delete_keys(&self.bucket, keys).await?;
        }

        self.purge(&filename).await
    }

    async fn direct_get_urls<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunks: &[i64],
    ) -> AppResult<Option<Vec<String>>> {
        if !self.direct_enabled() {
            return Ok(None);
        }

        let filename = filename.filename()?;
        // One probe for the whole set. Asking per chunk would turn a
        // manifest into a LIST storm.
        let legacy = self.should_use_legacy(&filename, version).await?;

        let mut urls = Vec::with_capacity(chunks.len());
        for chunk in chunks {
            let key = if legacy {
                self.object_key(&filename.clone().with_chunk(*chunk))
            } else {
                self.versioned_chunk_key(&filename, version, *chunk)
            };
            urls.push(self.presign_get(&key).await?);
        }

        Ok(Some(urls))
    }

    async fn direct_put_urls<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunks: &[(i64, u64)],
    ) -> AppResult<Option<Vec<String>>> {
        if !self.direct_enabled() {
            return Ok(None);
        }

        let filename = filename.filename()?;
        // The same probe the read side runs, for the same reason: the layout
        // belongs to the file, not to how a particular chunk happened to be
        // written. Signing versioned keys unconditionally put a direct upload
        // somewhere `get_uploaded_chunks` never looks, so every non-editable
        // file finalized as `chunks_missing` however well the writes went.
        let legacy = self.should_use_legacy(&filename, version).await?;

        let mut urls = Vec::with_capacity(chunks.len());
        for (chunk, len) in chunks {
            let key = if legacy {
                self.object_key(&filename.clone().with_chunk(*chunk))
            } else {
                self.versioned_chunk_key(&filename, version, *chunk)
            };
            urls.push(self.presign_put(&key, *len).await?);
        }

        Ok(Some(urls))
    }
}

/// Build a lazy stream that fetches each S3 key one at a time and emits the
/// bytes. Keys are consumed in order (the caller reverses ahead of time so
/// they can `pop()` from the end).
fn chunk_key_stream(
    bucket: s3::Bucket,
    keys: Vec<String>,
) -> impl futures_util::Stream<Item = AppResult<Bytes>> {
    futures_util::stream::unfold(
        (bucket, keys),
        |(bucket, mut keys)| async move {
            let key = keys.pop()?;
            match get_object_bytes(&bucket, &key).await {
                Ok(data) => Some((Ok(Bytes::from(data)), (bucket, keys))),
                Err(e) => {
                    log::error!("S3 stream read failed for '{}': {}", key, e);
                    Some((Err(e), (bucket, keys)))
                }
            }
        },
    )
}

/// Build a lazy tar stream over a list of (entry_name, s3_key) pairs. Each
/// entry fetches its body on-demand and emits the 512-byte header, the
/// payload, any 512-byte padding, and finally the two-block end-of-archive
/// marker.
fn tar_entry_stream(
    bucket: s3::Bucket,
    entries: Vec<(String, String)>,
) -> impl futures_util::Stream<Item = AppResult<Bytes>> {
    enum Phase {
        NextEntry,
        Data(Vec<u8>),
        Padding(usize),
        EndOfArchive,
        Done,
    }
    struct State {
        entries: Vec<(String, String)>,
        phase: Phase,
        bucket: s3::Bucket,
    }

    let mut entries = entries;
    entries.reverse();

    let state = State {
        entries,
        phase: Phase::NextEntry,
        bucket,
    };

    futures_util::stream::unfold(state, |mut state| async move {
        loop {
            match state.phase {
                Phase::NextEntry => {
                    if let Some((name, key)) = state.entries.pop() {
                        let data = match get_object_bytes(&state.bucket, &key).await {
                            Ok(d) => d,
                            Err(e) => return Some((Err(e), state)),
                        };

                        let size = data.len() as u64;
                        let header = tar::tar_header(&name, size);
                        state.phase = Phase::Data(data);
                        return Some((Ok(Bytes::from(header.to_vec())), state));
                    } else {
                        state.phase = Phase::EndOfArchive;
                    }
                }
                Phase::Data(data) => {
                    let size = data.len() as u64;
                    let padding_len = tar::tar_padding_len(size);
                    state.phase = if padding_len > 0 {
                        Phase::Padding(padding_len)
                    } else {
                        Phase::NextEntry
                    };
                    return Some((Ok(Bytes::from(data)), state));
                }
                Phase::Padding(len) => {
                    state.phase = Phase::NextEntry;
                    return Some((Ok(Bytes::from(vec![0u8; len])), state));
                }
                Phase::EndOfArchive => {
                    state.phase = Phase::Done;
                    return Some((
                        Ok(Bytes::from(vec![0u8; tar::TAR_END_OF_ARCHIVE_LEN])),
                        state,
                    ));
                }
                Phase::Done => return None,
            }
        }
    })
}

/// Accumulate tar total size across a list of S3 keys by `HEAD`-ing each one
/// and summing header + payload + padding, plus the two-block trailer.
async fn tar_total_length(bucket: &s3::Bucket, keys: Vec<String>) -> AppResult<u64> {
    let mut total: u64 = 0;
    for key in &keys {
        let size = required_chunk_size(bucket, key).await?;
        total += 512 + size + tar::tar_padding_len(size) as u64;
    }
    total += tar::TAR_END_OF_ARCHIVE_LEN as u64;
    Ok(total)
}

/// `GET` an object and translate rust-s3's "return the status code inside
/// ResponseData" behaviour into a proper error for 404s. `rust-s3` 0.35 is
/// built without `fail-on-err`, so any non-2xx response comes back as
/// `Ok(ResponseData { status_code, … })` — we have to check manually.
async fn get_object_bytes(bucket: &s3::Bucket, key: &str) -> AppResult<Vec<u8>> {
    let response = bucket.get_object(key).await.map_err(|e| {
        if S3Provider::is_not_found(&e) {
            Error::NotFound(format!("S3 chunk not found: {} ({})", key, e))
        } else {
            Error::StorageError(format!("S3 get_object failed for '{}': {}", key, e))
        }
    })?;
    let status = response.status_code();
    if (200..300).contains(&status) {
        Ok(response.to_vec())
    } else if status == 404 {
        Err(Error::NotFound(format!("S3 chunk not found: {}", key)))
    } else {
        Err(Error::StorageError(format!(
            "S3 get_object for '{}' returned status {}",
            key, status
        )))
    }
}

/// `PUT` an object, failing on any non-2xx. Without this a rejected write
/// (quota, policy, expired credentials) reports success to the caller, which
/// then records a chunk that isn't in the bucket.
async fn put_object_checked(bucket: &s3::Bucket, key: &str, data: &[u8]) -> AppResult<()> {
    let response = bucket
        .put_object(key, data)
        .await
        .map_err(|e| Error::StorageError(format!("S3 put_object failed for '{}': {}", key, e)))?;
    let status = response.status_code();
    if (200..300).contains(&status) {
        Ok(())
    } else {
        Err(Error::StorageError(format!(
            "S3 put_object for '{}' returned status {}",
            key, status
        )))
    }
}

/// `HEAD` an object, returning its size or `None` when the key is absent.
/// `rust-s3` reports a 404 either as the status inside an `Ok` tuple or as a
/// stringly-typed `Err`, depending on the backend; both mean absence.
async fn head_size(bucket: &s3::Bucket, key: &str) -> AppResult<Option<u64>> {
    match bucket.head_object(key).await {
        Ok((head, status)) if (200..300).contains(&status) => {
            Ok(Some(head.content_length.unwrap_or(0) as u64))
        }
        Ok((_, 404)) => Ok(None),
        Ok((_, status)) => Err(Error::StorageError(format!(
            "S3 head_object for '{}' returned unexpected status {}",
            key, status
        ))),
        Err(e) if S3Provider::is_not_found(&e) => Ok(None),
        Err(e) => Err(Error::StorageError(format!(
            "S3 head_object failed for '{}': {}",
            key, e
        ))),
    }
}

/// Size of a chunk that must be there. Callers summing sizes would otherwise
/// under-report a missing key as a zero-length object.
async fn required_chunk_size(bucket: &s3::Bucket, key: &str) -> AppResult<u64> {
    head_size(bucket, key)
        .await?
        .ok_or_else(|| Error::NotFound(format!("S3 chunk not found: {}", key)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_legacy_chunk_index() {
        assert_eq!(
            S3Provider::parse_chunk_index(
                "1712345600-550e8400-e29b-41d4-a716-446655440000.part.0"
            )
            .unwrap(),
            0
        );
        assert_eq!(
            S3Provider::parse_chunk_index(
                "prefix/1712345600-550e8400-e29b-41d4-a716-446655440000.part.42"
            )
            .unwrap(),
            42
        );
        assert_eq!(
            S3Provider::parse_chunk_index(
                "hoodik/1712345600-550e8400-e29b-41d4-a716-446655440000.part.100"
            )
            .unwrap(),
            100
        );
    }

    #[test]
    fn parse_legacy_chunk_index_error() {
        assert!(S3Provider::parse_chunk_index("invalid-key-no-part").is_err());
    }

    #[test]
    fn parse_versioned_chunk_index_ok() {
        assert_eq!(
            S3Provider::parse_versioned_chunk_index("abc-uuid/v3/000042.chunk").unwrap(),
            42
        );
        assert_eq!(
            S3Provider::parse_versioned_chunk_index("hoodik/abc-uuid/v1/000000.chunk").unwrap(),
            0
        );
    }

    #[test]
    fn parse_versioned_chunk_index_rejects_non_chunk() {
        assert!(S3Provider::parse_versioned_chunk_index("abc/v1/000000.part").is_err());
    }
}

#[cfg(all(test, feature = "s3-integration-tests"))]
mod s3_versioned_tests;
