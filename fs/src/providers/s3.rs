use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};

use crate::{
    contract::FsProviderContract,
    filename::{Filename, IntoFilename},
    streamer::Streamer,
    tar,
};

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

    /// Key prefix for listing all chunks of a file.
    /// Appends ".part." so we only match chunk objects, not the base filename.
    fn chunk_prefix(&self, filename: &Filename) -> String {
        format!("{}{}.part.", self.prefix, filename)
    }

    /// Parse chunk index from an S3 key like "{prefix}{timestamp}-{id}.part.{chunk}"
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

        match self.bucket.head_object(&key).await {
            Ok(_) => Ok(true),
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("404")
                    || err_str.contains("NoSuchKey")
                    || err_str.contains("Not Found")
                {
                    Ok(false)
                } else {
                    Err(Error::StorageError(format!(
                        "S3 head object failed for '{}': {}",
                        key, e
                    )))
                }
            }
        }
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

        let response = self
            .bucket
            .get_object(&key)
            .await
            .map_err(|e| Error::StorageError(format!("S3 pull failed for '{}': {}", key, e)))?;

        Ok(response.to_vec())
    }

    async fn purge<T: IntoFilename>(&self, filename: &T) -> AppResult<()> {
        let prefix = self.chunk_prefix(&filename.filename()?);
        let objects = self.list_objects(&prefix).await?;

        for object in objects {
            self.bucket
                .delete_object(&object.key)
                .await
                .map_err(|e| {
                    Error::StorageError(format!(
                        "S3 delete failed for '{}': {}",
                        object.key, e
                    ))
                })?;
        }

        Ok(())
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

        // S3 doesn't have file handles — fetch all chunk data upfront
        // and stream from memory.
        let chunks_to_stream: Vec<i64> = match chunk {
            Some(c) => vec![c],
            None => self.get_uploaded_chunks(&filename).await?,
        };

        let mut chunk_data: Vec<Vec<u8>> = Vec::with_capacity(chunks_to_stream.len());
        for chunk_idx in &chunks_to_stream {
            let key = self.object_key(&filename.clone().with_chunk(*chunk_idx));
            let response = self
                .bucket
                .get_object(&key)
                .await
                .map_err(|e| {
                    Error::StorageError(format!("S3 stream read failed for '{}': {}", key, e))
                })?;
            chunk_data.push(response.to_vec());
        }

        chunk_data.reverse();

        let stream =
            futures_util::stream::unfold(chunk_data, |mut data: Vec<Vec<u8>>| async move {
                let chunk = data.pop()?;
                Some((Ok(Bytes::from(chunk)), data))
            });

        Ok(Streamer::new(stream))
    }

    async fn stream_tar<T: IntoFilename>(&self, filename: &T) -> AppResult<Streamer> {
        let filename = filename.filename()?;
        let chunks = self.get_uploaded_chunks(&filename).await?;

        let mut entries: Vec<(String, String)> = Vec::with_capacity(chunks.len());
        for chunk_idx in &chunks {
            let chunk_filename = filename.clone().with_chunk(*chunk_idx);
            let key = self.object_key(&chunk_filename);
            let name = format!("{:06}.enc", chunk_idx);
            entries.push((name, key));
        }

        entries.reverse();

        let bucket = self.bucket.clone();

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

        let state = State {
            entries,
            phase: Phase::NextEntry,
            bucket,
        };

        let stream = futures_util::stream::unfold(state, |mut state| async move {
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
                    Phase::Done => {
                        return None;
                    }
                }
            }
        });

        Ok(Streamer::new(stream))
    }

    async fn tar_content_length<T: IntoFilename>(&self, filename: &T) -> AppResult<u64> {
        let filename = filename.filename()?;
        let chunks = self.get_uploaded_chunks(&filename).await?;

        let mut total: u64 = 0;

        for chunk_idx in &chunks {
            let key = self.object_key(&filename.clone().with_chunk(*chunk_idx));

            let (head, _) = self.bucket.head_object(&key).await.map_err(|e| {
                Error::StorageError(format!(
                    "S3 head object failed for '{}': {}",
                    key, e
                ))
            })?;

            let size = head.content_length.unwrap_or(0) as u64;
            total += 512 + size + tar::tar_padding_len(size) as u64;
        }

        total += tar::TAR_END_OF_ARCHIVE_LEN as u64;
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_chunk_index() {
        assert_eq!(
            S3Provider::parse_chunk_index("1712345600-550e8400-e29b-41d4-a716-446655440000.part.0")
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
    fn test_parse_chunk_index_error() {
        assert!(S3Provider::parse_chunk_index("invalid-key-no-part").is_err());
    }
}
