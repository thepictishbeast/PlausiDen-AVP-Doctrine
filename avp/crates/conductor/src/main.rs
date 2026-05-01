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

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{Context as _, Result};
use avp_core::IntentFile;
use clap::{Args, Parser, Subcommand};
use conductor::{driver_local::LocalClaudeDriver, driver_ssh::SshClaudeDriver};
use conductor_core::{
    ClaudeDriver, Host, HostsConfig, MockDriver, RecoveryPolicy, Session, SshTarget, Supervisor,
    SupervisorEvent, SupervisorEventKind,
};
use tracing::{info, warn};

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
    /// Drive real claude-cli sessions across hosts. One supervisor per
    /// host group; each group runs concurrently. Use --host-config to
    /// route specific intents to specific hosts.
    Run(RunArgs),
}

#[derive(Debug, Args)]
struct RunArgs {
    /// Repeatable: path to a `.avp-intent.toml` to enroll.
    #[arg(long = "intent")]
    intents: Vec<PathBuf>,
    /// Override worktree path; defaults to `<intent-parent>` for local
    /// sessions and the host's `remote_workdir` for SSH sessions.
    #[arg(long)]
    worktree: Option<PathBuf>,
    /// Path to the `claude` binary on the *local* machine. Default:
    /// `claude` (resolved via PATH).
    #[arg(long, default_value = "claude")]
    claude_bin: PathBuf,
    /// Path to the `claude` binary on every *remote* SSH host. Default:
    /// `claude`. Override per-host once we have a per-host config knob.
    #[arg(long, default_value = "claude")]
    remote_claude_bin: String,
    /// Poll interval (ms) — how often each supervisor steps and drains
    /// events.
    #[arg(long, default_value_t = 250)]
    poll_ms: u64,
    /// Hard cap on supervisor steps before bailing out (per group).
    #[arg(long, default_value_t = 10_000)]
    max_steps: u32,
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
        Cmd::Run(args) => run_real(args, &hosts).await,
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

// ─── conductor run ───────────────────────────────────────────────────────

/// Real-driver execution. Group intents by host name, instantiate the
/// right driver per group (Local / Ssh), spawn one supervisor per
/// group, and drive them concurrently until every session terminates.
async fn run_real(args: RunArgs, hosts: &HostsConfig) -> Result<ExitCode> {
    if args.intents.is_empty() {
        return Err(anyhow::anyhow!("--intent <path> required (repeatable)"));
    }

    // Group intents by resolved host name.
    let mut groups: HashMap<String, Vec<Session>> = HashMap::new();
    for path in &args.intents {
        let intent = IntentFile::load(path).with_context(|| format!("load {}", path.display()))?;
        let host = hosts.resolve(intent.agent_id.as_str()).clone();
        let host_name = hosts.resolve_name(intent.agent_id.as_str()).to_owned();
        let worktree = args.worktree.clone().unwrap_or_else(|| match &host {
            Host::Ssh(t) => t.remote_workdir.clone(),
            // Local + any future variant: use the intent's parent dir.
            _ => parent_or_dot(path).to_path_buf(),
        });
        info!(agent = %intent.agent_id, host = %host_name, "routed");
        let session = Session::new_on(intent, worktree, host);
        groups.entry(host_name).or_default().push(session);
    }

    // Spawn one supervisor per group, all concurrent. Each task owns its
    // supervisor + driver so lifetimes are clean.
    let mut joinset: tokio::task::JoinSet<Result<u32>> = tokio::task::JoinSet::new();
    let poll = Duration::from_millis(args.poll_ms);
    let max_steps = args.max_steps;

    for (host_name, sessions) in groups {
        let host =
            hosts.hosts.get(&host_name).cloned().with_context(|| {
                format!("config bug: resolved host_name {host_name:?} not in map")
            })?;
        match host {
            Host::Local => {
                let driver = Arc::new(LocalClaudeDriver::with_bin(args.claude_bin.clone()));
                let sup = Arc::new(Supervisor::with_policy(driver, RecoveryPolicy::default()));
                for s in sessions {
                    sup.enroll(s).await;
                }
                let label = host_name.clone();
                joinset.spawn(async move { drive_group(label, sup, poll, max_steps).await });
            }
            Host::Ssh(target) => {
                let driver = Arc::new(make_ssh_driver(target, &args.remote_claude_bin));
                let sup = Arc::new(Supervisor::with_policy(driver, RecoveryPolicy::default()));
                for s in sessions {
                    sup.enroll(s).await;
                }
                let label = host_name.clone();
                joinset.spawn(async move { drive_group(label, sup, poll, max_steps).await });
            }
            // Future Host variants (Docker / k8s / cloud-run) get
            // explicit handling here; until then we refuse rather
            // than silently routing to local.
            _ => {
                return Err(anyhow::anyhow!(
                    "host kind not yet supported by `conductor run`: {host_name}"
                ));
            }
        }
    }

    let mut group_count = 0u32;
    let mut total_sessions = 0u32;
    while let Some(joined) = joinset.join_next().await {
        let count = match joined {
            Ok(Ok(n)) => n,
            Ok(Err(err)) => {
                warn!(?err, "supervisor task errored");
                0
            }
            Err(err) => {
                warn!(?err, "supervisor task panicked");
                0
            }
        };
        group_count = group_count.saturating_add(1);
        total_sessions = total_sessions.saturating_add(count);
    }

    println!("conductor: {group_count} group(s), {total_sessions} session(s) terminal");
    Ok(ExitCode::SUCCESS)
}

fn make_ssh_driver(target: SshTarget, remote_claude_bin: &str) -> SshClaudeDriver {
    SshClaudeDriver::new(target).with_remote_claude(remote_claude_bin.to_owned())
}

/// Drive one group's supervisor until every session is terminal (or we
/// hit the step cap). Streams events to stdout per session.
async fn drive_group<D>(
    label: String,
    sup: Arc<Supervisor<D>>,
    poll: Duration,
    max_steps: u32,
) -> Result<u32>
where
    D: ClaudeDriver + 'static,
{
    let mut steps = 0u32;
    while !sup.is_done().await {
        if steps >= max_steps {
            return Err(anyhow::anyhow!(
                "[{label}] supervisor not done after {max_steps} steps"
            ));
        }
        if let Err(err) = sup.step().await {
            warn!(group = %label, ?err, "step failed");
        }
        for event in sup.drain_events().await {
            print_real_event(&label, &event);
        }
        steps = steps.saturating_add(1);
        tokio::time::sleep(poll).await;
    }
    // Drain any final events emitted on the last step.
    for event in sup.drain_events().await {
        print_real_event(&label, &event);
    }
    let snap = sup.snapshot().await;
    Ok(u32::try_from(snap.len()).unwrap_or(u32::MAX))
}

fn print_real_event(label: &str, e: &SupervisorEvent) {
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
        _ => "?".to_owned(),
    };
    println!("[{label:>14}] {session:>30}  {kind}");
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
