use error::{AppResult, Error};
use std::{
    fs::{remove_file, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
};

use crate::contract::StorageProvider;

pub struct FsProvider<'provider> {
    data_dir: &'provider str,
}

impl<'provider> FsProvider<'provider> {
    pub fn new(data_dir: &'provider str) -> Self {
        Self { data_dir }
    }

    pub fn full_path(&self, path: &str) -> String {
        format!("{}/{}", self.data_dir, path)
    }

    fn inner_concat_parts(&self, path: &str, chunks: u64) -> AppResult<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(self.full_path(path))
            .map_err(Error::from)?;

        for chunk in 0..chunks {
            let chunk_path = format!("{}.{}.part", path, chunk);

            log::debug!("Reading part: {}", &chunk_path);

            let mut part = match self.get(&chunk_path) {
                Ok(part) => part,
                Err(err) => {
                    log::error!(
                        "Error while reading part: {}, upstream error: {}",
                        chunk_path,
                        err
                    );
                    return Err(err);
                }
            };

            let mut buffer = Vec::new();

            part.read_to_end(&mut buffer)?;

            log::debug!("Writing part: {}", &chunk_path);

            let n = file.write(&buffer)?;

            if n != buffer.len() {
                return Err(Error::InternalError(format!(
                    "Failed to write all bytes for a chunk {}",
                    chunk_path
                )));
            }
        }

        Ok(())
    }

    fn inner_remove_chunks(&self, path: &str, chunks: u64) -> AppResult<()> {
        for chunk in 0..chunks {
            let chunk_path = format!("{}.{}.part", path, chunk);

            match self.remove(&chunk_path) {
                Ok(_) => (),
                Err(err) => {
                    log::error!(
                        "Error while removing part: {}, upstream error: {}",
                        chunk_path,
                        err
                    );
                    return Err(err);
                }
            };
        }

        Ok(())
    }
}

impl<'ctx> StorageProvider for FsProvider<'ctx> {
    fn part_exists(&self, filename: &str, chunk: i32) -> AppResult<bool> {
        Ok(std::path::Path::new(
            self.full_path(format!("{}.{}.part", filename, chunk).as_str())
                .as_str(),
        )
        .exists())
    }

    fn get(&self, filename: &str) -> AppResult<File> {
        File::open(self.full_path(filename)).map_err(Error::from)
    }

    fn create(&self, filename: &str) -> AppResult<File> {
        File::create(self.full_path(filename)).map_err(Error::from)
    }

    fn get_or_create(&self, filename: &str) -> AppResult<File> {
        if let Ok(file) = self.get(filename) {
            return Ok(file);
        }

        self.create(filename)
    }

    fn push(&self, filename: &str, data: &[u8]) -> AppResult<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(self.full_path(filename))
            .map_err(Error::from)?;

        file.write_all(data).map_err(Error::from)
    }

    fn push_part(&self, filename: &str, chunk: i32, data: &[u8]) -> AppResult<()> {
        let chunk_path = format!("{}.{}.part", filename, chunk);
        log::debug!("Writing part: {}", &chunk_path);

        let mut file = self.get_or_create(&chunk_path)?;

        file.write_all(data).map_err(Error::from)
    }

    fn pull(&self, filename: &str, chunk: u64) -> AppResult<Vec<u8>> {
        let mut file = self.get(filename)?;
        let file_size = file.seek(SeekFrom::End(0))?;
        let start = chunk * crate::CHUNK_SIZE_BYTES;
        let mut buffer = vec![0; crate::CHUNK_SIZE_BYTES as usize];

        if start >= file_size {
            return Err(Error::BadRequest("chunk_after_eof".to_string()));
        }

        // Ensure that we don't read past the end of the file
        let bytes_to_read = std::cmp::min(crate::CHUNK_SIZE_BYTES, file_size - start) as usize;
        buffer.resize(bytes_to_read, 0);

        file.seek(SeekFrom::Start(start))?;
        file.read_exact(&mut buffer)?;

        Ok(buffer)
    }

    fn remove(&self, filename: &str) -> AppResult<()> {
        remove_file(self.full_path(filename)).map_err(Error::from)
    }

    fn concat_files(&self, filename: &str, chunks: u64) -> AppResult<()> {
        match self.inner_concat_parts(filename, chunks) {
            Ok(_) => (),
            Err(e) => {
                log::error!(
                    "Error while concatenating parts of a file: {}, upstream error: {}",
                    filename,
                    e
                );

                self.inner_remove_chunks(filename, chunks)?;

                self.remove(filename)?;

                return Err(e);
            }
        }

        self.inner_remove_chunks(filename, chunks)?;

        Ok(())
    }
}
