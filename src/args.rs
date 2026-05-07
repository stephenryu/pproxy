use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "pproxy", disable_help_subcommand = true)]
pub(crate) struct Args {
    /// Config file name (default: pproxy.yaml)
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,

    /// Show the application version
    #[arg(long)]
    pub(crate) version: bool,
}

pub(crate) fn parse() -> Args {
    Args::parse_from(preprocess_args())
}

pub(crate) fn version_string() -> String {
    let v = env!("CARGO_PKG_VERSION");
    let build_hash = option_env!("PPROXY_BUILD_HASH").unwrap_or("").trim();
    let build_date = option_env!("PPROXY_BUILD_DATE").unwrap_or("").trim();

    match (!build_hash.is_empty(), !build_date.is_empty()) {
        (true, true) => format!("{v} ({build_hash} {build_date})"),
        (true, false) => format!("{v} ({build_hash})"),
        (false, true) => format!("{v} ({build_date})"),
        (false, false) => v.to_string(),
    }
}

fn preprocess_args() -> Vec<std::ffi::OsString> {
    // Support Go-style flags like `-config foo.yaml` and `-version`.
    // Also support `-V`, `-v`, `--v`, `-ver`, `--ver`.
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    let mut iter = std::env::args_os();
    if let Some(bin) = iter.next() {
        out.push(bin);
    }
    for arg in iter {
        match arg.to_string_lossy().as_ref() {
            "-config" => out.push("--config".into()),
            "-version" => out.push("--version".into()),
            "-V" | "-v" | "--v" | "-ver" | "--ver" => out.push("--version".into()),
            _ => out.push(arg),
        }
    }
    out
}
