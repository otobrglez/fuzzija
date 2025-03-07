use crate::indexer::IndexMap;
use crate::tpconfig::SourceName;
use log::info;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Document, IndexReader, ReloadPolicy, Score, TantivyDocument};
use tokio::sync::Mutex;

pub type ReaderMap = HashMap<SourceName, IndexReader>;
pub async fn open_readers(indexes: &Arc<Mutex<IndexMap>>) -> Arc<Mutex<ReaderMap>> {
    let locked_indexes = indexes.lock().await;
    let index_map: ReaderMap = locked_indexes
        .iter()
        .map(|(&source_name, index)| {
            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()
                .unwrap_or_else(|e| {
                    panic!("Failed to create reader for index '{}': {}", source_name, e)
                });
            (source_name, reader)
        })
        .collect();

    Arc::new(Mutex::new(index_map))
}

pub type IndexResult = (Score, NamedFieldDocument, String);
pub type SearchResults = HashMap<SourceName, Vec<IndexResult>>;

pub async fn search_indexes(
    indexes: &Arc<Mutex<IndexMap>>,
    readers: &Arc<Mutex<ReaderMap>>,
    selected_sources: HashSet<SourceName>,
    query: String,
    maybe_limit: Option<usize>,
) -> Result<SearchResults, Box<dyn std::error::Error + Send + Sync>> {
    let limit = maybe_limit.unwrap_or(10);
    let (indexes_map, readers_map) = (indexes.lock().await, readers.lock().await);

    let mut search_results: SearchResults = HashMap::new();
    selected_sources.iter().for_each(|source_name| {
        if let (Some(index), Some(reader)) =
            (indexes_map.get(source_name), readers_map.get(source_name))
        {
            info!("Searching in {} for {:#?}", source_name, query);

            let schema = index.schema();
            let all_fields: Vec<Field> = schema.fields().map(|(field, _)| field).collect();
            let query_parser = QueryParser::for_index(index, all_fields);

            let query = query_parser.parse_query(&query).unwrap();
            let searcher = reader.searcher();
            let top_docs = searcher
                .search(&query, &TopDocs::with_limit(limit))
                .unwrap();

            let mut documents: Vec<IndexResult> = Vec::new();
            for (score, doc_address) in top_docs {
                let document: TantivyDocument = searcher.doc(doc_address).unwrap();
                documents.push((
                    score,
                    document.to_named_doc(&schema),
                    document.to_json(&schema).clone(),
                ));
            }

            search_results.insert(*source_name, documents.into_iter().collect());
        }
    });

    Ok(search_results)
}
