use clap::Parser;
use fuzzija::config::AppConfig;
use fuzzija::*;
use log::*;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let app_config = AppConfig::parse();

    let (_, indexes_folders) = create_directories(&app_config).unwrap();

    let indexes = Arc::new(Mutex::new(indexer::open_or_create_indexes(
        app_config.clone(),
        &indexes_folders,
    )?));

    let index_readers = search::open_readers(&indexes).await;

    if app_config.reindex {
        info!("Reindexing.");
        let mut collection_tasks = JoinSet::new();
        for source_config in tpconfig::CONFIG
            .iter()
            .filter(|c| c.kind != tpconfig::SourceKind::Disabled)
        {
            collection_tasks.spawn(sources::collect(app_config.clone(), source_config));
        }

        let mut indexing_tasks = JoinSet::new();
        while let Some(Ok(result)) = collection_tasks.join_next().await {
            let (source_name, path) = result.unwrap();
            info!("Collected data from {} to {}", source_name, path.display());

            let indexes = Arc::clone(&indexes);
            indexing_tasks.spawn(async move {
                let maybe_index = {
                    let locked_indexes = indexes.lock().await;
                    locked_indexes.get(&source_name).cloned()
                };

                if maybe_index.is_none() || 1 == 1 {
                    indexer::index_source(source_name, maybe_index, path).await
                } else {
                    info!(
                        "Index for '{}' already exists, skipping indexing.",
                        source_name
                    );
                    Ok(())
                }
            });
        }

        while let Some(Ok(result)) = indexing_tasks.join_next().await {
            if let Err(e) = result {
                error!("Indexing failed: {}", e);
                continue;
            }
        }
        info!("Indexing finished.");
    }

    if let Some(query) = app_config.query {
        info!("Searching for \"{}\"", query);
        _ = search::search_indexes(
            &indexes,
            &index_readers,
            HashSet::from([
                "Pravne Osebe",
                "Fizične osebe",
                "Poslovni Register Slovenije",
            ]),
            query,
        )
        .await;
    }

    Ok(())
}

fn create_directories(
    app_config: &AppConfig,
) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
    let storage_folder_dir = PathBuf::from(&app_config.storage_folder);
    let indexes_folder_dir = PathBuf::from(&app_config.indexes_folder);

    if !storage_folder_dir.exists() {
        fs::create_dir_all(&storage_folder_dir)?;
    }
    if !indexes_folder_dir.exists() {
        fs::create_dir_all(&indexes_folder_dir)?;
    }
    Ok((storage_folder_dir.clone(), indexes_folder_dir.clone()))
}
