mod client {
    include!(concat!(env!("OUT_DIR"), "/client.rs"));
}
pub mod migrate;
pub mod server;

pub use config::Config;
pub use context::Context;
