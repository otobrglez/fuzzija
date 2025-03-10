use clap::Parser;

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(author, version, about, long_about = None)]
pub struct AppConfig {
    #[arg(long, env, default_value = "raw-data")]
    pub storage_folder: String,
    #[arg(long, env, default_value = "indexes")]
    pub indexes_folder: String,
    #[arg(long, default_value_t = false)]
    pub force_download: bool,
    #[arg(short, long, default_value_t = false)]
    pub reindex: bool,
    #[arg(short, long)]
    pub query: Option<String>,
}

#[derive(Parser, Debug, Clone, PartialEq)]
#[command(long_about = None)] // , disable_help_flag = true
pub struct ServerConfig {
    #[arg(short, long, env, default_value_t = 8080)]
    pub port: usize,
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,
}
