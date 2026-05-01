//! `b3sum-shim` — tiny CLI that prints `<hash>  <basename>` for a file.
//!
//! Used by the release workflow to produce blake3 manifests without
//! requiring `b3sum` or `python3-blake3` on the runner. Reuses the
//! `blake3` Rust crate that conductor already depends on, so no new
//! deps are pulled in just for CI.
//!
//! Usage: `b3sum-shim <file>`. Output mirrors `b3sum`'s default
//! format: 64 hex chars, two spaces, basename of the input.

#![forbid(unsafe_code)]
#![allow(clippy::disallowed_macros)] // it's a CLI; printing is the point

use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: b3sum-shim <file>");
        return ExitCode::from(2);
    };
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(err) => {
            eprintln!("b3sum-shim: read {path}: {err}");
            return ExitCode::from(1);
        }
    };
    let hash = blake3::hash(&data);
    let basename = Path::new(&path)
        .file_name()
        .map_or_else(|| path.clone(), |n| n.to_string_lossy().into_owned());
    println!("{hash}  {basename}");
    ExitCode::SUCCESS
}
