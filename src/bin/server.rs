use axum::extract::Query;
use axum::http::Method;
use axum::{
    Router,
    response::{IntoResponse, Json},
    routing::get,
};
use clap::Parser;
use fuzzija::config::{AppConfig, ServerConfig};
use fuzzija::indexer::IndexMap;
use fuzzija::search::{ReaderMap, SearchResults};
use fuzzija::tpconfig::SourceName;
use fuzzija::{indexer, search};
use log::{error, info};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

#[derive(Parser, Debug)]
struct Config {
    #[command(flatten)]
    app: AppConfig,
    #[command(flatten)]
    server: ServerConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchQuery {
    query: String,
    limit: Option<usize>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let config: Config = Config::parse();
    let server_config = config.server;
    let app_config = config.app;
    let server_address = format!("{}:{}", server_config.host, server_config.port);
    info!("Booting server on {}", server_address);

    let (index_map, reader_map) = indexer::init(&app_config).await?;

    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET]);

    let app = Router::new()
        .route("/", get(|| async { "Ok." }))
        .route("/search", get(search))
        .layer(cors_layer)
        .with_state((index_map, reader_map));

    let listener = tokio::net::TcpListener::bind(server_address).await.unwrap();

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Shutdown handler
    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM signal");
                shutdown_clone.store(true, Ordering::SeqCst);
            }
            _ = sigint.recv() => {
                info!("Received SIGINT signal");
                shutdown_clone.store(true, Ordering::SeqCst);
            }
        }
    });

    // Server
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            while !shutdown.load(Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            info!("Shutting down server gracefully...");
        })
        .await?;

    info!("Server shutdown complete");

    Ok(())
}

async fn search(
    state: axum::extract::State<(Arc<Mutex<IndexMap>>, Arc<Mutex<ReaderMap>>)>,
    search_query: Query<SearchQuery>,
) -> axum::response::Result<impl IntoResponse> {
    let (indexes, readers) = state.0;

    let query: String = search_query.query.clone();
    let maybe_limit: Option<usize> = search_query.limit.clone();

    let selected_sources = HashSet::from([
        SourceName::PravneOsebe,
        SourceName::FizicneOsebe,
        SourceName::PoslovniRegisterSlovenije,
    ]);

    match search::search_indexes(
        &indexes,
        &readers,
        selected_sources,
        query.clone(),
        maybe_limit,
    )
    .await
    {
        Ok(res) => Ok(Json(results_to_json(res))),
        Err(err) => {
            error!(
                "Failed to search indexes: {} with {:#?}. Returning empty response",
                err, query
            );
            Ok(Json(SearchResult { results: vec![] }))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DocumentResult {
    pub source_name: String,
    pub document: Value,
    pub score: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    pub results: Vec<DocumentResult>,
}

fn results_to_json(search_results: SearchResults) -> SearchResult {
    let mut results = Vec::new();
    for (source_name, documents) in search_results {
        for (score, _, json_document) in documents {
            if let Ok(doc_value) = serde_json::from_str(&json_document) {
                results.push(DocumentResult {
                    source_name: source_name.to_string(),
                    document: doc_value,
                    score,
                });
            }
        }
    }

    SearchResult { results }
}
