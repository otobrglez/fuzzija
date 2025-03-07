use clap::Parser;
use fuzzija::config::AppConfig;
use fuzzija::tpconfig::SourceName;
use fuzzija::*;
use log::*;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let app_config = AppConfig::parse();
    let (index_map, reader_map) = indexer::init(&app_config).await?;

    if app_config.reindex {
        info!("Reindexing.");
        let mut collection_tasks = JoinSet::new();
        for (_, source_config) in tpconfig::available_sources() {
            collection_tasks.spawn(sources::collect(app_config.clone(), &source_config));
        }

        let mut indexing_tasks = JoinSet::new();
        while let Some(Ok(result)) = collection_tasks.join_next().await {
            let (source_name, path) = result.unwrap();
            info!("Collected data from {} to {}", source_name, path.display());

            let indexes = Arc::clone(&index_map);
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
        let search_results = search::search_indexes(
            &index_map,
            &reader_map,
            HashSet::from([
                SourceName::PravneOsebe,
                SourceName::FizicneOsebe,
                SourceName::PoslovniRegisterSlovenije,
            ]),
            query,
            None,
        )
        .await?;

        for (source_name, results) in search_results {
            println!("{}:", source_name);
            for (score, _, json_document) in results {
                println!("\t- {:.2} {}", score, json_document);
            }
        }
    }

    Ok(())
}
