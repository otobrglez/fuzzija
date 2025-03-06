use crate::config::AppConfig;
use crate::tpconfig::{SourceConfig, SourceKind, SourceName};
use log::info;
use scraper::{Html, Selector};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::task;

async fn get_first_zip_link_safe(
    source_config: &SourceConfig,
    data_file_path: &PathBuf,
) -> Result<(SourceName, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
    info!("Fetching download page for PR");
    // Fetch the HTML content of the source URL
    let response = reqwest::get(source_config.source_url).await?;
    let html_content = response.text().await?;

    // Parse the HTML content and extract the necessary result inside a blocking thread
    let download_url = task::spawn_blocking(
        move || -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            let document = Html::parse_document(&html_content);
            let selector = Selector::parse("a[href$='.zip']").unwrap();

            if let Some(element) = document.select(&selector).next() {
                let href = element
                    .value()
                    .attr("href")
                    .ok_or_else(|| "No href attribute found".to_string())?;
                Ok(href.to_string())
            } else {
                Err("No ZIP file link found on the page.".into())
            }
        },
    )
    .await??;

    // If the download URL is relative, resolve it to an absolute URL
    let download_url = if download_url.starts_with("http") {
        download_url
    } else {
        let base_url = reqwest::Url::parse(&source_config.source_url)?;
        base_url.join(&download_url)?.to_string()
    };

    info!("Found download link: {}. Starting download.", download_url);

    // Fetch the ZIP file from the resolved download URL
    let body = reqwest::get(&download_url).await?;
    let mut file = tokio::fs::File::create(data_file_path).await?;
    let body_bytes = body.bytes().await?;
    file.write_all(&body_bytes).await?;

    info!("Download from {} completed.", download_url);

    Ok((source_config.name, data_file_path.clone()))
}

pub async fn collect(
    config: AppConfig,
    source_config: &SourceConfig,
) -> Result<(SourceName, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
    let data_file_path =
        PathBuf::from(&config.storage_folder).join(source_config.data_path.as_ref().unwrap());
    match (
        source_config.kind,
        data_file_path.exists(),
        config.force_download,
    ) {
        (source_kind @ SourceKind::PoslovniRegisterSlovenije, false, _) => {
            info!("Source kind {} does not exist, downloading.", source_kind);
            get_first_zip_link_safe(source_config, &data_file_path).await
        }
        (source_kind @ SourceKind::PoslovniRegisterSlovenije, true, true) => {
            info!("Source kind {} exists, downloading again.", source_kind);
            tokio::fs::remove_file(&data_file_path).await?;
            get_first_zip_link_safe(source_config, &data_file_path).await
        }
        (source_kind, false, _) => {
            info!("Source kind {} does not exist, downloading.", source_kind);

            let body = reqwest::get(source_config.source_url).await?;
            let mut file = tokio::fs::File::create(&data_file_path).await?;
            let body_bytes = body.bytes().await?;
            file.write_all(&body_bytes).await?;

            Ok((source_config.name, data_file_path))
        }
        (source_kind, true, true) => {
            info!("Source kind {} exists, forcing download.", source_kind);

            // Removing existing one
            tokio::fs::remove_file(&data_file_path).await?;

            let body = reqwest::get(source_config.source_url).await?;
            let mut file = tokio::fs::File::create(&data_file_path).await?;
            let body_bytes = body.bytes().await?;
            file.write_all(&body_bytes).await?;

            Ok((source_config.name, data_file_path))
        }
        (_, true, false) => {
            info!(
                "Source for \"{}\" already exists as {}. Skipping download.",
                source_config.name,
                data_file_path.display()
            );
            Ok((source_config.name, data_file_path))
        }
    }
}
