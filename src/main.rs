mod indexer;
mod search;
mod sources;
mod tpconfig;

use clap::Parser;
use log::*;
use std::cmp::PartialEq;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(author, version, about, long_about = None)]
struct AppConfig {
    #[arg(short, long, default_value = "tmp")]
    storage_folder: String,
    #[arg(short, long, default_value = "indexes")]
    indexes_folder: String,
    #[arg(long, default_value_t = false)]
    force_download: bool,
    #[arg(long, default_value_t = false)]
    reindex: bool,
    #[arg(long)]
    query: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli_config = AppConfig::parse();

    let storage_folder_dir = PathBuf::from(&cli_config.storage_folder);
    if !storage_folder_dir.exists() {
        fs::create_dir_all(&storage_folder_dir)?;
    }
    let indexes_folder_dir = PathBuf::from(&cli_config.indexes_folder);
    if !indexes_folder_dir.exists() {
        fs::create_dir_all(&indexes_folder_dir)?;
    }

    let indexes = Arc::new(Mutex::new(
        indexer::open_or_create_indexes(cli_config.clone()).unwrap(),
    ));

    let index_readers = search::open_readers(&indexes).await;

    if cli_config.reindex {
        info!("Reindexing.");
        let mut collection_tasks = JoinSet::new();
        for source_config in tpconfig::CONFIG
            .iter()
            .filter(|c| c.kind != tpconfig::SourceKind::Disabled)
        {
            collection_tasks.spawn(sources::collect(cli_config.clone(), source_config));
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
                error!("Indexing failed: {:?}", e);
                continue;
            }
        }
        info!("Indexing finished.");
    }

    if let Some(query) = cli_config.query {
        info!("Searching for \"{}\"", query);
        _ = search::search_indexes(
            &indexes,
            &index_readers,
            HashSet::from(["Pravne Osebe", "Fizične osebe"]),
            query,
        )
        .await;
    }

    Ok(())
}
