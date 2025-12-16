use anyhow::{Context, Result};
use clap::Parser;
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use log::{error, info};
use serde::Deserialize;
use std::{
    ffi::OsStr,
    fs,
    io::ErrorKind,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::SystemTime,
};
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};

#[derive(Debug, Deserialize, Clone)]
struct ProxyRule {
    #[serde(rename = "listen_port")]
    listen_port: u16,
    #[serde(rename = "target_host")]
    target_host: String,
    #[serde(rename = "target_port")]
    target_port: u16,
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    #[serde(default, rename = "proxy_list")]
    proxy_list: Vec<ProxyRule>,

    #[serde(default = "default_log_dir", rename = "log_dir")]
    log_dir: String,
    #[serde(default = "default_log_file", rename = "log_file")]
    log_file: String,
    #[serde(default = "default_log_max_size", rename = "log_max_size")]
    log_max_size: usize,
    #[serde(default = "default_log_max_backups", rename = "log_max_backups")]
    log_max_backups: usize,
    #[serde(default = "default_log_max_age", rename = "log_max_age")]
    log_max_age: u64,
}

fn default_log_dir() -> String {
    "./logs".to_string()
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

#[derive(Debug, Parser)]
#[command(name = "pproxy", disable_help_subcommand = true)]
struct Args {
    /// Config file name (same as Go: -config/--config)
    #[arg(long)]
    config: Option<PathBuf>,

    /// Show the application version
    #[arg(long)]
    version: bool,
}

fn preprocess_args() -> Vec<std::ffi::OsString> {
    // Support Go-style flags like `-config foo.yaml` and `-version`.
    // Also support `-v`, `--v`, `-ver`, `--ver`.
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    let mut iter = std::env::args_os();
    if let Some(bin) = iter.next() {
        out.push(bin);
    }
    for arg in iter {
        match arg.to_string_lossy().as_ref() {
            "-config" => out.push("--config".into()),
            "-version" => out.push("--version".into()),
            "-v" | "--v" | "-ver" | "--ver" => out.push("--version".into()),
            _ => out.push(arg),
        }
    }
    out
}

fn version_string() -> String {
    // Print just version unless extra build info is set.
    let v = env!("CARGO_PKG_VERSION");
    let commit = option_env!("PPROXY_COMMIT").unwrap_or("");
    let build_unix = option_env!("PPROXY_BUILD_UNIX").unwrap_or("0");

    if commit.is_empty() || build_unix == "0" {
        return v.to_string();
    }

    let formatted = build_unix
        .parse::<i64>()
        .ok()
        .and_then(|ts| {
            chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
                .map(|dt| dt.with_timezone(&chrono::Local))
        })
        .map(|dt| dt.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| build_unix.to_string());

    format!("{v} ({commit} {formatted})")
}

fn read_config(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file: {}", path.display()))?;
    let conf: Config = serde_yaml::from_str(&data)
        .with_context(|| format!("failed to parse YAML config: {}", path.display()))?;
    Ok(conf)
}

fn setup_logging(conf: &Config) -> Result<()> {
    fs::create_dir_all(&conf.log_dir)
        .with_context(|| format!("failed to create log_dir: {}", conf.log_dir))?;

    let log_path = Path::new(&conf.log_file);
    let (basename, suffix) = split_basename_suffix(log_path);

    let file_spec = FileSpec::default()
        .directory(&conf.log_dir)
        .basename(&basename)
        .suffix(&suffix);

    // Interpret log_max_size as MiB.
    let max_bytes: u64 = (conf.log_max_size as u64).saturating_mul(1024 * 1024);

    Logger::try_with_str("info")?
        .duplicate_to_stdout(Duplicate::All)
        .log_to_file(file_spec)
        .rotate(
            Criterion::Size(max_bytes),
            Naming::Numbers,
            Cleanup::KeepLogFiles(conf.log_max_backups),
        )
        .start()?;

    cleanup_old_logs(&conf.log_dir, &basename, conf.log_max_age)?;
    Ok(())
}

fn split_basename_suffix(file: &Path) -> (String, String) {
    let basename = file
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("pproxy")
        .to_string();
    let suffix = file
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("log")
        .to_string();
    (basename, suffix)
}

fn cleanup_old_logs(log_dir: &str, basename: &str, max_age_days: u64) -> Result<()> {
    // Best-effort cleanup: remove rotated files older than max_age_days.
    if max_age_days == 0 {
        return Ok(());
    }

    let dir = Path::new(log_dir);
    let cutoff = SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(max_age_days * 24 * 60 * 60))
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };

        if !name.starts_with(basename) {
            continue;
        }

        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        if modified < cutoff {
            let _ = fs::remove_file(&path);
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse_from(preprocess_args());
    if args.version {
        println!("pproxy {}", version_string());
        return Ok(());
    }

    let config_path = args.config.unwrap_or_else(|| PathBuf::from("pproxy.yaml"));
    let conf = read_config(&config_path)
        .with_context(|| format!("Error reading config file: {}", config_path.display()))?;

    setup_logging(&conf)?;

    let pproxys = conf.proxy_list.len();
    info!("PProxy Starting... {pproxys} proxy(s) configured");

    if pproxys == 0 {
        info!("No proxies configured. Exiting.");
        return Ok(());
    }

    let mut started = 0usize;
    for rule in conf.proxy_list.clone() {
        match bind_listener(&rule).await? {
            Some(listener) => {
                started += 1;
                tokio::spawn(async move {
                    if let Err(err) = start_proxy_server(rule, listener).await {
                        error!("proxy server exited with error: {err:#}");
                    }
                });
            }
            None => {
                // bind_listener already logged why it was skipped.
            }
        }
    }

    if started == 0 {
        error!("No proxy listeners could be started. Exiting.");
        return Ok(());
    }

    wait_for_shutdown_signal().await;
    info!("Shutting down...");
    Ok(())
}

async fn bind_listener(rule: &ProxyRule) -> Result<Option<TcpListener>> {
    let listen_addr: SocketAddr = format!("0.0.0.0:{}", rule.listen_port)
        .parse()
        .context("invalid listen address")?;

    let listener = match TcpListener::bind(listen_addr).await {
        Ok(listener) => listener,
        Err(err) if err.kind() == ErrorKind::AddrInUse => {
            error!(
                "Port {} is already in use; cannot bind {}. Stop the process using it or change listen_port.",
                rule.listen_port, listen_addr
            );
            return Ok(None);
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to bind to {listen_addr}"));
        }
    };

    info!("Proxy server started on : {}", rule.listen_port);
    Ok(Some(listener))
}

async fn start_proxy_server(rule: ProxyRule, listener: TcpListener) -> Result<()> {
    info!(
        "Starting proxy server on port: {} with target server port: {}",
        rule.listen_port, rule.target_port
    );

    loop {
        let (client, remote_addr) = listener.accept().await?;
        info!("New client connected {remote_addr}");

        let target = format!("{}:{}", rule.target_host, rule.target_port);
        tokio::spawn(async move {
            if let Err(err) = handle_connection(client, &target, remote_addr).await {
                error!("connection error: {err:#}");
            }
        });
    }
}

async fn handle_connection(
    mut client: TcpStream,
    target_addr: &str,
    remote_addr: SocketAddr,
) -> Result<()> {
    info!("Connecting to target server: {target_addr}");

    let mut target = TcpStream::connect(target_addr)
        .await
        .with_context(|| format!("failed to connect to target {target_addr}"))?;

    let _ = copy_bidirectional(&mut client, &mut target).await;

    info!("Client disconnected {remote_addr}");
    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigint = signal(SignalKind::interrupt()).expect("sigint handler");
        let mut sigterm = signal(SignalKind::terminate()).expect("sigterm handler");
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
