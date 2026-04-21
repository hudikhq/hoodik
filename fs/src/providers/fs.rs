use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};
use fs4::available_space;
use tokio::{
    fs::{copy as fs_copy, create_dir_all, read_dir, remove_dir_all, remove_file, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    contract::FsProviderContract,
    filename::{Filename, IntoFilename},
    streamer::Streamer,
    tar,
};

pub(crate) struct FsProvider<'provider> {
    data_dir: &'provider str,
}

impl<'provider> FsProvider<'provider> {
    pub(crate) fn new(data_dir: &'provider str) -> Self {
        Self { data_dir }
    }

    /// Get full path of a file for the chunk
    fn full_path(&self, filename: &Filename) -> String {
        format!("{}/{}", self.data_dir, filename)
    }

    /// Directory holding all versions of a file: `{data_dir}/{file_id}/`.
    fn file_root(&self, filename: &Filename) -> String {
        format!("{}/{}", self.data_dir, filename.inner_name())
    }

    /// Directory holding chunks of a single version: `{data_dir}/{file_id}/v{N}/`.
    fn version_dir(&self, filename: &Filename, version: i32) -> String {
        format!("{}/v{}", self.file_root(filename), version)
    }

    /// Path of one chunk file under a version: `.../v{N}/{chunk:06}.chunk`.
    fn versioned_chunk_path(&self, filename: &Filename, version: i32, chunk: i64) -> String {
        format!(
            "{}/{:06}.chunk",
            self.version_dir(filename, version),
            chunk
        )
    }

    /// True if a versioned chunk directory exists and contains at least one
    /// `*.chunk` file. Used to decide whether to fall back to the legacy
    /// flat layout for `version == 1` reads.
    async fn versioned_dir_has_chunks(&self, filename: &Filename, version: i32) -> bool {
        let dir = self.version_dir(filename, version);
        let mut entries = match read_dir(&dir).await {
            Ok(e) => e,
            Err(_) => return false,
        };
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".chunk") {
                    return true;
                }
            }
        }
        false
    }

    /// True if the FS layer should serve this read from the legacy flat
    /// layout. Only `version == 1` ever falls back — older versions
    /// definitionally only exist in the new layout (created by edits
    /// after the migration shipped).
    async fn should_use_legacy(&self, filename: &Filename, version: i32) -> bool {
        version == 1 && !self.versioned_dir_has_chunks(filename, version).await
    }

    /// List chunk indices inside a version directory by reading its entries.
    /// Returns empty if the directory is missing.
    async fn list_versioned_chunks(&self, filename: &Filename, version: i32) -> AppResult<Vec<i64>> {
        let dir = self.version_dir(filename, version);
        let mut entries = match read_dir(&dir).await {
            Ok(e) => e,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(Error::from(e)),
        };

        let mut chunks = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name();
            let name = match name.to_str() {
                Some(s) => s,
                None => continue,
            };
            // Files are `{index:06}.chunk` — strip the suffix and parse.
            let stem = match name.strip_suffix(".chunk") {
                Some(s) => s,
                None => continue,
            };
            if let Ok(idx) = stem.parse::<i64>() {
                chunks.push(idx);
            }
        }
        chunks.sort();
        Ok(chunks)
    }

    /// Create the inner streaming method that is then passed into the streamer for
    /// better readability of the code.
    ///
    /// Files are opened lazily one at a time during streaming to avoid exhausting
    /// the OS file descriptor limit for files with many chunks. Only a list of
    /// file paths is built upfront; each path is opened, read, and closed inside
    /// the `unfold` closure so at most one FD is held at a time.
    async fn inner_stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> AppResult<impl futures_util::Stream<Item = AppResult<actix_web::web::Bytes>>> {
        let filename = filename.filename()?;

        let chunk_indices: Vec<i64> = match chunk {
            Some(c) => vec![c],
            None => self.get_uploaded_chunks(&filename).await?,
        };

        let mut paths: Vec<String> = chunk_indices
            .into_iter()
            .map(|c| self.full_path(&filename.clone().with_chunk(c)))
            .collect();

        // Reverse so we can pop from the end in chunk order.
        paths.reverse();

        Ok(futures_util::stream::unfold(
            paths,
            |mut paths: Vec<String>| async move {
                let path = paths.pop()?;

                let mut file = match File::open(&path).await {
                    Ok(f) => f,
                    Err(e) => {
                        log::error!("Failed to open chunk file {}: {:#?}", path, e);
                        return Some((Err(Error::from(e)), paths));
                    }
                };

                let mut data = vec![];
                match file.read_to_end(&mut data).await {
                    Ok(_) => Some((Ok(Bytes::from(data)), paths)),
                    Err(e) => {
                        log::error!("Failed to read chunk file {}: {:#?}", path, e);
                        Some((Err(Error::from(e)), paths))
                    }
                }
            },
        ))
    }

    /// Stream all chunks as an uncompressed tar archive. Each chunk becomes a
    /// named entry (`{chunk_index:06}.enc`) with a proper tar header, data,
    /// and padding. The archive ends with two 512-byte zero blocks.
    ///
    /// Files are opened one at a time during streaming to avoid exhausting the
    /// OS file descriptor limit for files with many chunks.
    async fn inner_stream_tar<T: IntoFilename>(
        &self,
        filename: &T,
    ) -> impl futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> {
        let chunks = match self.get_uploaded_chunks(filename).await {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to get uploaded chunks for tar stream: {:#?}", e);
                vec![]
            }
        };

        // Build (entry_name, on_disk_path) for each chunk.
        let mut entries: Vec<(String, String)> = Vec::with_capacity(chunks.len());
        for chunk_idx in &chunks {
            match filename.filename() {
                Ok(f) => {
                    let chunk_filename = f.with_chunk(*chunk_idx);
                    let path = self.full_path(&chunk_filename);
                    let name = format!("{:06}.enc", chunk_idx);
                    entries.push((name, path));
                }
                Err(e) => {
                    log::error!("Failed to build filename for chunk {}: {:#?}", chunk_idx, e);
                }
            }
        }

        // Reverse so we can pop from the end in order.
        entries.reverse();

        /// Phases of the tar entry state machine.
        enum Phase {
            /// Pop next entry, open its file, and emit the tar header.
            NextEntry,
            /// Read the chunk file data and emit it.
            Data(File, u64),
            /// Emit zero-padding to reach a 512-byte boundary.
            Padding(usize),
            /// Emit the end-of-archive marker (1024 zero bytes).
            EndOfArchive,
            /// Stream is finished.
            Done,
        }

        struct State {
            entries: Vec<(String, String)>,
            phase: Phase,
        }

        let state = State {
            entries,
            phase: Phase::NextEntry,
        };

        futures_util::stream::unfold(state, |mut state| async move {
            loop {
                match state.phase {
                    Phase::NextEntry => {
                        if let Some((name, path)) = state.entries.pop() {
                            let file = match File::open(&path).await {
                                Ok(f) => f,
                                Err(e) => {
                                    log::error!("Failed to open chunk {}: {:#?}", name, e);
                                    return Some((Err(Error::from(e)), state));
                                }
                            };
                            let size = match file.metadata().await {
                                Ok(m) => m.len(),
                                Err(e) => {
                                    log::error!("Failed to stat chunk {}: {:#?}", name, e);
                                    return Some((Err(Error::from(e)), state));
                                }
                            };
                            let header = tar::tar_header(&name, size);
                            state.phase = Phase::Data(file, size);
                            return Some((Ok(Bytes::from(header.to_vec())), state));
                        } else {
                            state.phase = Phase::EndOfArchive;
                        }
                    }
                    Phase::Data(mut file, size) => {
                        let mut data = Vec::with_capacity(size as usize);
                        match file.read_to_end(&mut data).await {
                            Ok(_) => {
                                let padding_len = tar::tar_padding_len(size);
                                state.phase = if padding_len > 0 {
                                    Phase::Padding(padding_len)
                                } else {
                                    Phase::NextEntry
                                };
                                return Some((Ok(Bytes::from(data)), state));
                            }
                            Err(e) => {
                                state.phase = Phase::NextEntry;
                                return Some((Err(Error::from(e)), state));
                            }
                        }
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
                    Phase::Done => {
                        return None;
                    }
                }
            }
        })
    }

    /// Get a read-only file handle for a specific chunk from the local filesystem.
    async fn get<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<File> {
        let filename = filename.filename()?.with_chunk(chunk);

        File::open(self.full_path(&filename))
            .await
            .map_err(Error::from)
    }

    /// Calculate the total tar archive size by statting chunk files in batches.
    async fn inner_tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        const BATCH_SIZE: usize = 50;

        let chunks = self.get_uploaded_chunks(filename).await?;
        let mut total: u64 = 0;

        for batch in chunks.chunks(BATCH_SIZE) {
            for chunk_idx in batch {
                let file = self.get(filename, *chunk_idx).await?;
                let size = file.metadata().await?.len();
                total += 512 + size + tar::tar_padding_len(size) as u64;
                // `file` dropped here — FD released.
            }
        }

        total += tar::TAR_END_OF_ARCHIVE_LEN as u64;
        Ok(total)
    }
}

#[async_trait]
impl FsProviderContract for FsProvider<'_> {
    async fn available_space(&self) -> AppResult<u64> {
        available_space(self.data_dir).map_err(Error::from)
    }

    /// Direct read of the file data
    async fn read<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<u8>> {
        let path = self.full_path(&filename.filename()?);

        let mut file = File::open(path).await?;

        let mut data = vec![];

        file.read_to_end(&mut data).await?;

        Ok(data)
    }

    /// Direct write of the file data
    async fn write<T: IntoFilename>(&self, filename: &T, data: &[u8]) -> AppResult<()> {
        let filename = filename.filename()?;

        let file = File::create(self.full_path(&filename)).await?;

        let mut writer = tokio::io::BufWriter::new(file);
        writer.write_all(data).await?;
        writer.flush().await?;

        Ok(())
    }

    async fn exists<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<bool> {
        Ok(std::path::Path::new(
            self.full_path(&filename.filename()?.with_chunk(chunk))
                .as_str(),
        )
        .exists())
    }

    async fn push<T: IntoFilename>(&self, filename: &T, chunk: i64, data: &[u8]) -> AppResult<()> {
        let filename = filename.filename()?.with_chunk(chunk);

        // Idempotent parent-dir creation — matches `push_v`. The server may
        // race with concurrent deletes (or a fresh install where the data_dir
        // hasn't been touched yet), and `File::create` alone returns ENOENT
        // if the parent is missing.
        create_dir_all(self.data_dir).await?;

        let file = File::create(self.full_path(&filename)).await?;

        let mut writer = tokio::io::BufWriter::new(file);
        writer.write_all(data).await?;
        writer.flush().await?;

        Ok(())
    }

    async fn pull<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<Vec<u8>> {
        let filename = filename.filename()?.with_chunk(chunk);

        let mut file = File::open(self.full_path(&filename)).await?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;

        Ok(buffer)
    }

    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        let filename = filename.filename()?;

        let chunks = self.get_uploaded_chunks(&filename).await?;

        for chunk in chunks {
            remove_file(self.full_path(&filename.clone().with_chunk(chunk))).await?;
        }

        Ok(())
    }

    async fn get_uploaded_chunks<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<i64>> {
        let filename = filename.filename()?.with_chunk("*");
        let pattern = self.full_path(&filename);
        let paths = glob::glob(&pattern)?;

        let mut chunks = Vec::new();

        for path in paths {
            let path_str = path?.to_str().unwrap_or_default().replace(".part", "");

            let chunk = path_str
                .split('.')
                .next_back()
                .unwrap_or_default()
                .parse::<i64>()
                .map_err(|_| {
                    Error::InternalError(
                        "Failed to parse chunk number while getting uploaded chunks".to_string(),
                    )
                })?;

            chunks.push(chunk);
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
        let stream = self.inner_stream(&filename, chunk).await?;

        Ok(Streamer::new(stream))
    }

    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer> {
        let filename = filename.filename()?;
        let stream = self.inner_stream_tar(&filename).await;

        Ok(Streamer::new(stream))
    }

    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        self.inner_tar_content_length(filename).await
    }

    // ── Versioned chunk operations ──────────────────────────────────────

    async fn push_v<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
        chunk: i64,
        data: &[u8],
    ) -> AppResult<()> {
        let filename = filename.filename()?;
        let dir = self.version_dir(&filename, version);
        // Cheap: a missing dir means the first chunk of this version landed.
        // `create_dir_all` is idempotent on subsequent calls.
        create_dir_all(&dir).await?;

        let path = self.versioned_chunk_path(&filename, version, chunk);
        let file = File::create(&path).await?;
        let mut writer = tokio::io::BufWriter::new(file);
        writer.write_all(data).await?;
        writer.flush().await?;
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

        let path = self.versioned_chunk_path(&filename, version, chunk);
        let mut file = File::open(&path).await?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await?;
        Ok(buffer)
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

        let path = self.versioned_chunk_path(&filename, version, chunk);
        Ok(std::path::Path::new(&path).exists())
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
        self.list_versioned_chunks(&filename, version).await
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

        // Inline a small versioned variant of `inner_stream` — same lazy
        // open-one-at-a-time pattern, just using versioned chunk paths.
        let chunk_indices: Vec<i64> = match chunk {
            Some(c) => vec![c],
            None => self.list_versioned_chunks(&filename, version).await?,
        };

        let mut paths: Vec<String> = chunk_indices
            .into_iter()
            .map(|c| self.versioned_chunk_path(&filename, version, c))
            .collect();
        paths.reverse();

        let stream = futures_util::stream::unfold(
            paths,
            |mut paths: Vec<String>| async move {
                let path = paths.pop()?;
                let mut file = match File::open(&path).await {
                    Ok(f) => f,
                    Err(e) => {
                        log::error!("Failed to open chunk file {}: {:#?}", path, e);
                        return Some((Err(Error::from(e)), paths));
                    }
                };
                let mut data = vec![];
                match file.read_to_end(&mut data).await {
                    Ok(_) => Some((Ok(Bytes::from(data)), paths)),
                    Err(e) => {
                        log::error!("Failed to read chunk file {}: {:#?}", path, e);
                        Some((Err(Error::from(e)), paths))
                    }
                }
            },
        );

        Ok(Streamer::new(stream))
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

        // Build the entries upfront (name + path), then stream lazily.
        let chunks = self.list_versioned_chunks(&filename, version).await?;
        let mut entries: Vec<(String, String)> = chunks
            .iter()
            .map(|idx| {
                let path = self.versioned_chunk_path(&filename, version, *idx);
                let name = format!("{:06}.enc", idx);
                (name, path)
            })
            .collect();
        entries.reverse();

        // Re-use the existing tar state machine by transplanting it inline.
        enum Phase {
            NextEntry,
            Data(File, u64),
            Padding(usize),
            EndOfArchive,
            Done,
        }
        struct State {
            entries: Vec<(String, String)>,
            phase: Phase,
        }
        let state = State {
            entries,
            phase: Phase::NextEntry,
        };

        let stream = futures_util::stream::unfold(state, |mut state| async move {
            loop {
                match state.phase {
                    Phase::NextEntry => {
                        if let Some((name, path)) = state.entries.pop() {
                            let file = match File::open(&path).await {
                                Ok(f) => f,
                                Err(e) => {
                                    log::error!("Failed to open chunk {}: {:#?}", name, e);
                                    return Some((Err(Error::from(e)), state));
                                }
                            };
                            let size = match file.metadata().await {
                                Ok(m) => m.len(),
                                Err(e) => {
                                    log::error!("Failed to stat chunk {}: {:#?}", name, e);
                                    return Some((Err(Error::from(e)), state));
                                }
                            };
                            let header = tar::tar_header(&name, size);
                            state.phase = Phase::Data(file, size);
                            return Some((Ok(Bytes::from(header.to_vec())), state));
                        } else {
                            state.phase = Phase::EndOfArchive;
                        }
                    }
                    Phase::Data(mut file, size) => {
                        let mut data = Vec::with_capacity(size as usize);
                        match file.read_to_end(&mut data).await {
                            Ok(_) => {
                                let padding_len = tar::tar_padding_len(size);
                                state.phase = if padding_len > 0 {
                                    Phase::Padding(padding_len)
                                } else {
                                    Phase::NextEntry
                                };
                                return Some((Ok(Bytes::from(data)), state));
                            }
                            Err(e) => {
                                state.phase = Phase::NextEntry;
                                return Some((Err(Error::from(e)), state));
                            }
                        }
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
        });

        Ok(Streamer::new(stream))
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

        let chunks = self.list_versioned_chunks(&filename, version).await?;
        let mut total: u64 = 0;
        for idx in &chunks {
            let path = self.versioned_chunk_path(&filename, version, *idx);
            let file = File::open(&path).await?;
            let size = file.metadata().await?.len();
            total += 512 + size + tar::tar_padding_len(size) as u64;
        }
        total += tar::TAR_END_OF_ARCHIVE_LEN as u64;
        Ok(total)
    }

    async fn purge_version<T: IntoFilename>(
        &self,
        filename: &T,
        version: i32,
    ) -> AppResult<()> {
        let filename = filename.filename()?;
        let dir = self.version_dir(&filename, version);
        match remove_dir_all(&dir).await {
            Ok(()) => Ok(()),
            // Tolerate already-gone — the caller may be cleaning up after a
            // partial failure where the dir was never created.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::from(e)),
        }
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
        let src_chunks = if src_is_legacy {
            self.get_uploaded_chunks(&src).await?
        } else {
            self.list_versioned_chunks(&src, src_version).await?
        };

        let dst_dir = self.version_dir(&dst, dst_version);
        create_dir_all(&dst_dir).await?;

        for idx in src_chunks {
            let src_path = if src_is_legacy {
                self.full_path(&src.clone().with_chunk(idx))
            } else {
                self.versioned_chunk_path(&src, src_version, idx)
            };
            let dst_path = self.versioned_chunk_path(&dst, dst_version, idx);
            fs_copy(&src_path, &dst_path).await?;
        }
        Ok(())
    }

    async fn purge_all<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        let filename = filename.filename()?;
        // Versioned layout: drop the whole {file_id}/ tree if present.
        let root = self.file_root(&filename);
        match remove_dir_all(&root).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::from(e)),
        }
        // Legacy chunks (flat under data_dir) — fall through to the existing
        // glob+delete path so files that were never edited post-migration
        // are still cleaned up on full deletion.
        self.purge(&filename).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filename::Filename;
    use tempfile::tempdir;

    /// A versioned write+read round-trip lands chunks under `{uuid}/v{N}/`
    /// and reads them back via the versioned API. Validates the basic happy
    /// path before the storage repo wires it in.
    #[tokio::test]
    async fn versioned_round_trip() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("a1b2c3d4-uuid");

        provider.push_v(&filename, 2, 0, b"hello").await.unwrap();
        provider.push_v(&filename, 2, 1, b"world").await.unwrap();

        // Both chunks visible via the versioned listing.
        let chunks = provider.get_uploaded_chunks_v(&filename, 2).await.unwrap();
        assert_eq!(chunks, vec![0, 1]);

        // exists_v true for present chunks, false for missing.
        assert!(provider.exists_v(&filename, 2, 0).await.unwrap());
        assert!(!provider.exists_v(&filename, 2, 99).await.unwrap());

        // Read back a single chunk.
        let bytes = provider.pull_v(&filename, 2, 0).await.unwrap();
        assert_eq!(bytes, b"hello");
    }

    /// A read against `version == 1` for a file with no `v1/` directory but
    /// existing legacy chunks must transparently fall back. This is the
    /// pre-migration compatibility path.
    #[tokio::test]
    async fn legacy_fallback_for_v1() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        // Legacy filename includes a timestamp prefix and `.part.{n}` suffix.
        let filename = Filename::new("legacy-file-uuid").with_timestamp(1234567890);

        // Seed two legacy-format chunk files directly.
        provider.push(&filename, 0, b"old-chunk-0").await.unwrap();
        provider.push(&filename, 1, b"old-chunk-1").await.unwrap();

        // Versioned listing for v1 must surface the legacy chunks unchanged.
        let chunks = provider.get_uploaded_chunks_v(&filename, 1).await.unwrap();
        assert_eq!(chunks, vec![0, 1]);

        let bytes = provider.pull_v(&filename, 1, 1).await.unwrap();
        assert_eq!(bytes, b"old-chunk-1");
    }

    /// Once a versioned `v1/` directory has chunks, the fallback no longer
    /// fires — even if legacy chunks coexist (which won't happen in practice
    /// after the post-edit migration, but is worth pinning down).
    #[tokio::test]
    async fn legacy_fallback_skipped_when_versioned_has_chunks() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("dual-uuid").with_timestamp(42);

        provider.push(&filename, 0, b"legacy").await.unwrap();
        provider.push_v(&filename, 1, 0, b"versioned").await.unwrap();

        let bytes = provider.pull_v(&filename, 1, 0).await.unwrap();
        assert_eq!(bytes, b"versioned");
    }

    /// `purge_version` deletes only the targeted version directory; other
    /// versions stay intact.
    #[tokio::test]
    async fn purge_version_isolated() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("multi-uuid");

        provider.push_v(&filename, 1, 0, b"v1").await.unwrap();
        provider.push_v(&filename, 2, 0, b"v2").await.unwrap();

        provider.purge_version(&filename, 1).await.unwrap();

        assert!(provider.get_uploaded_chunks_v(&filename, 1).await.unwrap().is_empty());
        assert_eq!(
            provider.get_uploaded_chunks_v(&filename, 2).await.unwrap(),
            vec![0]
        );
    }

    /// `purge_version` against a missing directory is a no-op (used in the
    /// abandon-pending recovery path where the dir may never have been
    /// created).
    #[tokio::test]
    async fn purge_version_missing_is_ok() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("ghost-uuid");

        provider.purge_version(&filename, 99).await.unwrap();
    }

    /// In-place `copy_version` (same src and dst filename) duplicates
    /// all chunks from the source version into the destination, leaving
    /// the source intact. This is the restore-in-place path.
    #[tokio::test]
    async fn copy_version_in_place() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("copy-uuid");

        provider.push_v(&filename, 3, 0, b"a").await.unwrap();
        provider.push_v(&filename, 3, 1, b"b").await.unwrap();

        provider.copy_version(&filename, 3, &filename, 4).await.unwrap();

        assert_eq!(
            provider.get_uploaded_chunks_v(&filename, 3).await.unwrap(),
            vec![0, 1]
        );
        assert_eq!(
            provider.get_uploaded_chunks_v(&filename, 4).await.unwrap(),
            vec![0, 1]
        );
        assert_eq!(provider.pull_v(&filename, 4, 0).await.unwrap(), b"a");
        assert_eq!(provider.pull_v(&filename, 4, 1).await.unwrap(), b"b");
    }

    /// Cross-file `copy_version` with distinct src and dst filenames —
    /// the fork-as-new-note path.
    #[tokio::test]
    async fn copy_version_across_files() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let src = Filename::new("src-uuid");
        let dst = Filename::new("dst-uuid");

        provider.push_v(&src, 2, 0, b"hi").await.unwrap();
        provider.push_v(&src, 2, 1, b"there").await.unwrap();

        provider.copy_version(&src, 2, &dst, 1).await.unwrap();

        // Source untouched.
        assert_eq!(
            provider.get_uploaded_chunks_v(&src, 2).await.unwrap(),
            vec![0, 1]
        );
        // Destination has the chunks under its own dir at v1.
        assert_eq!(
            provider.get_uploaded_chunks_v(&dst, 1).await.unwrap(),
            vec![0, 1]
        );
        assert_eq!(provider.pull_v(&dst, 1, 0).await.unwrap(), b"hi");
        assert_eq!(provider.pull_v(&dst, 1, 1).await.unwrap(), b"there");
    }

    /// `copy_version` from a legacy-only source (no `v{src}/` dir) lifts
    /// legacy chunks into the new versioned destination — covers the
    /// "first edit of a pre-migration file" path.
    #[tokio::test]
    async fn copy_version_from_legacy_source() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("legacy-copy-uuid").with_timestamp(99);

        provider.push(&filename, 0, b"x").await.unwrap();
        provider.push(&filename, 1, b"y").await.unwrap();

        provider.copy_version(&filename, 1, &filename, 2).await.unwrap();

        let dst = provider.get_uploaded_chunks_v(&filename, 2).await.unwrap();
        assert_eq!(dst, vec![0, 1]);
        assert_eq!(provider.pull_v(&filename, 2, 0).await.unwrap(), b"x");
        assert_eq!(provider.pull_v(&filename, 2, 1).await.unwrap(), b"y");
    }

    /// `purge_all` wipes both the versioned tree and any legacy chunks.
    #[tokio::test]
    async fn purge_all_removes_versions_and_legacy() {
        let dir = tempdir().unwrap();
        let provider = FsProvider::new(dir.path().to_str().unwrap());
        let filename = Filename::new("nuke-uuid").with_timestamp(7);

        provider.push(&filename, 0, b"legacy").await.unwrap();
        provider.push_v(&filename, 2, 0, b"versioned").await.unwrap();

        provider.purge_all(&filename).await.unwrap();

        assert!(provider.get_uploaded_chunks_v(&filename, 2).await.unwrap().is_empty());
        // Legacy listing should also be empty after purge_all.
        assert!(provider.get_uploaded_chunks(&filename).await.unwrap().is_empty());
    }
}
