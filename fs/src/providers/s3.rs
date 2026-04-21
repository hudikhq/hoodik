use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};
use futures::stream::{StreamExt, TryStreamExt};
use s3::error::S3Error;

use crate::{
    contract::FsProviderContract,
    filename::{Filename, IntoFilename},
    streamer::Streamer,
    tar,
};

mod bulk_delete;

pub struct S3Provider {
    bucket: s3::Bucket,
    prefix: String,
}

impl S3Provider {
    pub fn new(config: &config::s3::S3Config) -> Self {
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

        let mut bucket = s3::Bucket::new(&config.bucket, region, credentials)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to create S3 bucket handle for '{}': {}",
                    &config.bucket, e
                )
            });

        if config.path_style {
            bucket.set_path_style();
        }

        Self {
            bucket: *bucket,
            prefix: config.prefix.clone().unwrap_or_default(),
        }
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

    /// True when `version == 1` and the versioned directory is empty. Used
    /// by every read-side `_v` method to transparently fall back to the
    /// legacy flat layout for pre-migration files.
    async fn should_use_legacy(&self, filename: &Filename, version: i32) -> bool {
        if version != 1 {
            return false;
        }
        let prefix = self.version_prefix(filename, 1);
        match self.list_objects(&prefix).await {
            Ok(objects) => !objects.iter().any(|o| o.key.ends_with(".chunk")),
            Err(_) => true,
        }
    }
}

#[async_trait]
impl FsProviderContract for S3Provider {
    async fn available_space(&self) -> AppResult<u64> {
        Ok(u64::MAX)
    }

    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>> {
        let key = self.object_key(&filename.filename()?);

        let response = self
            .bucket
            .get_object(&key)
            .await
            .map_err(|e| Error::StorageError(format!("S3 read failed for '{}': {}", key, e)))?;

        Ok(response.to_vec())
    }

    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()> {
        let key = self.object_key(&filename.filename()?);

        self.bucket
            .put_object(&key, data)
            .await
            .map_err(|e| Error::StorageError(format!("S3 write failed for '{}': {}", key, e)))?;

        Ok(())
    }

    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool> {
        let key = self.object_key(&filename.filename()?.with_chunk(chunk));
        head_exists(&self.bucket, &key).await
    }

    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()> {
        let key = self.object_key(&filename.filename()?.with_chunk(chunk));

        self.bucket
            .put_object(&key, data)
            .await
            .map_err(|e| Error::StorageError(format!("S3 push failed for '{}': {}", key, e)))?;

        Ok(())
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
            Some(c) => vec![c],
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
        self.bucket.put_object(&key, data).await.map_err(|e| {
            Error::StorageError(format!("S3 push_v failed for '{}': {}", key, e))
        })?;
        Ok(())
    }

    async fn pull_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
    ) -> AppResult<Vec<u8>> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await {
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
        if self.should_use_legacy(&filename, version).await {
            return self.exists(&filename, chunk).await;
        }

        let key = self.versioned_chunk_key(&filename, version, chunk);
        head_exists(&self.bucket, &key).await
    }

    async fn get_uploaded_chunks_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<Vec<i64>> {
        let filename = filename.filename()?;
        if self.should_use_legacy(&filename, version).await {
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
        if self.should_use_legacy(&filename, version).await {
            return self.stream(&filename, chunk).await;
        }

        let chunk_indices: Vec<i64> = match chunk {
            Some(c) => vec![c],
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
        if self.should_use_legacy(&filename, version).await {
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
        if self.should_use_legacy(&filename, version).await {
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

        let src_is_legacy = self.should_use_legacy(&src, src_version).await;

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
        const COPY_OBJECT_MAX_BYTES: i64 = 5 * 1024 * 1024 * 1024;
        for key in &src_keys {
            let (head, _) = self.bucket.head_object(key).await.map_err(|e| {
                Error::StorageError(format!("S3 head_object failed for '{}': {}", key, e))
            })?;
            if let Some(size) = head.content_length {
                if size > COPY_OBJECT_MAX_BYTES {
                    return Err(Error::InternalError(format!(
                        "S3 copy_version source chunk '{}' is {} bytes, \
                         exceeding the CopyObject single-op limit of 5 GiB",
                        key, size
                    )));
                }
            }
        }

        let dst_keys: Vec<String> = src_indices
            .iter()
            .map(|idx| self.versioned_chunk_key(&dst, dst_version, *idx))
            .collect();

        let bucket = self.bucket.clone();
        futures::stream::iter(src_keys.into_iter().zip(dst_keys.into_iter()))
            .map(|(src_key, dst_key)| {
                let bucket = bucket.clone();
                async move {
                    bucket
                        .copy_object_internal(&src_key, &dst_key)
                        .await
                        .map_err(|e| {
                            Error::StorageError(format!(
                                "S3 copy_object failed for '{}' -> '{}': {}",
                                src_key, dst_key, e
                            ))
                        })?;
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
            match bucket.get_object(&key).await {
                Ok(response) => Some((Ok(Bytes::from(response.to_vec())), (bucket, keys))),
                Err(e) => {
                    let err = Error::StorageError(format!(
                        "S3 stream read failed for '{}': {}",
                        key, e
                    ));
                    log::error!("{}", err);
                    Some((Err(err), (bucket, keys)))
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
                        let response = match state.bucket.get_object(&key).await {
                            Ok(r) => r,
                            Err(e) => {
                                return Some((
                                    Err(Error::StorageError(format!(
                                        "S3 tar stream failed for '{}': {}",
                                        key, e
                                    ))),
                                    state,
                                ));
                            }
                        };

                        let data = response.to_vec();
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
        let (head, status) = bucket.head_object(key).await.map_err(|e| {
            Error::StorageError(format!("S3 head_object failed for '{}': {}", key, e))
        })?;
        if status == 404 {
            return Err(Error::NotFound(format!(
                "S3 chunk not found for tar length: {}",
                key
            )));
        }
        let size = head.content_length.unwrap_or(0) as u64;
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

/// `HEAD` an object and translate rust-s3's "return the status code in the
/// Ok tuple" behaviour into a plain bool. A 404 is absence; 2xx is presence;
/// anything else surfaces as an error.
async fn head_exists(bucket: &s3::Bucket, key: &str) -> AppResult<bool> {
    match bucket.head_object(key).await {
        Ok((_, status)) => {
            if (200..300).contains(&status) {
                Ok(true)
            } else if status == 404 {
                Ok(false)
            } else {
                Err(Error::StorageError(format!(
                    "S3 head_object for '{}' returned unexpected status {}",
                    key, status
                )))
            }
        }
        Err(e) => {
            if S3Provider::is_not_found(&e) {
                Ok(false)
            } else {
                Err(Error::StorageError(format!(
                    "S3 head_object failed for '{}': {}",
                    key, e
                )))
            }
        }
    }
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
