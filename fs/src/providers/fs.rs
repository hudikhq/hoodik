use actix_web::web::Bytes;
use async_trait::async_trait;
use error::{AppResult, Error};
use fs4::available_space;
use tokio::{
    fs::{remove_file, File, OpenOptions},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    contract::FsProviderContract,
    filename::{Filename, IntoFilename},
    streamer::Streamer,
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
    /// better readeability of the code.
    async fn inner_stream<T: IntoFilename>(
        &self,
        filename: &T,
        chunk: Option<i64>,
    ) -> impl futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> {
        let mut files: Vec<File> = match chunk {
            Some(chunk) => match self.get(filename, chunk).await {
                Ok(file) => vec![file],
                Err(e) => {
                    log::error!("Got error when trying to create inner stream: {:#?}", e);
                    vec![]
                }
            },
            None => match self.all(filename).await {
                Ok(files) => files,
                Err(e) => {
                    log::error!("Got error when trying to create inner stream: {:#?}", e);
                    vec![]
                }
            },
        };

        // Reverse the files so we can pop them from the end
        files.reverse();

        // We are passing the Vec<File> here because those files are not read yet..
        // but in the future if we want to create another FsProvider, for example S3, this would
        // would only have the chunk number and file name passed, or construct of both and then the
        // file getting would be happening inside the closure itself and not before.
        futures_util::stream::unfold(files as Vec<File>, |mut files: Vec<File>| async move {
            let mut file = files.pop()?;

            let mut data = vec![];

            match file.read_to_end(&mut data).await {
                Ok(_) => (),
                Err(e) => return Some((Err(Error::from(e)), files)),
            };

            let data = Bytes::from(data);

            Some((Ok(data), files))
        })
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

        let mut file = OpenOptions::new().read(true).write(true).open(path).await?;

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

    async fn get<T: IntoFilename>(&self, filename: &T, chunk: i64) -> AppResult<File> {
        let filename = filename.filename()?.with_chunk(chunk);

        OpenOptions::new()
            .read(true)
            .write(true)
            .open(self.full_path(&filename))
            .await
            .map_err(Error::from)
    }

    async fn all<T: IntoFilename>(&self, filename: &T) -> AppResult<Vec<File>> {
        let filename = filename.filename()?;

        let chunks = self.get_uploaded_chunks(&filename).await?;
        let mut files: Vec<File> = vec![];

        for chunk in chunks {
            files.push(self.get(&filename, chunk).await?);
        }

        Ok(files)
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
        let stream = self.inner_stream(&filename, chunk).await;

        Ok(Streamer::new(stream))
    }
}
