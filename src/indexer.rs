use crate::tpconfig::{CONFIG, SourceConfig, SourceKind, SourceName};
use crate::{AppConfig, sources, tpconfig};
use log::{info, warn};
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{BufRead, BufReader, Cursor};
use std::ops::Deref;
use std::path::PathBuf;
use std::{fs, io};
use tantivy::Index;
use tantivy::directory::MmapDirectory;
use tantivy::doc;
use tantivy::schema::*;
use tantivy::schema::{STORED, Schema, TEXT};

fn read_file_from_zip(
    zip_path: PathBuf,
    file_path: &str,
) -> Result<BufReader<Cursor<Vec<u8>>>, Box<dyn std::error::Error + Send + Sync>> {
    let zip_file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;
    let mut file = archive.by_name(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(BufReader::new(Cursor::new(buffer)))
}

fn index_pravne_osebe(
    source_config: &SourceConfig,
    index: &Index,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Beginning indexing pravne osebe from {}",
        source_config.name
    );
    let (schema, fields) = (source_config.schema)().unwrap();
    let reader = BufReader::new(read_file_from_zip(path, "DURS_zavezanci_PO.txt")?);
    let vat_field = schema.get_field("vat_id").unwrap();
    let name_field = schema.get_field("company_id").unwrap();
    let company_field = schema.get_field("company_name").unwrap();

    let mut index_writer = index.writer(50_000_000)?;

    let mut rows = 0;
    for line in reader.lines() {
        if let Ok(line) = line {
            let vat_id = line.chars().skip(4).take(9).collect::<String>().trim().to_string();
            let company_id = line.chars().skip(13).take(10).collect::<String>().trim().to_string();
            let company_name = line.chars().skip(42).take(102).collect::<String>().trim().to_string();

            let document = doc!(
                vat_field => vat_id,
                name_field => company_id,
                company_field => company_name
            );

            index_writer
                .add_document(document)
                .expect("Failed to add document");
            rows += 1;
        }
    }

    // index_writer.commit()?;
    info!("Indexed {} rows w/ pravne osebe", rows);

    Ok(())
}

fn index_fizicne_osebe(
    source_config: &SourceConfig,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Beginning indexing fizicne osebe from {}",
        source_config.name
    );
    let reader = BufReader::new(read_file_from_zip(path, "DURS_zavezanci_FO.txt")?);
    let mut rows = 0;
    for line in reader.lines() {
        if let Ok(line) = line {
            // Process each line here, e.g., log it or parse it
            rows += 1;
        }
    }

    info!("Indexed {} rows w/ fizicne osebe", rows);

    Ok(())
}

pub async fn index_source(
    source_name: SourceName,
    maybe_index: Option<Index>,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let source_config = CONFIG
        .iter()
        .find(|config| config.name == source_name)
        .unwrap();

    match (source_config.kind, maybe_index) {
        (SourceKind::PravneOsebe, Some(index)) => index_pravne_osebe(source_config, &index, path),
        (SourceKind::FizicneOsebe, _) => index_fizicne_osebe(source_config, path),
        _ => Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Functionality not yet implemented",
        ))),
    }
}

pub type IndexMap = HashMap<SourceName, Index>;
pub fn open_or_create_indexes(
    config: AppConfig,
) -> Result<IndexMap, Box<dyn std::error::Error + Send + Sync>> {
    let indexes_folder = PathBuf::from(config.indexes_folder);
    let mut indexes = HashMap::new();

    for source_config in CONFIG.iter().filter(|c| c.kind != SourceKind::Disabled) {
        if let Some((schema, _)) = (source_config.schema)() {
            info!(
                "Creating index for {} in {}",
                source_config.name,
                indexes_folder.display()
            );

            let index_path_raw = source_config
                .index_path
                .or_else(|| Some("unknown"))
                .unwrap();

            let index_path = indexes_folder.join(index_path_raw);
            if !index_path.exists() {
                fs::create_dir_all(&index_path)?;
            }

            let directory = MmapDirectory::open(&index_path)?;
            let index = Index::open_or_create(directory, schema.clone())?;

            indexes.insert(source_config.name, index);
        }
    }

    Ok(indexes)
}
