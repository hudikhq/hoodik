use error::AppResult;
use hoodik::{Config, Context};
use migration::{Migrator, MigratorTrait};

#[actix_web::main]
async fn main() -> AppResult<()> {
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

    // Create context from the config
    let context = Context::new(config).await?;

    // Run database migrations
    Migrator::up(&context.db, None).await?;

    // Init logger
    env_logger::init();
    log::info!("Starting server");
    log::warn!("This is a warning");
    log::error!("This is a error");
    log::debug!("This is a debug");

    // Start the server
    hoodik::server::engage(context).await
}
