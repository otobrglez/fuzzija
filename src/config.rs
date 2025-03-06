use clap::Parser;

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct AppConfig {
    #[arg(short, long, default_value = "tmp")]
    pub storage_folder: String,
    #[arg(short, long, default_value = "indexes")]
    pub indexes_folder: String,
    #[arg(long, default_value_t = false)]
    pub force_download: bool,
    #[arg(long, default_value_t = false)]
    pub reindex: bool,
    #[arg(long)]
    pub query: Option<String>,
}
