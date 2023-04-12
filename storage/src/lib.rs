pub mod contract;
pub mod data;
pub mod providers;
pub mod repository;
pub mod routes;
pub mod storage;

pub const CHUNK_SIZE_BYTES: u64 = 1024 * 1024;

pub use actix_multipart as multipart;
pub use mime;

#[cfg(test)]
mod test;
