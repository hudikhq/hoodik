use config::Config;
use error::AppResult;

fn human_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    if bytes == 0 {
        return "0 B".to_string();
    }
    let i = (bytes as f64).log(1024.0).floor() as usize;
    let i = i.min(UNITS.len() - 1);
    let val = bytes as f64 / 1024_f64.powi(i as i32);
    format!("{:.2} {}", val, UNITS[i])
}

/// Migrate all file chunks from local filesystem to S3.
/// Idempotent — skips files that already exist in S3, safe to re-run.
pub async fn migrate_storage(config: &Config) -> AppResult<()> {
    let s3_config = config.s3.as_ref().ok_or_else(|| {
        error::Error::InternalError(
            "S3 configuration is required for migrate-storage. \
             Set S3_BUCKET, S3_ACCESS_KEY, S3_SECRET_KEY."
                .to_string(),
        )
    })?;

    let data_dir = &config.app.data_dir;
    let pattern = format!("{}/*.part.*", data_dir);

    println!("Scanning for chunks in {}...", data_dir);

    let paths: Vec<_> = glob::glob(&pattern)
        .map_err(|e| error::Error::InternalError(format!("Invalid glob pattern: {}", e)))?
        .filter_map(|p| p.ok())
        .collect();

    if paths.is_empty() {
        println!("No chunk files found. Nothing to migrate.");
        return Ok(());
    }

    println!("Found {} chunk files.", paths.len());

    let s3 = fs::prelude::S3Provider::new(s3_config);
    let prefix = s3_config.prefix.as_deref().unwrap_or("");

    let mut migrated: u64 = 0;
    let mut migrated_bytes: u64 = 0;
    let mut skipped: u64 = 0;

    for path in &paths {
        let file_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        let key = format!("{}{}", prefix, file_name);

        let already_exists = match s3.bucket().head_object(&key).await {
            Ok((_, code)) => code == 200,
            Err(_) => false,
        };

        if already_exists {
            skipped += 1;
            continue;
        }

        let data = tokio::fs::read(path).await.map_err(|e| {
            error::Error::StorageError(format!("Failed to read {}: {}", file_name, e))
        })?;

        let size = data.len() as u64;

        s3.bucket().put_object(&key, &data).await.map_err(|e| {
            error::Error::StorageError(format!("Failed to upload {} to S3: {}", key, e))
        })?;

        migrated += 1;
        migrated_bytes += size;

        if migrated.is_multiple_of(100) {
            println!("  {} chunks migrated ({})...", migrated, human_bytes(migrated_bytes));
        }
    }

    println!();
    println!("Done: {} migrated ({}), {} skipped.", migrated, human_bytes(migrated_bytes), skipped);

    Ok(())
}
