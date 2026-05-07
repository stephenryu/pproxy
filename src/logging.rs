use anyhow::{Context, Result};
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use std::{
    ffi::OsStr,
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use crate::config::Config;

pub(crate) fn setup_logging(conf: &Config) -> Result<()> {
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
        .checked_sub(Duration::from_secs(max_age_days * 24 * 60 * 60))
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
