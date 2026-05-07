use anyhow::{Context, Result};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ProxyRule {
    #[serde(default = "default_listen_host", rename = "listen_host")]
    pub(crate) listen_host: String,
    #[serde(rename = "listen_port")]
    pub(crate) listen_port: u16,
    #[serde(rename = "target_host")]
    pub(crate) target_host: String,
    #[serde(rename = "target_port")]
    pub(crate) target_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Config {
    #[serde(default, rename = "proxy_list")]
    pub(crate) proxy_list: Vec<ProxyRule>,

    #[serde(default = "default_log_dir", rename = "log_dir")]
    pub(crate) log_dir: String,
    #[serde(default = "default_log_file", rename = "log_file")]
    pub(crate) log_file: String,
    #[serde(default = "default_log_max_size", rename = "log_max_size")]
    pub(crate) log_max_size: usize,
    #[serde(default = "default_log_max_backups", rename = "log_max_backups")]
    pub(crate) log_max_backups: usize,
    #[serde(default = "default_log_max_age", rename = "log_max_age")]
    pub(crate) log_max_age: u64,
}

fn default_log_dir() -> String {
    "./logs".to_string()
}

fn default_listen_host() -> String {
    "0.0.0.0".to_string()
}

fn default_log_file() -> String {
    "log.log".to_string()
}

fn default_log_max_size() -> usize {
    2
}

fn default_log_max_backups() -> usize {
    100
}

fn default_log_max_age() -> u64 {
    28
}

pub(crate) fn read_config(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let conf: Config = serde_yaml::from_str(&data)
        .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;
    Ok(conf)
}
