use error::{AppResult, Error};
use std::{
    fs::{remove_file, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
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

    /// Recursively search for files matching the pattern
    fn search_dir(dir: &Path, pattern: &str) -> AppResult<Vec<String>> {
        let mut results = Vec::new();

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                results.extend(Self::search_dir(&path, pattern)?);
            } else if let Some(filename) = path.file_name() {
                if filename.to_string_lossy().contains(pattern) {
                    results.push(path.display().to_string())
                }
            }
        }

        Ok(results)
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

    fn purge(&self, filename: &str) -> AppResult<()> {
        let string_path = self.full_path("");
        let path = Path::new(string_path.as_str());

        let pattern = format!("{}{}*", string_path, filename);
        let mut found_part_files = Self::search_dir(path, &pattern)?;

        let file = format!("{}{}", string_path, filename);
        found_part_files.push(file);

        for path in found_part_files {
            remove_file(path)?;
        }

        Ok(())
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

    fn get_uploaded_chunks(&self, filename: &str) -> AppResult<Vec<i32>> {
        let pattern = format!("{}.*.part", filename);
        let paths = glob::glob(self.full_path(pattern.as_str()).as_str())?;

        let mut chunks = Vec::new();

        for path in paths {
            let path = path?.to_str().unwrap().replace(".part", "");

            let chunk = path
                .split('.')
                .last()
                .unwrap()
                .parse::<i32>()
                .map_err(|_| Error::InternalError("Failed to parse chunk number".to_string()))?;

            chunks.push(chunk);
        }

        chunks.sort();

        Ok(chunks)
    }
}
