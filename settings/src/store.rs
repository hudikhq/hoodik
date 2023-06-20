use config::Config;
use error::AppResult;
use fs::prelude::*;

use crate::data::Data;

/// Get the filename of the settings file.
fn get_filename() -> Filename {
    Filename::new("settings").with_extension("json")
}

/// Read the settings from the filesystem.
pub(crate) async fn read(config: &Config) -> AppResult<Data> {
    let fs = Fs::new(config);
    let provider = fs.local();

    let filename = get_filename();

    let data = provider.read(&filename).await?;
    let inner = serde_json::from_slice::<Data>(&data)?;

    Ok(inner)
}

/// Write the settings to the filesystem.
pub(crate) async fn write(config: &Config, inner: &Data) -> AppResult<()> {
    let fs = Fs::new(config);
    let provider = fs.local();

    let filename = get_filename();
    let data = inner.to_vec()?;

    provider.write(&filename, data.as_slice()).await
}
