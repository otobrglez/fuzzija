use crate::AppConfig;
use crate::tpconfig::{SourceConfig, SourceName};
use log::info;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub async fn collect(
    config: AppConfig,
    source_config: &SourceConfig,
) -> Result<(SourceName, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
    let file_path =
        PathBuf::from(&config.storage_folder).join(source_config.data_path.as_ref().unwrap());

    if file_path.exists() && !config.force_download {
        info!(
            "Source for \"{}\" for {} already exists. Skipping download.",
            source_config.name,
            file_path.display()
        );
        Ok((source_config.name, file_path))
    } else {
        info!(
            "Downloading file {} to {}",
            source_config.source_url,
            file_path.display()
        );

        let body = reqwest::get(source_config.source_url)
            .await
            .expect("Failed to get response");

        let mut file = tokio::fs::File::create(&file_path)
            .await
            .expect(format!("Failed to create file {}", file_path.display()).as_str());

        let body_bytes = body.bytes().await?;
        file.write_all(&body_bytes).await?;

        Ok((source_config.name, file_path))
    }
}
