use actix_web::web;
use error::AppResult;
use futures_util::stream::StreamExt;
use std::pin::Pin;

/// Wrapper around a stream of bytes.
/// Because it cannot be implemented directly on the trait
pub struct Streamer {
    pub(crate) inner: Pin<Box<dyn futures_util::Stream<Item = AppResult<actix_web::web::Bytes>>>>,
    pub(crate) map_fn: Box<dyn FnMut(AppResult<web::Bytes>) -> AppResult<web::Bytes>>,
}

impl Streamer {
    /// Set the streamer inner stream provider
    pub fn new<S>(stream: S) -> Self
    where
        S: futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> + 'static,
    {
        Self {
            inner: Pin::from(Box::new(stream)),
            map_fn: Box::new(|data| data),
        }
    }

    /// Create an empty streamer
    pub fn empty() -> Self {
        Self::new(futures_util::stream::empty())
    }

    /// Create a streamer with a single chunk
    pub fn once(data: actix_web::web::Bytes) -> Self {
        Self::new(futures_util::stream::once(async move { Ok(data) }))
    }

    /// Create a streamer with an error
    pub fn once_error(err: AppResult<actix_web::web::Bytes>) -> Self {
        Self::new(futures_util::stream::once(async move { err }))
    }

    /// Load the inner stream
    pub fn load<S>(mut self, stream: S) -> Self
    where
        S: futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> + 'static,
    {
        self.inner = Pin::from(Box::new(stream));

        self
    }

    /// Map the inner stream with a custom function
    pub fn map<F>(mut self, f: F) -> Self
    where
        F: FnMut(AppResult<web::Bytes>) -> AppResult<web::Bytes> + 'static,
    {
        self.map_fn = Box::new(f);

        self
    }

    /// Get the inner stream to pass it to the actix_web::HttpResponse::streaming
    pub fn stream(self) -> impl futures_util::Stream<Item = AppResult<actix_web::web::Bytes>> {
        self.inner.map(self.map_fn)
    }
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_create_stream() {
        use super::Streamer;
        use futures_util::stream::StreamExt;

        let stream = Streamer::new(futures_util::stream::once(async move {
            Ok(actix_web::web::Bytes::from_static(b"test"))
        }));

        let mut stream = stream.stream();

        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            actix_web::web::Bytes::from_static(b"test")
        );
    }

    #[tokio::test]
    async fn test_map_stream() {
        use super::Streamer;
        use futures_util::stream::StreamExt;

        let stream = Streamer::new(futures_util::stream::once(async move {
            Ok(actix_web::web::Bytes::from_static(b"test"))
        }))
        .map(|_data| Ok(actix_web::web::Bytes::from_static(b"test2")));

        let mut stream = stream.stream();

        assert_eq!(
            stream.next().await.unwrap().unwrap(),
            actix_web::web::Bytes::from_static(b"test2")
        );
    }
}
