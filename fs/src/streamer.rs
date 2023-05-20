use error::AppResult;
use std::pin::Pin;

/// Wrapper around a stream of bytes.
/// Because it cannot be implemented directly on the trait
pub struct Streamer {
    pub(crate) inner: Pin<Box<dyn futures_util::Stream<Item = AppResult<actix_web::web::Bytes>>>>,
}

impl Streamer {
    /// Set the streamer inner stream provider
    pub fn new<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> + 'static,
    {
        Self {
            inner: Pin::from(Box::new(stream)),
        }
    }

    /// Get the inner stream to pass it to the actix_web::HttpResponse::streaming
    pub fn stream(self) -> impl futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> {
        self.inner
    }
}
