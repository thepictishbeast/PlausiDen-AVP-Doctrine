//! `avp` CLI definition + dispatch.
//!
//! Most subcommands are scaffold stubs at v0.1.0-dev. Implementations land
//! incrementally per task #14, #15, #16, #19 in the project tracker.
//!
//! `avp ratchet validate` and `avp gate list` are *real* in v0.1.0-dev so
//! the binary already exposes a useful surface area on first build.

// AVP-PASS-2026-04-30: see the rationale in main.rs — the binary CLI prints
// to stdout/stderr by design; the doctrine ban on println!/eprintln! is
// scoped to library code, not entry points.
#![allow(clippy::disallowed_macros)]
// AVP-PASS-2026-04-30: rust's `unreachable_pub` and clippy's
// `redundant_pub_crate` are mutually exclusive in a binary crate.
// Convention is `pub(crate)` + allow the clippy lint locally.
#![allow(clippy::redundant_pub_crate)]
// AVP-PASS-2026-04-30: dispatch arms share `Result<ExitCode>` for uniformity;
// stubs that can't fail still wrap to satisfy the trait of the enclosing match.
#![allow(clippy::unnecessary_wraps)]

use std::{io::Write as _, path::PathBuf, process::ExitCode};

use anyhow::{Context as _, Result, anyhow};
use avp_core::{
    Context as GateContext, CrateName, Finding, GateId, GithubActionsReporter, HumanReporter,
    JsonReporter, RatchetFile, RepoRoot, Reporter, Severity,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use comfy_table::{Cell, ContentArrangement, Table, presets};
use tracing::{info, instrument, warn};

const ABOUT_LONG: &str = "\
avp — the AVP-2 supersociety toolchain.

One binary, many subcommands. Enforces the gates defined in the
PlausiDen-AVP-Doctrine across every sibling repo.

Run `avp gate list` to see the gates this build enforces, and
`avp explain <gate>` for the doctrine rationale behind each.";

/// Top-level CLI.
#[derive(Debug, Parser)]
#[command(name = "avp", version, about, long_about = ABOUT_LONG, propagate_version = true)]
pub(crate) struct Cli {
    /// Increase verbosity. `-v` = debug, `-vv` = trace.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Disable ANSI color (also honors `NO_COLOR`).
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Output format for subcommands that produce findings.
    #[arg(long, global = true, default_value_t = OutputFormat::Auto, value_enum)]
    pub format: OutputFormat,

    #[command(subcommand)]
    pub cmd: Cmd,
}

/// Output format selector.
#[derive(Debug, Copy, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum OutputFormat {
    /// `human` if a TTY, `github-actions` if CI is detected, otherwise `human`.
    Auto,
    /// Colored, TTY-friendly summary.
    Human,
    /// `::error::file=…,line=…::msg` annotations.
    GithubActions,
    /// JSON-lines (one finding per line).
    Json,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum Cmd {
    /// Run AVP gates against a sibling repo.
    Check(CheckArgs),
    /// Manage the per-repo `avp-ratchet.toml`.
    Ratchet(RatchetArgs),
    /// Detect drift across the PlausiDen portfolio.
    Drift(DriftArgs),
    /// Drop canonical workflow + configs into a sibling repo.
    Install(InstallArgs),
    /// Introspect gates defined in this build.
    Gate(GateArgs),
    /// Print the doctrine rationale for a single gate.
    Explain(ExplainArgs),
    /// Multi-instance coordination over `.avp-intent.toml`.
    Intent(IntentArgs),
    /// Emit shell-completion scripts.
    Completions(CompletionsArgs),
}

// ─── check ───────────────────────────────────────────────────────────────

/// `avp check`.
#[derive(Debug, Args)]
pub(crate) struct CheckArgs {
    /// Language to gate.
    #[arg(value_enum)]
    pub language: CheckLanguage,
    /// Repo root (default: cwd).
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Path to `avp-ratchet.toml` (default: <root>/avp-ratchet.toml).
    #[arg(long)]
    pub ratchet: Option<PathBuf>,
    /// Pass `--workspace` to cargo subcommands (Rust only).
    #[arg(long, default_value_t = true)]
    pub workspace: bool,
    /// Clippy strictness (Rust only).
    #[arg(long, value_enum, default_value_t = ClippyStrictness::Doctrine)]
    pub strictness: ClippyStrictness,
    /// Aggregate test-density floor (Rust only).
    #[arg(long, default_value_t = 4.0)]
    pub test_density_min: f64,
}

/// Languages `avp check` accepts.
#[derive(Debug, Copy, Clone, ValueEnum)]
#[value(rename_all = "lowercase")]
pub(crate) enum CheckLanguage {
    /// Rust crate or workspace.
    Rust,
    /// TypeScript / JavaScript via `package.json`.
    Ts,
    /// Python via `pyproject.toml`.
    Py,
}

/// Clippy strictness selector.
#[derive(Debug, Copy, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub(crate) enum ClippyStrictness {
    /// `-D warnings -D clippy::pedantic -D clippy::nursery` (full doctrine).
    Doctrine,
    /// `-D warnings` only (legacy/pre-cleanup siblings).
    Minimal,
}

// ─── ratchet ─────────────────────────────────────────────────────────────

/// `avp ratchet`.
#[derive(Debug, Args)]
pub(crate) struct RatchetArgs {
    #[command(subcommand)]
    pub cmd: RatchetCmd,
}

/// Ratchet subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum RatchetCmd {
    /// Parse + validate a ratchet file. Exits non-zero on schema errors.
    Validate {
        /// Path to the ratchet file.
        #[arg(default_value = "avp-ratchet.toml")]
        path: PathBuf,
    },
    /// Print one row per active override, including days-until-expiry.
    List {
        /// Path to the ratchet file.
        #[arg(default_value = "avp-ratchet.toml")]
        path: PathBuf,
        /// Include expired entries in the listing.
        #[arg(long)]
        include_expired: bool,
    },
    /// Print warnings for soon-to-expire overrides; exit 1 if any expired.
    Preflight {
        /// Path to the ratchet file.
        #[arg(default_value = "avp-ratchet.toml")]
        path: PathBuf,
        /// Window size in days for "expiring soon" notice.
        #[arg(long, default_value_t = 14)]
        warn_window_days: u16,
    },
    /// Add a new override entry. Stub in v0.1.0-dev.
    Add(RatchetAddArgs),
}

/// `avp ratchet add` — stub.
#[derive(Debug, Args)]
pub(crate) struct RatchetAddArgs {
    /// Gate to ratchet (kebab-case).
    #[arg(long)]
    pub gate: String,
    /// Optional crate scope.
    #[arg(long)]
    pub crate_scope: Option<String>,
    /// Optional file regex.
    #[arg(long)]
    pub file: Option<String>,
    /// Justification.
    #[arg(long)]
    pub reason: String,
    /// Accountable signer email.
    #[arg(long)]
    pub signed_by: String,
    /// Expiry date (YYYY-MM-DD).
    #[arg(long)]
    pub expires_after: String,
}

// ─── drift / install / gate / explain / intent / completions ─────────────

/// `avp drift` args.
#[derive(Debug, Args)]
pub(crate) struct DriftArgs {
    /// Portfolio root.
    #[arg(long, default_value = "/home/user/Development/PlausiDen")]
    pub root: PathBuf,
    /// File a drift issue per offender via `gh issue create`.
    #[arg(long)]
    pub open_issues: bool,
}

/// `avp install` args.
#[derive(Debug, Args)]
pub(crate) struct InstallArgs {
    /// Sibling repo root.
    #[arg(default_value = ".")]
    pub repo: PathBuf,
    /// Don't write; print the diff.
    #[arg(long)]
    pub dry_run: bool,
    /// Overwrite locally-modified files.
    #[arg(long)]
    pub force: bool,
}

/// `avp gate` args.
#[derive(Debug, Args)]
pub(crate) struct GateArgs {
    #[command(subcommand)]
    pub cmd: GateCmd,
}

/// `avp gate` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum GateCmd {
    /// Print every gate, with descriptions.
    List,
}

/// `avp explain` args.
#[derive(Debug, Args)]
pub(crate) struct ExplainArgs {
    /// Gate id (kebab-case).
    pub gate: String,
}

/// `avp intent` args.
#[derive(Debug, Args)]
pub(crate) struct IntentArgs {
    #[command(subcommand)]
    pub cmd: IntentCmd,
}

/// `avp intent` subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum IntentCmd {
    /// Claim a worktree by writing `.avp-intent.toml`. Stub.
    Claim,
    /// Compute file-touch overlap across all open intents. Stub.
    Overlap,
    /// Print a topologically sorted merge order. Stub.
    MergeOrder,
    /// Verify branch's actual diff against its declared intent. Stub.
    Verify,
}

/// `avp completions` args.
#[derive(Debug, Args)]
pub(crate) struct CompletionsArgs {
    /// Target shell.
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

// ─── dispatch ────────────────────────────────────────────────────────────

impl Cli {
    /// Dispatch the parsed CLI. Returns the desired process exit code.
    #[instrument(level = "debug", skip(self))]
    pub(crate) fn run(self) -> Result<ExitCode> {
        match self.cmd {
            Cmd::Check(args) => run_check(&args),
            Cmd::Ratchet(args) => run_ratchet(args),
            Cmd::Drift(_) => stub("avp drift"),
            Cmd::Install(args) => run_install(&args),
            Cmd::Gate(args) => run_gate(&args),
            Cmd::Explain(args) => run_explain(&args),
            Cmd::Intent(_) => stub("avp intent"),
            Cmd::Completions(args) => run_completions(&args),
        }
    }
}

impl CheckArgs {
    // AVP-PASS-2026-04-30: these methods ignore `self` deliberately as the
    // global flags from `Cli` (format, no-color) aren't plumbed into the
    // per-subcommand args yet; the indirection is here so v0.2 can wire it
    // without changing call sites.
    #[allow(clippy::unused_self)]
    const fn format(&self) -> OutputFormat {
        OutputFormat::Auto
    }

    #[allow(clippy::unused_self)]
    const fn no_color(&self) -> bool {
        false
    }
}

fn stub(name: &str) -> Result<ExitCode> {
    warn!("{name} is not implemented in this build");
    eprintln!(
        "avp: {name} is not implemented yet (v0.1.0-dev). See task tracker in PlausiDen-AVP-Doctrine."
    );
    Ok(ExitCode::from(64))
}

#[instrument(level = "debug", skip_all)]
fn run_install(args: &InstallArgs) -> Result<ExitCode> {
    // Pin to this build's version; `avp install` writes a workflow that
    // matches the binary the user is currently invoking.
    let version_tag = format!("v{}", env!("CARGO_PKG_VERSION"));
    crate::install::run(&args.repo, &version_tag, args.dry_run, args.force)
}

#[instrument(level = "debug", skip_all)]
fn run_check(args: &CheckArgs) -> Result<ExitCode> {
    match args.language {
        CheckLanguage::Rust => run_check_rust(args),
        CheckLanguage::Ts => stub("avp check ts"),
        CheckLanguage::Py => stub("avp check py"),
    }
}

/// Resolve the repo root and the ratchet path. The ratchet is loaded
/// strictly: schema errors fail the run.
#[instrument(level = "debug", skip_all)]
fn run_check_rust(args: &CheckArgs) -> Result<ExitCode> {
    let cwd = std::env::current_dir().context("read current directory")?;
    let root_path = args.root.clone().unwrap_or(cwd);
    let repo = RepoRoot::discover(&root_path)
        .with_context(|| format!("discover repo at {}", root_path.display()))?;
    info!(root = %repo.path.display(), "rust gate run");

    // Honor the configured ratchet (schema validation only at this stage; the
    // gate-level finding filter lands in the next slice). Errors are loud.
    let ratchet_path = args
        .ratchet
        .clone()
        .unwrap_or_else(|| repo.path.join(avp_core::RATCHET_FILE));
    let mut ratchet = RatchetFile::load(&ratchet_path)
        .with_context(|| format!("load ratchet from {}", ratchet_path.display()))?;
    ratchet.validate().context("validate ratchet")?;
    let ratchet_active = ratchet.active(today()).len();
    info!(active = ratchet_active, "ratchet loaded");

    let in_ci = std::env::var_os("CI").is_some();
    let ctx = GateContext::new(&repo, in_ci);

    let gates = avp_rust::gates::all_gates();
    let mut findings: Vec<Finding> = Vec::new();
    for gate in &gates {
        let gate_id = gate.id();
        let mut produced = gate.run(&ctx);
        info!(gate = %gate_id, count = produced.len(), "gate finished");
        findings.append(&mut produced);
    }

    // Apply ratchet filter: any finding covered by an active override is
    // downgraded to Notice severity. Expired overrides have already failed
    // the preflight in load+validate above, so nothing here silently passes.
    let now = today();
    let repo_root = repo.path.clone();
    for f in &mut findings {
        if !f.severity.is_failing() {
            continue;
        }
        let crate_scope = avp_rust::source::crate_name_for_path(&repo_root.join(&f.location.file))
            .and_then(|n| CrateName::new(n).ok());
        if ratchet.covers(f.gate, crate_scope.as_ref(), Some(&f.location.file), now) {
            f.severity = Severity::Notice;
            f.message = format!("[ratcheted] {}", f.message);
        }
    }

    // Choose a reporter. AUTO picks GitHub-Actions in CI, human elsewhere.
    let format = match args.format() {
        OutputFormat::Auto if in_ci => OutputFormat::GithubActions,
        OutputFormat::Auto => OutputFormat::Human,
        other => other,
    };

    let any_error = findings.iter().any(|f| f.severity == Severity::Error);

    let mut reporter: Box<dyn Reporter> = match format {
        OutputFormat::GithubActions => Box::new(GithubActionsReporter::new(std::io::stdout())),
        OutputFormat::Json => Box::new(JsonReporter::new(std::io::stdout())),
        // Auto resolved above; treat as Human here.
        OutputFormat::Human | OutputFormat::Auto => {
            let colored = !args.no_color() && std::env::var_os("NO_COLOR").is_none();
            Box::new(HumanReporter::new(std::io::stdout(), colored))
        }
    };
    for f in &findings {
        reporter.emit(f).context("reporter emit")?;
    }
    reporter.finalize().context("reporter finalize")?;

    Ok(if any_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

#[instrument(level = "debug", skip_all)]
fn run_ratchet(args: RatchetArgs) -> Result<ExitCode> {
    match args.cmd {
        RatchetCmd::Validate { path } => {
            let mut f = RatchetFile::load(&path)
                .with_context(|| format!("loading ratchet from {}", path.display()))?;
            f.validate().context("validating ratchet")?;
            let n = f.entries.len();
            info!(count = n, path = %path.display(), "ratchet ok");
            println!("{}: {n} override(s) — valid", path.display());
            Ok(ExitCode::SUCCESS)
        }
        RatchetCmd::List {
            path,
            include_expired,
        } => {
            let f = RatchetFile::load(&path)
                .with_context(|| format!("loading ratchet from {}", path.display()))?;
            print_ratchet_table(&f, include_expired)?;
            Ok(ExitCode::SUCCESS)
        }
        RatchetCmd::Preflight {
            path,
            warn_window_days,
        } => {
            let f = RatchetFile::load(&path)
                .with_context(|| format!("loading ratchet from {}", path.display()))?;
            let now = today();
            let expired = f.expired(now);
            let soon = f.expiring_soon(now, warn_window_days);
            for e in &expired {
                eprintln!(
                    "::error::expired ratchet: gate={} signed_by={} reason={:?}",
                    e.gate, e.signed_by, e.reason
                );
            }
            for e in &soon {
                eprintln!(
                    "::warning::ratchet expires in {}d: gate={} signed_by={}",
                    e.days_until_expiry(now),
                    e.gate,
                    e.signed_by
                );
            }
            let active = f.active(now);
            for e in &active {
                if !soon.iter().any(|s| std::ptr::addr_eq(*s, *e)) {
                    println!(
                        "::notice::ratchet active ({}d left): gate={} signed_by={}",
                        e.days_until_expiry(now),
                        e.gate,
                        e.signed_by
                    );
                }
            }
            Ok(if expired.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        RatchetCmd::Add(_) => stub("avp ratchet add"),
    }
}

#[instrument(level = "debug")]
fn run_gate(args: &GateArgs) -> Result<ExitCode> {
    match &args.cmd {
        GateCmd::List => {
            let mut t = Table::new();
            t.load_preset(presets::UTF8_BORDERS_ONLY);
            t.set_content_arrangement(ContentArrangement::Dynamic);
            t.set_header(vec![
                Cell::new("gate"),
                Cell::new("ratchetable"),
                Cell::new("description"),
            ]);
            for g in GateId::ALL {
                t.add_row(vec![
                    Cell::new(g.as_kebab()),
                    Cell::new(if g.ratchetable() { "yes" } else { "NO" }),
                    Cell::new(g.description()),
                ]);
            }
            let mut out = anstream::stdout().lock();
            writeln!(out, "{t}").map_err(|e| anyhow!(e))?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

#[instrument(level = "debug")]
fn run_explain(args: &ExplainArgs) -> Result<ExitCode> {
    let g: GateId = args.gate.parse().map_err(|e| anyhow!("{e}"))?;
    println!("{}\n  {}", g.as_kebab(), g.description());
    println!(
        "\nRatchetable: {}\nDoctrine: PlausiDen-AVP-Doctrine/AVP2_PROTOCOL.md",
        g.ratchetable()
    );
    Ok(ExitCode::SUCCESS)
}

#[instrument(level = "debug", skip_all)]
fn run_completions(args: &CompletionsArgs) -> Result<ExitCode> {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    let bin = cmd.get_name().to_owned();
    clap_complete::generate(args.shell, &mut cmd, bin, &mut std::io::stdout());
    Ok(ExitCode::SUCCESS)
}

fn print_ratchet_table(f: &RatchetFile, include_expired: bool) -> Result<()> {
    let now = today();
    let mut t = Table::new();
    t.load_preset(presets::UTF8_BORDERS_ONLY);
    t.set_content_arrangement(ContentArrangement::Dynamic);
    t.set_header(vec![
        Cell::new("gate"),
        Cell::new("crate"),
        Cell::new("file"),
        Cell::new("days_left"),
        Cell::new("signed_by"),
        Cell::new("reason"),
    ]);
    for e in &f.entries {
        let expired = e.is_expired(now);
        if expired && !include_expired {
            continue;
        }
        let days = e.days_until_expiry(now);
        t.add_row(vec![
            Cell::new(e.gate.as_kebab()),
            Cell::new(e.crate_scope.as_deref().unwrap_or("-")),
            Cell::new(e.file.as_deref().unwrap_or("-")),
            Cell::new(if expired {
                format!("{days} (expired)")
            } else {
                format!("{days}")
            }),
            Cell::new(&e.signed_by),
            Cell::new(elide(&e.reason, 60)),
        ]);
    }
    let mut out = anstream::stdout().lock();
    writeln!(out, "{t}").map_err(|e| anyhow!(e))?;
    Ok(())
}

fn elide(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        return s.to_owned();
    }
    let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn today() -> time::Date {
    time::OffsetDateTime::now_local()
        .unwrap_or_else(|_| time::OffsetDateTime::now_utc())
        .date()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_compiles() {
        Cli::command().debug_assert();
    }

    #[test]
    fn explain_unknown_gate_errors() {
        let r = run_explain(&ExplainArgs {
            gate: "no-such".into(),
        });
        assert!(r.is_err());
    }

    #[test]
    fn elide_short_unchanged() {
        assert_eq!(elide("hi", 10), "hi");
    }

    #[test]
    fn elide_long_shortens() {
        let s = elide("aaaaaaaaaaaaaaaaaaaa", 5);
        assert!(s.ends_with('…'));
        assert_eq!(s.chars().count(), 5);
    }
}
