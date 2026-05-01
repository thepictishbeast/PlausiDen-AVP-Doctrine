//! `avp` — AVP-2 supersociety toolchain CLI.
//!
//! Subcommands:
//! - `avp check {rust|ts|py}` — run AVP gates against a sibling repo.
//! - `avp ratchet {validate|list|preflight|add}` — manage `avp-ratchet.toml`.
//! - `avp drift` — detect drift across the PlausiDen portfolio.
//! - `avp install` — drop canonical configs into a sibling.
//! - `avp gate {list|explain}` — introspect gates.
//! - `avp completions <shell>` — emit shell completion scripts.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
// AVP-PASS-2026-04-30: the workspace clippy.toml disallows println!/eprintln!
// in *library* code (per AVP-2 §Logging). This crate is the CLI binary —
// printing to stdout/stderr is its job. The override is local to the binary.
#![allow(clippy::disallowed_macros)]
#![allow(clippy::redundant_pub_crate)]

mod cli;
mod install;

use std::process::ExitCode;

use clap::Parser as _;
use tracing::{debug, error};

use crate::cli::Cli;

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);
    debug!(?cli, "parsed CLI args");

    match cli.run() {
        Ok(code) => code,
        Err(err) => {
            error!(?err, "fatal");
            // Walk the error chain for the human reporter.
            eprintln!("avp: {err}");
            for cause in err.chain().skip(1) {
                eprintln!("  caused by: {cause}");
            }
            ExitCode::from(2)
        }
    }
}

/// Initialize `tracing-subscriber` with `RUST_LOG` honored, default `INFO`,
/// `-v` => `DEBUG`, `-vv` => `TRACE`. ANSI is auto-detected via the env.
fn init_tracing(verbose: u8) {
    use tracing_subscriber::{EnvFilter, fmt};

    let default_level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!("avp={default_level},avp_core={default_level}"))
    });

    let _ = fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
