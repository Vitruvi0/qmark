use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// Where libraries and headers usually live. Extended with the classic
/// search-path environment variables when they are set.
const DEFAULT_ROOTS: &[&str] = &[
    "/usr/include",
    "/usr/local/include",
    "/usr/lib",
    "/usr/lib64",
    "/usr/local/lib",
    "/lib",
    "/opt",
];

const PATH_ENV_VARS: &[&str] = &["C_INCLUDE_PATH", "CPATH", "LIBRARY_PATH", "LD_LIBRARY_PATH"];

pub fn run(name: &str, roots: &[PathBuf], exact: bool) -> Result<()> {
    if name.is_empty() {
        bail!("nothing to search for");
    }
    let mut roots = if roots.is_empty() {
        default_roots()
    } else {
        roots.to_vec()
    };
    // /lib and /usr/lib64 are usually symlinks to /usr/lib: resolve, then dedup.
    for root in &mut roots {
        if let Ok(real) = root.canonicalize() {
            *root = real;
        }
    }
    roots.sort();
    roots.dedup();
    let needle = name.to_lowercase();
    let mut hits = Vec::new();
    for root in &roots {
        walk(root, &needle, exact, &mut hits);
    }
    if hits.is_empty() {
        bail!("`{name}` not found under {}", join(&roots));
    }
    hits.sort();
    hits.dedup();
    for hit in hits {
        println!("{}", hit.display());
    }
    Ok(())
}

fn default_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = DEFAULT_ROOTS.iter().map(PathBuf::from).collect();
    for var in PATH_ENV_VARS {
        if let Some(value) = std::env::var_os(var) {
            roots.extend(std::env::split_paths(&value).filter(|p| !p.as_os_str().is_empty()));
        }
    }
    roots
}

// ponytail: plain recursive read_dir, no symlink following (avoids loops),
// no depth cap. Switch to `walkdir` if /opt ever gets too deep to be fun.
fn walk(dir: &Path, needle: &str, exact: bool, hits: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_lowercase();
        let matched = if exact {
            file_name == needle
        } else {
            file_name.contains(needle)
        };
        if matched {
            hits.push(path.clone());
        }
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            walk(&path, needle, exact, hits);
        }
    }
}

fn join(roots: &[PathBuf]) -> String {
    roots
        .iter()
        .map(|r| r.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}
