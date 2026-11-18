#![forbid(unsafe_code)]

//! `xtask`: repository tasks run with `cargo xtask <task>` (spec 046).
//!
//! - `gen-icons` writes the application and `.gaf` icons of 03 §5.3 from the
//!   brand SVGs (DIS-13).
//! - `gen-icons --check` regenerates them in memory and fails when a tracked
//!   file differs.

mod icons;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["gen-icons"] => gen_icons(&root(), false),
        ["gen-icons", "--check"] => gen_icons(&root(), true),
        _ => Err("usage: cargo xtask gen-icons [--check]".to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("xtask: {error}");
            ExitCode::FAILURE
        }
    }
}

fn gen_icons(root: &Path, check: bool) -> Result<(), String> {
    let outputs = icons::generate(root)?;
    let mut stale = Vec::new();
    for out in &outputs {
        let path = root.join(&out.path);
        if std::fs::read(&path).is_ok_and(|bytes| bytes == out.bytes) {
            continue;
        }
        if check {
            stale.push(out.path.as_str());
            continue;
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
        std::fs::write(&path, &out.bytes).map_err(|e| format!("{}: {e}", out.path))?;
        println!("wrote {}", out.path);
    }
    if stale.is_empty() {
        println!(
            "{} icon files up to date in {}",
            outputs.len(),
            icons::OUT_DIR
        );
        Ok(())
    } else {
        Err(format!(
            "stale icons (run `cargo xtask gen-icons`):\n  {}",
            stale.join("\n  ")
        ))
    }
}
