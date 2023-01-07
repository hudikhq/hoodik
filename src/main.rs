use config::Config;
use context::Context;
use migration::{Migrator, MigratorTrait};
use std::sync::Arc;

#[actix_web::main]
async fn main() {
    // Catch any panic from any thread running and dump it here
    // This enables us to kill the entire process if any of the inner threads die
    let origin_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        origin_hook(panic_info);
        std::process::exit(1);
    }));

    // Initialize the configuration of the app
    let config = Config::new(
        "My app",
        env!("CARGO_PKG_VERSION"),
        "This is a first try at a drive app",
    );

    env_logger::init();

    // Create context from the config
    let context = Arc::new(Context::new(config).await.unwrap());

    // Run database migrations
    Migrator::up(&context.db, None).await.unwrap();
}
