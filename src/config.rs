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

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(long_about = None)] // , disable_help_flag = true
pub struct ServerConfig {
    #[arg(short, long, default_value_t = 8080)]
    pub port: usize,
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,
}
