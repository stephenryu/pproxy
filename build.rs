use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let git_dir = find_git_dir();
    let build_hash = git_hash(&manifest_dir).unwrap_or_default();
    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();

    println!("cargo:rustc-env=PPROXY_BUILD_HASH={build_hash}");
    println!("cargo:rustc-env=PPROXY_BUILD_DATE={build_date}");

    if let Some(git_dir) = git_dir {
        watch_git_state(&git_dir);
    }
}

fn find_git_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").ok()?);
    let dot_git = manifest_dir.join(".git");

    if dot_git.is_dir() {
        return Some(dot_git);
    }

    let content = fs::read_to_string(&dot_git).ok()?;
    let gitdir = content.strip_prefix("gitdir:")?.trim();
    let resolved = Path::new(gitdir);
    if resolved.is_absolute() {
        Some(resolved.to_path_buf())
    } else {
        Some(manifest_dir.join(resolved))
    }
}

fn git_hash(manifest_dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .current_dir(manifest_dir)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let hash = String::from_utf8(output.stdout).ok()?;
    let hash = hash.trim().to_string();
    if hash.is_empty() {
        None
    } else {
        Some(hash)
    }
}

fn watch_git_state(git_dir: &Path) {
    let head = git_dir.join("HEAD");
    println!("cargo:rerun-if-changed={}", head.display());

    if let Ok(head_content) = fs::read_to_string(&head) {
        if let Some(ref_path) = head_content.strip_prefix("ref:").map(str::trim) {
            let ref_file = git_dir.join(ref_path);
            println!("cargo:rerun-if-changed={}", ref_file.display());
        }
    }
}
