use crate::error::{Error, HttpError, Result};
use crate::platform::HttpClient;
use futures::StreamExt;
use crate::types::{Auth, ChunkResponse, DownloadSource, FileHashes};
use std::collections::HashMap;

/// Native HTTP client backed by `reqwest`.
pub struct NativeHttpClient {
    client: reqwest::Client,
}

impl NativeHttpClient {
    /// Collect a response body while reporting the running byte count after
    /// every network read, so download progress moves with the wire instead
    /// of jumping when the body completes.
    async fn read_streaming(resp: reqwest::Response, on_bytes: &dyn Fn(u64)) -> Result<Vec<u8>> {
        let expected = resp.content_length().unwrap_or(0) as usize;
        let mut data = Vec::with_capacity(expected);
        let mut stream = resp.bytes_stream();

        while let Some(piece) = stream.next().await {
            let piece = piece.map_err(|e| Error::Io(format!("Failed to read response bytes: {e}")))?;
            data.extend_from_slice(&piece);
            on_bytes(data.len() as u64);
        }

        Ok(data)
    }

    pub fn new() -> Result<Self> {
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| Error::Io(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self { client })
    }

    fn auth_headers(auth: &Auth) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(ref token) = auth.jwt_token {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {token}")) {
                headers.insert(reqwest::header::AUTHORIZATION, val);
            }
        }

        if let Some(ref refresh) = auth.refresh_token {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(refresh) {
                headers.insert("X-Auth-Refresh", val);
            }
        }

        if let Some(ref cookie) = auth.cookie {
            if let Ok(val) = reqwest::header::HeaderValue::from_str(cookie) {
                headers.insert(reqwest::header::COOKIE, val);
            }
        }

        headers
    }
}

impl HttpClient for NativeHttpClient {
    fn upload_chunk(
        &self,
        auth: &Auth,
        file_id: &str,
        chunk_index: u64,
        checksum: &str,
        data: &[u8],
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>> {
        let auth = auth.clone();
        let file_id = file_id.to_string();
        let checksum = checksum.to_string();
        let data = data.to_vec();

        Box::pin(async move {
            let url = format!("{}/api/storage/{}", auth.base_url, file_id);
            let headers = Self::auth_headers(&auth);

            let resp = self
                .client
                .post(&url)
                .headers(headers)
                .query(&[
                    ("chunk", chunk_index.to_string()),
                    ("checksum", checksum),
                    ("checksum_function", "crc16".to_string()),
                ])
                .header("Content-Type", "application/octet-stream")
                .body(data)
                .send()
                .await
                .map_err(|e| Error::Io(format!("Upload request failed: {e}")))?;

            let status = resp.status().as_u16();
            let text = resp
                .text()
                .await
                .map_err(|e| Error::Io(format!("Failed to read response: {e}")))?;

            if status >= 400 {
                let validation = parse_validation(&text, status);
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation,
                }));
            }

            serde_json::from_str::<ChunkResponse>(&text)
                .map_err(|e| Error::Io(format!("Failed to parse upload response: {e}")))
        })
    }

    fn download_chunk<'a>(
        &'a self,
        auth: &Auth,
        source: DownloadSource<'_>,
        chunk_index: u64,
        on_bytes: Box<dyn Fn(u64) + 'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + 'a>> {
        let auth = auth.clone();
        let url = source.chunk_url(&auth.base_url, chunk_index);
        let method = source.method();

        Box::pin(async move {
            let headers = Self::auth_headers(&auth);

            let request = match method {
                "POST" => self.client.post(&url),
                _ => self.client.get(&url),
            };
            let resp = request
                .headers(headers)
                .send()
                .await
                .map_err(|e| Error::Io(format!("Download request failed: {e}")))?;

            let status = resp.status().as_u16();

            if status >= 400 {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| Error::Io(format!("Failed to read error response: {e}")))?;
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation: None,
                }));
            }

            Self::read_streaming(resp, &on_bytes).await
        })
    }

    fn download_all_chunks<'a>(
        &'a self,
        auth: &Auth,
        file_id: &str,
        on_bytes: Box<dyn Fn(u64) + 'a>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<u8>>> + 'a>> {
        let auth = auth.clone();
        let file_id = file_id.to_string();

        Box::pin(async move {
            let url = format!("{}/api/storage/{}", auth.base_url, file_id);
            let headers = Self::auth_headers(&auth);

            let resp = self
                .client
                .get(&url)
                .headers(headers)
                .query(&[("format", "tar")])
                .send()
                .await
                .map_err(|e| Error::Io(format!("Download tar request failed: {e}")))?;

            let status = resp.status().as_u16();

            if status >= 400 {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| Error::Io(format!("Failed to read error response: {e}")))?;
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation: None,
                }));
            }

            Self::read_streaming(resp, &on_bytes).await
        })
    }

    fn upload_chunks_tar(
        &self,
        auth: &Auth,
        file_id: &str,
        tar_body: Vec<u8>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ChunkResponse>> + '_>> {
        let auth = auth.clone();
        let file_id = file_id.to_string();

        Box::pin(async move {
            let url = format!("{}/api/storage/{}", auth.base_url, file_id);
            let headers = Self::auth_headers(&auth);

            let resp = self
                .client
                .post(&url)
                .headers(headers)
                .query(&[("format", "tar")])
                .header("Content-Type", "application/x-tar")
                .body(tar_body)
                .send()
                .await
                .map_err(|e| Error::Io(format!("Upload tar request failed: {e}")))?;

            let status = resp.status().as_u16();
            let text = resp
                .text()
                .await
                .map_err(|e| Error::Io(format!("Failed to read response: {e}")))?;

            if status >= 400 {
                let validation = parse_validation(&text, status);
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation,
                }));
            }

            serde_json::from_str::<ChunkResponse>(&text)
                .map_err(|e| Error::Io(format!("Failed to parse upload-tar response: {e}")))
        })
    }

    fn update_hashes(
        &self,
        auth: &Auth,
        file_id: &str,
        hashes: &FileHashes,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + '_>> {
        let auth = auth.clone();
        let file_id = file_id.to_string();
        let hashes = hashes.clone();

        Box::pin(async move {
            let url = format!("{}/api/storage/{}/hashes", auth.base_url, file_id);
            let headers = Self::auth_headers(&auth);

            let resp = self
                .client
                .put(&url)
                .headers(headers)
                .json(&hashes)
                .send()
                .await
                .map_err(|e| Error::Io(format!("Update hashes request failed: {e}")))?;

            let status = resp.status().as_u16();

            if status >= 400 {
                let text = resp
                    .text()
                    .await
                    .map_err(|e| Error::Io(format!("Failed to read error response: {e}")))?;
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation: None,
                }));
            }

            Ok(())
        })
    }
}

fn parse_validation(body: &str, status: u16) -> Option<HashMap<String, String>> {
    if status != 422 {
        return None;
    }

    #[derive(serde::Deserialize)]
    struct ApiError {
        context: Option<ValidationContext>,
    }
    #[derive(serde::Deserialize)]
    struct ValidationContext {
        errors: Option<HashMap<String, ValidationField>>,
    }
    #[derive(serde::Deserialize)]
    struct ValidationField {
        errors: Option<Vec<String>>,
    }

    let api_err: ApiError = serde_json::from_str(body).ok()?;
    let ctx = api_err.context?;
    let errors = ctx.errors?;

    let mut result = HashMap::new();
    for (key, field) in errors {
        if let Some(ref errs) = field.errors {
            if !errs.is_empty() {
                result.insert(key, errs.join(", "));
            }
        }
    }
    Some(result)
}
