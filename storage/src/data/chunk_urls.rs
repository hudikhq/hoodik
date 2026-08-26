use serde::{Deserialize, Serialize};

/// One chunk's presigned URL. The index travels with it so a client never has
/// to infer position from order.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkUrl {
    pub chunk: i64,
    pub url: String,
}

/// Every URL a client needs to move one file, handed over in a single
/// response.
///
/// Whole-file rather than a window because a mobile client puts the entire set
/// into the OS transfer queue at once and is then free to be suspended, or
/// killed, for as long as the transfer takes. Anything that made it come back
/// mid-transfer for more URLs would need the app alive to do it, which is
/// exactly what background transfer is not.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChunkUrls {
    pub urls: Vec<ChunkUrl>,

    /// Unix seconds after which every URL above stops working. Clients hold
    /// this so they can renew before a transfer dies rather than discovering
    /// it as a failed chunk.
    pub expires_at: i64,
}

impl ChunkUrls {
    pub fn new(chunks: &[i64], urls: Vec<String>, expires_at: i64) -> Self {
        Self {
            urls: chunks
                .iter()
                .zip(urls)
                .map(|(chunk, url)| ChunkUrl { chunk: *chunk, url })
                .collect(),
            expires_at,
        }
    }
}

/// One chunk a client intends to write, and how many bytes it will be.
///
/// The length is not advisory: it gets signed into the URL, so the store
/// rejects a write of any other size. It is also what the quota is charged
/// against before a single byte is allowed into the bucket.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingChunk {
    pub chunk: i64,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadUrlsRequest {
    pub chunks: Vec<PendingChunk>,
}
