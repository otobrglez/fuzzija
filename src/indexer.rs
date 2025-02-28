use crate::AppConfig;
use crate::tpconfig::{CONFIG, SourceConfig, SourceKind, SourceName};
use io::Error;
use log::info;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{BufRead, BufReader, Cursor};
use std::ops::Deref;
use std::path::PathBuf;
use std::{fs, io};
use tantivy::Index;
use tantivy::directory::MmapDirectory;
use tantivy::doc;

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

fn slice_line(input: &String, position: (usize, usize)) -> String {
    let (start, end) = position;
    input
        .chars()
        .skip(start)
        .take(end - start)
        .collect::<String>()
        .trim()
        .to_string()
}

fn index_zipped_csv_fixed_positions(
    source_config: &SourceConfig,
    index: &Index,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Indexing {}", source_config.name);
    let (_, fields) = (source_config.schema)().unwrap();

    if let Some(zip_file_path) = source_config.zip_file_path {
        let reader = BufReader::new(read_file_from_zip(path, zip_file_path)?);

        let mut index_writer = index.writer(100_000_000)?;

        let mut rows = 0;
        for line in reader.lines() {
            if let Ok(line) = line {
                let mut document = doc! {};
                for (field, position) in fields {
                    let value = slice_line(&line, *position);
                    document.add_field_value(*field, value.deref());
                }

                index_writer
                    .add_document(document)
                    .expect("Failed to add document");
                rows += 1;
            }
        }

        index_writer.commit()?;
        info!("Indexed {} for {}", rows, source_config.name);

        Ok(())
    } else {
        Ok(())
    }
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
        (SourceKind::PravneOsebe, Some(index)) => {
            index_zipped_csv_fixed_positions(source_config, &index, path)
        }
        (SourceKind::FizicneOsebe, Some(index)) => {
            index_zipped_csv_fixed_positions(source_config, &index, path)
        }
        _ => Err(Box::new(Error::new(
            io::ErrorKind::Other,
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

    if config.reindex {
        if indexes_folder.exists() {
            fs::remove_dir_all(&indexes_folder)?;
            info!(
                "Deleted existing indexes folder at {}",
                indexes_folder.display()
            );
        }
    }

    for source_config in CONFIG.iter().filter(|c| c.kind != SourceKind::Disabled) {
        if let Some((schema, _)) = (source_config.schema)() {
            info!(
                "Creating or opening index for {} in {}/{}",
                source_config.name,
                indexes_folder.display(),
                source_config.index_path.as_deref().unwrap_or("unknown")
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
