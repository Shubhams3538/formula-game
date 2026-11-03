//! Derives `RACECRAFT_BUILD_HASH`: a hash of everything that decides whether two
//! builds simulate and talk identically. That is every source file in `sim` and
//! `net`, plus the locked versions of the crates those sources compute or
//! encode with. Comments count too; a false "incompatible" is safer than a
//! silent physics mismatch.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const LOCKED_CRATES: &[&str] = &[
    "glam",
    "serde",
    "serde_derive",
    "serde_json",
    "bincode",
    "miniz_oxide",
    "renet",
    "renetcode",
    "renet_netcode",
];

fn sources(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, files);
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
}

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.parent().unwrap().to_path_buf();
    let roots = [workspace.join("sim/src"), manifest.join("src")];
    let lock = workspace.join("Cargo.lock");
    for path in roots.iter().chain([&lock]) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!("cargo:rerun-if-changed=build.rs");

    let mut files = Vec::new();
    for root in &roots {
        sources(root, &mut files);
    }
    let mut named: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(&workspace)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            (name, path)
        })
        .collect();
    named.sort();

    let mut hash = Sha256::new();
    for (name, path) in &named {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .replace("\r\n", "\n");
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((text.len() as u64).to_le_bytes());
        hash.update(text.as_bytes());
    }
    // Only the versions of crates that affect results, so an unrelated
    // renderer update does not split players into incompatible groups.
    if let Ok(lockfile) = std::fs::read_to_string(&lock) {
        let mut versions = Vec::new();
        for package in lockfile.split("[[package]]") {
            let field = |key: &str| {
                package.lines().find_map(|line| {
                    line.strip_prefix(key)
                        .map(|rest| rest.trim_start_matches(" = ").trim_matches('"').to_owned())
                })
            };
            if let (Some(name), Some(version)) = (field("name"), field("version")) {
                if LOCKED_CRATES.contains(&name.as_str()) {
                    versions.push(format!("{name} {version}"));
                }
            }
        }
        versions.sort();
        for line in versions {
            hash.update(line.as_bytes());
            hash.update(b"\n");
        }
    }
    let hex: String = hash
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    println!("cargo:rustc-env=RACECRAFT_BUILD_HASH={hex}");
}
