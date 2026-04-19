//! Bulk `DeleteObjects` support for `S3Provider`.
//!
//! `rust-s3` 0.35 exposes no `DeleteObjects` command, but the pieces needed to
//! hand-build one are public: the bucket URL, HTTP client, and AWS SigV4
//! primitives. The S3 `POST ?delete` API is the right answer at the versioned
//! scale we care about (a thirty-version note with thirty chunks per version
//! is nine hundred objects; per-object delete is a cost amplifier against
//! S3-priced providers).
//!
//! Batches are capped at one thousand keys — the AWS documented limit — so
//! a caller with more keys is paginated transparently. A per-batch bounded
//! fallback to single-object deletes remains available for the narrow case
//! where the bulk endpoint is unsupported; MinIO and AWS both implement it.
//!
//! The body is signed with a SHA-256 payload hash and a `Content-MD5`
//! header, per the DeleteObjects API requirements.
//!
//! Refs:
//! - <https://docs.aws.amazon.com/AmazonS3/latest/API/API_DeleteObjects.html>
//! - <https://docs.aws.amazon.com/AmazonS3/latest/userguide/delete-multiple-objects.html>

use base64::Engine;
use error::{AppResult, Error};
use futures::stream::{StreamExt, TryStreamExt};
use hmac::{Hmac, Mac};
use http::header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE, HOST};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use s3::signing;
use sha2::{Digest as _, Sha256};
use time::OffsetDateTime;
use url::Url;

const BATCH_LIMIT: usize = 1000;

/// Delete every key in `keys` from `bucket`, batched up to `BATCH_LIMIT`
/// per request. No-op on an empty list.
pub(super) async fn delete_keys(
    bucket: &s3::Bucket,
    keys: Vec<String>,
) -> AppResult<()> {
    if keys.is_empty() {
        return Ok(());
    }

    for batch in keys.chunks(BATCH_LIMIT) {
        // `Error` is `!Send` (transitive `dyn ResponseError` inside
        // `MultipartError`), and `async-trait` requires the outer future
        // to be `Send`. Collapse the Result into a plain bool before the
        // next await so nothing non-Send crosses a yield point.
        let needs_fallback = match delete_one_batch(bucket, batch).await {
            Ok(()) => false,
            Err(bulk_err) => {
                if is_bulk_unsupported(&bulk_err) {
                    log::warn!(
                        "Bulk DeleteObjects unsupported on this endpoint; \
                         falling back to per-object deletes: {}",
                        bulk_err
                    );
                    true
                } else {
                    return Err(bulk_err);
                }
            }
        };
        if needs_fallback {
            delete_individually(bucket, batch).await?;
        }
    }
    Ok(())
}

/// Bounded-concurrency per-object delete. Used only as the fallback for
/// endpoints that reject `POST ?delete`.
async fn delete_individually(bucket: &s3::Bucket, keys: &[String]) -> AppResult<()> {
    futures::stream::iter(keys.iter().cloned())
        .map(|key| {
            let bucket = bucket.clone();
            async move {
                bucket.delete_object(&key).await.map_err(|e| {
                    Error::StorageError(format!(
                        "S3 delete_object failed for '{}': {}",
                        key, e
                    ))
                })?;
                Ok::<(), Error>(())
            }
        })
        .buffer_unordered(32)
        .try_collect::<Vec<()>>()
        .await?;
    Ok(())
}

/// Issue a single `POST ?delete` request for up to 1000 keys.
async fn delete_one_batch(bucket: &s3::Bucket, keys: &[String]) -> AppResult<()> {
    let body = build_delete_body(keys);
    let body_bytes = body.as_bytes();

    let content_md5 = {
        use md5::Digest as _;
        let mut hasher = md5::Md5::new();
        hasher.update(body_bytes);
        base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
    };

    let payload_sha = {
        let mut hasher = Sha256::new();
        hasher.update(body_bytes);
        hex::encode(hasher.finalize())
    };

    let datetime = OffsetDateTime::now_utc();
    let url = build_delete_url(bucket)?;
    let host = url
        .host_str()
        .ok_or_else(|| Error::InternalError("S3 bucket URL has no host".into()))?
        .to_string();
    let host_header = match url.port() {
        Some(p) => format!("{}:{}", host, p),
        None => host.clone(),
    };

    // Headers in the canonical request have to match what we actually send.
    let mut headers = HeaderMap::new();
    headers.insert(HOST, hv(&host_header)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/xml"));
    headers.insert(CONTENT_LENGTH, hv(&body_bytes.len().to_string())?);
    headers.insert(HeaderName::from_static("content-md5"), hv(&content_md5)?);
    headers.insert(
        HeaderName::from_static("x-amz-content-sha256"),
        hv(&payload_sha)?,
    );
    headers.insert(
        HeaderName::from_static("x-amz-date"),
        hv(&long_datetime(&datetime)?)?,
    );

    let access_key = bucket
        .access_key()
        .await
        .map_err(|e| Error::InternalError(format!("S3 access_key fetch failed: {}", e)))?
        .ok_or_else(|| Error::InternalError("S3 access_key is not configured".into()))?;
    let secret_key = bucket
        .secret_key()
        .await
        .map_err(|e| Error::InternalError(format!("S3 secret_key fetch failed: {}", e)))?
        .ok_or_else(|| Error::InternalError("S3 secret_key is not configured".into()))?;

    let canonical_request =
        signing::canonical_request(Method::POST.as_str(), &url, &headers, &payload_sha)
            .map_err(|e| Error::InternalError(format!("canonical_request: {}", e)))?;

    let string_to_sign = signing::string_to_sign(&datetime, &bucket.region(), &canonical_request)
        .map_err(|e| Error::InternalError(format!("string_to_sign: {}", e)))?;

    let signing_key = signing::signing_key(&datetime, &secret_key, &bucket.region(), "s3")
        .map_err(|e| Error::InternalError(format!("signing_key: {}", e)))?;

    let mut mac = <Hmac<Sha256>>::new_from_slice(&signing_key).map_err(|e| {
        Error::InternalError(format!("HMAC-SHA256 init failed: {}", e))
    })?;
    mac.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    let signed_headers = signing::signed_header_string(&headers);
    let authorization = signing::authorization_header(
        &access_key,
        &datetime,
        &bucket.region(),
        &signed_headers,
        &signature,
    )
    .map_err(|e| Error::InternalError(format!("authorization_header: {}", e)))?;

    headers.insert(AUTHORIZATION, hv(&authorization)?);

    // Build the hyper request from the assembled headers + body.
    let mut request = hyper::Request::builder()
        .method(hyper::Method::POST)
        .uri(url.as_str());
    for (name, value) in headers.iter() {
        request = request.header(name, value);
    }
    let request = request
        .body(hyper::Body::from(body))
        .map_err(|e| Error::InternalError(format!("hyper request build: {}", e)))?;

    let client = bucket.http_client();
    let response = client
        .request(request)
        .await
        .map_err(|e| Error::StorageError(format!("S3 DeleteObjects request failed: {}", e)))?;

    let status = response.status();
    let body_bytes = hyper::body::to_bytes(response.into_body())
        .await
        .map_err(|e| Error::StorageError(format!("DeleteObjects body read failed: {}", e)))?;
    let body_str = String::from_utf8_lossy(&body_bytes).into_owned();

    if !status.is_success() {
        return Err(Error::StorageError(format!(
            "S3 DeleteObjects returned status {}: {}",
            status, body_str
        )));
    }

    // A 200 body can still carry per-key <Error> entries. Bail on the first
    // one — callers treat any failure as terminal because keys that survived
    // a purge leave orphans behind.
    if let Some(err) = extract_per_key_error(&body_str) {
        return Err(Error::StorageError(format!(
            "S3 DeleteObjects reported per-key error: {}",
            err
        )));
    }

    Ok(())
}

fn build_delete_body(keys: &[String]) -> String {
    // `Quiet=true` suppresses per-key <Deleted> entries from the response
    // when nothing went wrong; errors still come back.
    let mut body = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <Delete><Quiet>true</Quiet>",
    );
    for key in keys {
        body.push_str("<Object><Key>");
        body.push_str(&xml_escape(key));
        body.push_str("</Key></Object>");
    }
    body.push_str("</Delete>");
    body
}

fn xml_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn build_delete_url(bucket: &s3::Bucket) -> AppResult<Url> {
    let base = bucket.url();
    let url = Url::parse(&format!("{}/?delete", base))
        .map_err(|e| Error::InternalError(format!("Invalid S3 bucket URL '{}': {}", base, e)))?;
    Ok(url)
}

fn long_datetime(datetime: &OffsetDateTime) -> AppResult<String> {
    use time::format_description::FormatItem;
    use time::macros::format_description;
    const FMT: &[FormatItem<'static>] = format_description!(
        "[year][month][day]T[hour][minute][second]Z"
    );
    datetime
        .format(FMT)
        .map_err(|e| Error::InternalError(format!("datetime format: {}", e)))
}

/// Signal of "this endpoint doesn't speak POST ?delete" — rare in practice
/// but cheap to detect so the fallback path is only taken when it's the
/// right answer. Any 400/501/403-from-the-endpoint-itself qualifies.
fn is_bulk_unsupported(err: &Error) -> bool {
    let s = format!("{:?}", err);
    s.contains("NotImplemented")
        || s.contains("MethodNotAllowed")
        || s.contains("status 501")
        || s.contains("status 405")
}

/// Scan the response body for an `<Error>` block. DeleteObjects with
/// `Quiet=true` omits `<Deleted>` entries but still emits `<Error>` for
/// failures — return the first one for reporting.
fn extract_per_key_error(body: &str) -> Option<String> {
    let start = body.find("<Error>")?;
    let end = body[start..]
        .find("</Error>")
        .map(|i| start + i + "</Error>".len())?;
    Some(body[start..end].to_string())
}

fn hv(value: &str) -> AppResult<HeaderValue> {
    HeaderValue::from_str(value)
        .map_err(|e| Error::InternalError(format!("Invalid HTTP header value '{}': {}", value, e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xml_escape_escapes_reserved() {
        assert_eq!(xml_escape("a<b>c&d\"e'f"), "a&lt;b&gt;c&amp;d&quot;e&apos;f");
    }

    #[test]
    fn delete_body_wraps_keys() {
        let body = build_delete_body(&["k1".into(), "k2".into()]);
        assert!(body.starts_with("<?xml"));
        assert!(body.contains("<Quiet>true</Quiet>"));
        assert!(body.contains("<Object><Key>k1</Key></Object>"));
        assert!(body.contains("<Object><Key>k2</Key></Object>"));
        assert!(body.trim_end().ends_with("</Delete>"));
    }

    #[test]
    fn extract_per_key_error_finds_block() {
        let body = "<DeleteResult><Error><Key>x</Key><Message>oops</Message></Error></DeleteResult>";
        assert!(extract_per_key_error(body).unwrap().contains("oops"));
    }

    #[test]
    fn extract_per_key_error_none_on_ok() {
        assert!(extract_per_key_error("<DeleteResult></DeleteResult>").is_none());
    }

    #[test]
    fn bulk_unsupported_detects_documented_codes() {
        assert!(is_bulk_unsupported(&Error::StorageError(
            "status 501: NotImplemented".into(),
        )));
        assert!(is_bulk_unsupported(&Error::StorageError(
            "MethodNotAllowed".into(),
        )));
        assert!(!is_bulk_unsupported(&Error::StorageError("403".into())));
    }
}
