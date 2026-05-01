//! `conductor` — long-running supervisor for parallel Claude Code
//! sessions across PlausiDen worktrees.
//!
//! v0.1 of conductor exposes the surface (CLI, types, supervisor, mock
//! driver) but does *not* yet shell to a real `claude` subprocess.
//! That driver lands in v0.2 — see `cross-repo/multi-instance.md` and
//! conductor-core's docs for the full design.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::disallowed_macros)]
#![allow(clippy::redundant_pub_crate)]

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    sync::Arc,
};

use anyhow::{Context as _, Result};
use avp_core::IntentFile;
use clap::{Args, Parser, Subcommand};
use conductor_core::{
    Host, HostsConfig, MockDriver, RecoveryPolicy, Session, Supervisor, SupervisorEvent,
    SupervisorEventKind,
};
use tracing::info;

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(name = "conductor", version, about = "PlausiDen conductor")]
struct Cli {
    /// `-v` debug, `-vv` trace.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Path to the hosts config TOML. Default:
    /// `$XDG_CONFIG_HOME/conductor/hosts.toml`, falling back to
    /// `~/.config/conductor/hosts.toml`. Missing files load as
    /// local-only (no error).
    #[arg(long, global = true)]
    host_config: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Print version + capability info.
    Info,
    /// List declared hosts + show how each given agent-id would route.
    Hosts(HostsArgs),
    /// Step the supervisor through a list of intents using the mock driver.
    /// Useful for exercising the FSM without a real claude subprocess.
    DryRun(DryRunArgs),
}

#[derive(Debug, Args)]
struct HostsArgs {
    /// Repeatable: agent ids to test against the routing rules.
    #[arg(long = "resolve")]
    resolve: Vec<String>,
}

#[derive(Debug, Args)]
struct DryRunArgs {
    /// Repeatable: path to a `.avp-intent.toml` to enroll.
    #[arg(long = "intent")]
    intents: Vec<PathBuf>,
    /// Override worktree path; defaults to `<intent-parent>`.
    #[arg(long)]
    worktree: Option<PathBuf>,
    /// Max supervisor steps before bailing (safety bound).
    #[arg(long, default_value_t = 64)]
    max_steps: u32,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(err) => {
            tracing::error!(?err, "tokio runtime init failed");
            return ExitCode::from(2);
        }
    };

    rt.block_on(async {
        match dispatch(cli).await {
            Ok(code) => code,
            Err(err) => {
                tracing::error!(?err, "fatal");
                eprintln!("conductor: {err}");
                for cause in err.chain().skip(1) {
                    eprintln!("  caused by: {cause}");
                }
                ExitCode::from(2)
            }
        }
    })
}

fn init_tracing(verbose: u8) {
    use tracing_subscriber::{EnvFilter, fmt};
    let level = match verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("conductor={level},conductor_core={level}")));
    let _ = fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init();
}

async fn dispatch(cli: Cli) -> Result<ExitCode> {
    let cfg_path = cli
        .host_config
        .clone()
        .unwrap_or_else(default_host_config_path);
    let hosts = HostsConfig::load(&cfg_path)
        .with_context(|| format!("load host config {}", cfg_path.display()))?;
    info!(hosts = ?hosts.host_names(), default = %hosts.default, "host config loaded");

    match cli.cmd {
        Cmd::Info => {
            println!(
                "conductor {} (conductor-core {})",
                env!("CARGO_PKG_VERSION"),
                conductor_core::VERSION,
            );
            println!("driver: mock (real claude-cli driver lands v0.2)");
            println!(
                "hosts: {}  (default: {})",
                hosts.host_names().join(", "),
                hosts.default,
            );
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Hosts(args) => run_hosts(&hosts, &args),
        Cmd::DryRun(args) => run_dry_run(args, &hosts).await,
    }
}

fn default_host_config_path() -> PathBuf {
    let xdg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let base = xdg.unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map_or_else(|| PathBuf::from("."), |h| h.join(".config"))
    });
    base.join("conductor").join("hosts.toml")
}

// AVP-PASS-2026-05-01: dispatch arms share the Result<ExitCode> shape;
// keeping run_hosts wrapped lets future error paths slot in without
// changing the signature.
#[allow(clippy::unnecessary_wraps)]
fn run_hosts(hosts: &HostsConfig, args: &HostsArgs) -> Result<ExitCode> {
    println!("hosts ({} declared):", hosts.host_names().len());
    for name in hosts.host_names() {
        let label = match hosts.hosts.get(name) {
            Some(Host::Local) => "local".to_owned(),
            Some(Host::Ssh(t)) => format!("ssh:{t}"),
            _ => "unknown".to_owned(),
        };
        let star = if name == hosts.default { " *" } else { "" };
        println!("  {name:>16}{star}  {label}");
    }
    if !args.resolve.is_empty() {
        println!("\nrouting:");
        for id in &args.resolve {
            let host = hosts.resolve(id);
            println!("  {id:>32}  →  {}", host.label());
        }
    }
    Ok(ExitCode::SUCCESS)
}

async fn run_dry_run(args: DryRunArgs, hosts: &HostsConfig) -> Result<ExitCode> {
    if args.intents.is_empty() {
        return Err(anyhow::anyhow!("--intent <path> required (repeatable)"));
    }

    let driver = Arc::new(MockDriver::new());
    let sup = Supervisor::with_policy(driver.clone(), RecoveryPolicy::default());

    for path in &args.intents {
        let intent = IntentFile::load(path).with_context(|| format!("load {}", path.display()))?;
        let worktree = args
            .worktree
            .clone()
            .unwrap_or_else(|| parent_or_dot(path).to_path_buf());
        // Resolve which Host this intent routes to (Local / Ssh).
        let host = hosts.resolve(intent.agent_id.as_str()).clone();
        info!(agent = %intent.agent_id, host = %host.label(), "routed");
        let session = Session::new_on(intent, worktree, host);
        sup.enroll(session).await;
    }

    // Synthesize a single Done event per session so the supervisor walks
    // the FSM end-to-end. Real driver replaces this loop with poll().
    let snap = sup.snapshot().await;
    for (id, _, _) in &snap {
        driver.script(id, conductor_core::driver::DriverEvent::Done);
    }

    let mut steps = 0u32;
    while !sup.is_done().await {
        if steps >= args.max_steps {
            return Err(anyhow::anyhow!(
                "supervisor not done after {} steps",
                args.max_steps
            ));
        }
        sup.step().await?;
        steps = steps.saturating_add(1);
    }
    info!(steps, "supervisor reached terminal");

    let events = sup.drain_events().await;
    print_events(&events);
    Ok(ExitCode::SUCCESS)
}

fn parent_or_dot(p: &Path) -> &Path {
    p.parent().unwrap_or_else(|| Path::new("."))
}

fn print_events(events: &[SupervisorEvent]) {
    for e in events {
        let session = e
            .session
            .as_ref()
            .map_or_else(|| "-".to_owned(), ToString::to_string);
        let kind = match &e.kind {
            SupervisorEventKind::Queued => "queued".to_owned(),
            SupervisorEventKind::Started => "started".to_owned(),
            SupervisorEventKind::Log { line } => format!("log: {line}"),
            SupervisorEventKind::Paused { reason } => format!("paused: {}", reason.label()),
            SupervisorEventKind::ResumeScheduled { delay_seconds } => {
                format!("resume-after-{delay_seconds}s")
            }
            SupervisorEventKind::Escalated { reason } => format!("escalated: {reason}"),
            SupervisorEventKind::Terminal { outcome } => format!("terminal: {outcome:?}"),
            // Non-exhaustive: future doctrine variants render as "?".
            _ => "?".to_owned(),
        };
        println!("{session:>30}  {kind}");
    }
}
