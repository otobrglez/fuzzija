#![allow(unused_variables, unused_imports, unused)]

use axum::extract::Query;
use axum::http::{HeaderValue, Method};
use axum::{
    Json, Router,
    response::{Html, IntoResponse, Json as JsonResponse},
    routing::get,
};
use clap::Parser;
use fuzzija::config::{AppConfig, ServerConfig};
use fuzzija::indexer;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
        .layer(cors_layer);

    let listener = tokio::net::TcpListener::bind(server_address).await.unwrap();

    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn search(search_query: Query<SearchQuery>) -> impl IntoResponse {
    info!("Searching w/ {:?}", search_query);
    let user = HashMap::from([("query", search_query.query.clone())]);
    JsonResponse(user)
}
