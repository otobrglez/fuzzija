use crate::config::AppConfig;
use crate::search::ReaderMap;
use crate::tpconfig::*;
use crate::{search, tpconfig};
use csv::ReaderBuilder;
use encoding_rs::WINDOWS_1252;
use io::Error;
use log::info;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{BufRead, BufReader, Cursor};
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, io};
use tantivy::Index;
use tantivy::directory::MmapDirectory;
use tantivy::doc;
use tokio::sync::Mutex;

fn read_by_name_from_zip(
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

fn read_first_csv_from_zip(
    zip_path: PathBuf,
) -> Result<BufReader<Cursor<Vec<u8>>>, Box<dyn std::error::Error + Send + Sync>> {
    let zip_file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(zip_file)?;

    let file_names = (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|file| file.name().to_string()))
        .collect::<Vec<_>>();

    if let Some(first_csv) = file_names.iter().find(|name| name.ends_with(".csv")) {
        let mut file = archive.by_name(first_csv)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        let (decoded, _, had_errors) = WINDOWS_1252.decode(&buffer);
        if had_errors {
            return Err(Box::new(Error::new(
                io::ErrorKind::InvalidData,
                "Failed to decode file with Windows-1252 encoding",
            )));
        }

        Ok(BufReader::new(Cursor::new(
            decoded.into_owned().into_bytes(),
        )))
    } else {
        Err(Box::new(Error::new(
            io::ErrorKind::NotFound,
            "No CSV files found in the zip archive",
        )))
    }
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
        let reader = BufReader::new(read_by_name_from_zip(path, zip_file_path)?);

        let mut index_writer = index.writer(100_000_000)?;

        let mut rows = 0;
        for line in reader.lines() {
            if let Ok(line) = line {
                let mut document = doc! {};
                for (field, position) in fields {
                    let Position::Fixed(start, stop) = position else {
                        panic!()
                    };
                    let value = slice_line(&line, (start.clone(), stop.clone()));
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

fn index_zipped_csv_with_header(
    source_config: &SourceConfig,
    index: &Index,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Indexing {}", source_config.name);

    let (_, fields) = (source_config.schema)().unwrap();
    let reader = read_first_csv_from_zip(path)?;
    let mut csv_reader = ReaderBuilder::new().has_headers(true).from_reader(reader);

    let mut index_writer = index.writer(100_000_000)?;
    for (_, result) in csv_reader.records().enumerate() {
        if let Ok(record) = result {
            let csv_fields: Vec<String> = record.iter().map(String::from).collect();
            let mut document = doc! {};

            for (field, position) in fields {
                let Position::Index(position_index) = position else {
                    panic!()
                };
                let value = csv_fields.get(position_index.clone()).unwrap();
                document.add_field_value(*field, value.deref());
            }

            index_writer.add_document(document)?;
        }
    }
    index_writer.commit()?;

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
        (SourceKind::PravneOsebe | SourceKind::FizicneOsebe, Some(index)) => {
            index_zipped_csv_fixed_positions(source_config, &index, path)
        }
        (SourceKind::PoslovniRegisterSlovenije, Some(index)) => {
            index_zipped_csv_with_header(source_config, &index, path)
        }
        (source_kind, _) => Err(Box::new(Error::new(
            io::ErrorKind::Other,
            format!("Functionality not yet implemented for {}", source_kind),
        ))),
    }
}

pub type IndexMap = HashMap<SourceName, Index>;
pub fn open_or_create_indexes(
    config: &AppConfig,
    indexes_folder: &PathBuf,
) -> Result<IndexMap, Box<dyn std::error::Error + Send + Sync>> {
    let mut indexes: IndexMap = HashMap::new();

    if config.reindex {
        if indexes_folder.exists() {
            fs::remove_dir_all(indexes_folder)?;
            info!(
                "Deleted existing indexes folder at {}",
                indexes_folder.display()
            );
        }
    }

    for (_, source_config) in tpconfig::available_sources() {
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

            // let index = Index::create_in_ram(schema.clone());

            let index = Index::open_or_create(directory, schema.clone())?;

            indexes.insert(source_config.name, index);
        }
    }

    Ok(indexes)
}

pub fn create_directories(
    app_config: &AppConfig,
) -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error + Send + Sync>> {
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

pub async fn init(
    app_config: &AppConfig,
) -> Result<(Arc<Mutex<IndexMap>>, Arc<Mutex<ReaderMap>>), Box<dyn std::error::Error + Send + Sync>>
{
    let (_, indexes_folders) = create_directories(&app_config)?;

    let index_map = Arc::new(Mutex::new(open_or_create_indexes(
        &app_config,
        &indexes_folders,
    )?));

    let reader_map = search::open_readers(&index_map).await;

    Ok((index_map, reader_map))
}
