use crate::error::{Error, HttpError, Result};
use crate::platform::HttpClient;
use crate::types::{Auth, ChunkResponse, DownloadSource, FileHashes};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, RequestInit, RequestMode, RequestCredentials, Response};

pub struct WasmHttpClient;

impl WasmHttpClient {
    pub fn new() -> Self {
        Self
    }

    fn build_headers(auth: &Auth) -> Result<Headers> {
        let headers = Headers::new().map_err(|e| Error::Io(format!("{e:?}")))?;
        if let Some(ref token) = auth.jwt_token {
            headers
                .set("Authorization", &format!("Bearer {token}"))
                .map_err(|e| Error::Io(format!("{e:?}")))?;
        }
        if let Some(ref refresh) = auth.refresh_token {
            headers
                .set("X-Auth-Refresh", refresh)
                .map_err(|e| Error::Io(format!("{e:?}")))?;
        }
        Ok(headers)
    }


    /// Collect a response body by pulling its `ReadableStream`, reporting the
    /// running byte count after every read so download progress moves with
    /// the wire instead of jumping when the body completes. Bodies without a
    /// stream (older engines, opaque responses) fall back to one buffered
    /// read with a single final report.
    async fn read_streaming(resp: &Response, on_bytes: &dyn Fn(u64)) -> Result<Vec<u8>> {
        let Some(body) = resp.body() else {
            let ab_promise = resp.array_buffer().map_err(|e| Error::Io(format!("{e:?}")))?;
            let array_buffer = JsFuture::from(ab_promise)
                .await
                .map_err(|e| Error::Io(format!("{e:?}")))?;
            let data = js_sys::Uint8Array::new(&array_buffer).to_vec();
            on_bytes(data.len() as u64);
            return Ok(data);
        };

        let reader: web_sys::ReadableStreamDefaultReader = body
            .get_reader()
            .dyn_into()
            .map_err(|e| Error::Io(format!("{e:?}")))?;

        let mut data: Vec<u8> = Vec::new();

        loop {
            let result = JsFuture::from(reader.read())
                .await
                .map_err(|e| Error::Io(format!("{e:?}")))?;

            let done = js_sys::Reflect::get(&result, &JsValue::from_str("done"))
                .ok()
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

            if done {
                break;
            }

            let value = js_sys::Reflect::get(&result, &JsValue::from_str("value"))
                .map_err(|e| Error::Io(format!("{e:?}")))?;
            let piece = js_sys::Uint8Array::new(&value);
            let start = data.len();
            data.resize(start + piece.length() as usize, 0);
            piece.copy_to(&mut data[start..]);
            on_bytes(data.len() as u64);
        }

        Ok(data)
    }

    async fn do_fetch(opts: &RequestInit, url: &str) -> Result<Response> {
        let global = js_sys::global();
        let fetch_fn: js_sys::Function = js_sys::Reflect::get(&global, &JsValue::from_str("fetch"))
            .map_err(|e| Error::Io(format!("No fetch: {e:?}")))?
            .into();

        let request = web_sys::Request::new_with_str_and_init(url, opts)
            .map_err(|e| Error::Io(format!("{e:?}")))?;

        let promise: js_sys::Promise = fetch_fn
            .call1(&global, &request)
            .map_err(|e| Error::Io(format!("{e:?}")))?
            .into();

        let resp_value = JsFuture::from(promise)
            .await
            .map_err(|e| Error::Io(format!("{e:?}")))?;

        Ok(Response::from(resp_value))
    }
}

impl HttpClient for WasmHttpClient {
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
            let headers = Self::build_headers(&auth)?;
            headers
                .set("Content-Type", "application/octet-stream")
                .map_err(|e| Error::Io(format!("{e:?}")))?;

            let encoded_checksum = js_sys::encode_uri_component(&checksum)
                .as_string()
                .unwrap_or(checksum.clone());

            let url = format!(
                "{}/api/storage/{}?chunk={}&checksum={}&checksum_function=crc16",
                auth.base_url, file_id, chunk_index, encoded_checksum,
            );

            let body = js_sys::Uint8Array::from(data.as_slice());

            let opts = RequestInit::new();
            opts.set_method("POST");
            opts.set_headers(&headers);
            opts.set_body(&body);
            opts.set_mode(RequestMode::Cors);
            opts.set_credentials(RequestCredentials::Include);

            let resp = Self::do_fetch(&opts, &url).await?;
            let status = resp.status();

            let text_promise = resp.text().map_err(|e| Error::Io(format!("{e:?}")))?;
            let text = JsFuture::from(text_promise)
                .await
                .map_err(|e| Error::Io(format!("{e:?}")))?
                .as_string()
                .unwrap_or_default();

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
            let headers = Self::build_headers(&auth)?;

            let opts = RequestInit::new();
            opts.set_method(method);
            opts.set_headers(&headers);
            opts.set_mode(RequestMode::Cors);
            opts.set_credentials(RequestCredentials::Include);

            let resp = Self::do_fetch(&opts, &url).await?;
            let status = resp.status();

            if status >= 400 {
                let text_promise = resp.text().map_err(|e| Error::Io(format!("{e:?}")))?;
                let text = JsFuture::from(text_promise)
                    .await
                    .map_err(|e| Error::Io(format!("{e:?}")))?
                    .as_string()
                    .unwrap_or_default();
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation: None,
                }));
            }

            Self::read_streaming(&resp, &on_bytes).await
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
            let headers = Self::build_headers(&auth)?;

            let url = format!(
                "{}/api/storage/{}?format=tar",
                auth.base_url, file_id
            );

            let opts = RequestInit::new();
            opts.set_method("GET");
            opts.set_headers(&headers);
            opts.set_mode(RequestMode::Cors);
            opts.set_credentials(RequestCredentials::Include);

            let resp = Self::do_fetch(&opts, &url).await?;
            let status = resp.status();

            if status >= 400 {
                let text_promise = resp.text().map_err(|e| Error::Io(format!("{e:?}")))?;
                let text = JsFuture::from(text_promise)
                    .await
                    .map_err(|e| Error::Io(format!("{e:?}")))?
                    .as_string()
                    .unwrap_or_default();
                return Err(Error::Http(HttpError {
                    status,
                    message: text,
                    validation: None,
                }));
            }

            Self::read_streaming(&resp, &on_bytes).await
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
            let headers = Self::build_headers(&auth)?;
            headers
                .set("Content-Type", "application/x-tar")
                .map_err(|e| Error::Io(format!("{e:?}")))?;

            let url = format!(
                "{}/api/storage/{}?format=tar",
                auth.base_url, file_id
            );

            let body = js_sys::Uint8Array::from(tar_body.as_slice());

            let opts = RequestInit::new();
            opts.set_method("POST");
            opts.set_headers(&headers);
            opts.set_body(&body);
            opts.set_mode(RequestMode::Cors);
            opts.set_credentials(RequestCredentials::Include);

            let resp = Self::do_fetch(&opts, &url).await?;
            let status = resp.status();

            let text_promise = resp.text().map_err(|e| Error::Io(format!("{e:?}")))?;
            let text = JsFuture::from(text_promise)
                .await
                .map_err(|e| Error::Io(format!("{e:?}")))?
                .as_string()
                .unwrap_or_default();

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
        let body = serde_json::to_string(hashes).unwrap_or_default();

        Box::pin(async move {
            let headers = Self::build_headers(&auth)?;
            headers
                .set("Content-Type", "application/json")
                .map_err(|e| Error::Io(format!("{e:?}")))?;

            let url = format!("{}/api/storage/{}/hashes", auth.base_url, file_id);

            let opts = RequestInit::new();
            opts.set_method("PUT");
            opts.set_headers(&headers);
            opts.set_body(&JsValue::from_str(&body));
            opts.set_mode(RequestMode::Cors);
            opts.set_credentials(RequestCredentials::Include);

            let resp = Self::do_fetch(&opts, &url).await?;
            let status = resp.status();

            if status >= 400 {
                let text_promise = resp.text().map_err(|e| Error::Io(format!("{e:?}")))?;
                let text = JsFuture::from(text_promise)
                    .await
                    .map_err(|e| Error::Io(format!("{e:?}")))?
                    .as_string()
                    .unwrap_or_default();
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
