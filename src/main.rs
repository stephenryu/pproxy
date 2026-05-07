mod args;
mod config;
mod logging;
mod proxy;

use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let args = args::parse();
    if args.version {
        println!("pproxy {}", args::version_string());
        return Ok(());
    }

    let config_path = args.config.unwrap_or_else(|| std::path::PathBuf::from("pproxy.yaml"));
    let conf = config::read_config(&config_path)
        .with_context(|| format!("Error reading config file: {}", config_path.display()))?;

    logging::setup_logging(&conf)?;
    proxy::run(conf).await
}
