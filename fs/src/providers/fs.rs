use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};
use fs4::available_space;
use tokio::{
    fs::{remove_file, File},
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
}
