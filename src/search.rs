use crate::indexer::IndexMap;
use crate::tpconfig::SourceName;
use log::info;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::{Document, IndexReader, ReloadPolicy, TantivyDocument};
use tokio::sync::Mutex;

type ReaderMap = HashMap<SourceName, IndexReader>;

pub async fn open_readers(
    indexes: &Arc<Mutex<IndexMap>>,
) -> Arc<Mutex<HashMap<SourceName, IndexReader>>> {
    let locked_indexes = indexes.lock().await;
    let ind = locked_indexes
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
        .collect::<HashMap<_, _>>();
    Arc::new(Mutex::new(ind))
}

pub async fn search_indexes(
    indexes: &Arc<Mutex<IndexMap>>,
    readers: &Arc<Mutex<ReaderMap>>,
    source_names: HashSet<SourceName>,
    query: String,
) {
    let locked_indexes = indexes.lock().await;
    let locked_readers = readers.lock().await;

    for (source_name, index) in locked_indexes
        .iter()
        .filter(|(source_name, _)| source_names.contains(*source_name))
    {
        for (source_name_2, reader) in locked_readers
            .iter()
            .filter(|(source_name_2, _)| source_names.contains(*source_name_2))
        {
            if source_names.contains(source_name_2) && source_name == source_name_2 {
                info!("Searching for {} in {}", query, source_name);

                let schema = index.schema();
                let all_fields = schema.fields().map(|(field, _)| field).collect::<Vec<_>>();

                let query_parser = QueryParser::for_index(&index, all_fields);

                let query = query_parser.parse_query(&query).unwrap();

                let searcher = reader.searcher();
                let top_docs = searcher.search(&query, &TopDocs::with_limit(10)).unwrap();

                for (_score, doc_address) in top_docs {
                    let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
                    println!("{}", retrieved_doc.to_json(&schema));
                }
            }
        }
    }
}
