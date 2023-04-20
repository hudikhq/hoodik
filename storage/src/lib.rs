pub mod contract;
pub mod data;
pub mod providers;
pub mod repository;
pub mod routes;
pub mod storage;
pub mod streamer;

pub const CHUNK_SIZE_BYTES: u64 = 1024 * 1024 * 10;

pub use mime;

#[cfg(test)]
mod test;
