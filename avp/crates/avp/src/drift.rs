//! `avp drift` — portfolio-wide drift detector.
//!
//! Walks `~/Development/PlausiDen` (or `--root <DIR>`), discovers every
//! sibling git repo, and reports each one's status against the canonical
//! `.github/workflows/avp.yml` + `avp-ratchet.toml` baselines installed
//! by [`crate::install`]. Output is a markdown report suitable for
//! pasting into a wiki page or piping into `gh issue create`.
//!
//! Three drift classes per sibling:
//! - **Clean**: every canonical file present and identical to the
//!   shipped template.
//! - **Missing**: at least one canonical file is absent.
//! - **Diverged**: at least one canonical file exists but doesn't match.
//!
//! `--open-issues` shells to `gh` to file one issue per drifted repo.
//! We deliberately don't use `octocrab` for v0.1 — `gh` is already
//! required infrastructure, and a tokio runtime + auth-flow surface
//! area is more attack surface than a single subprocess call.

#![allow(clippy::disallowed_macros)]

use std::{
    fmt::Write as _,
    fs,
    io::Write as IoWrite,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context as _, Result};
use comfy_table::{Cell, ContentArrangement, Table, presets};
use tracing::{debug, info, instrument, warn};

use crate::install::{FilePlan, plan as plan_files};

/// Drift status for a single sibling.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum DriftStatus {
    /// Every canonical file present + identical.
    Clean,
    /// At least one canonical file is missing entirely.
    Missing,
    /// At least one canonical file exists but diverges.
    Diverged,
}

impl DriftStatus {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::Missing => "missing",
            Self::Diverged => "diverged",
        }
    }

    pub(crate) const fn is_drift(self) -> bool {
        !matches!(self, Self::Clean)
    }
}

/// Per-sibling result.
#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct SiblingDrift {
    pub root: PathBuf,
    pub name: String,
    pub plans: Vec<FilePlan>,
    pub status: DriftStatus,
}

impl SiblingDrift {
    fn classify(plans: &[FilePlan]) -> DriftStatus {
        let mut missing = false;
        let mut diverged = false;
        for plan in plans {
            match plan {
                FilePlan::Create { .. } => missing = true,
                FilePlan::Diverged { .. } => diverged = true,
                FilePlan::Identical { .. } => {}
            }
        }
        if diverged {
            DriftStatus::Diverged
        } else if missing {
            DriftStatus::Missing
        } else {
            DriftStatus::Clean
        }
    }
}

/// Whole-portfolio result.
#[derive(Debug, Default)]
#[non_exhaustive]
pub(crate) struct PortfolioDrift {
    pub repos: Vec<SiblingDrift>,
}

impl PortfolioDrift {
    pub(crate) fn drifted(&self) -> Vec<&SiblingDrift> {
        self.repos.iter().filter(|s| s.status.is_drift()).collect()
    }

    pub(crate) fn counts(&self) -> (u32, u32, u32) {
        let mut clean = 0u32;
        let mut missing = 0u32;
        let mut diverged = 0u32;
        for s in &self.repos {
            match s.status {
                DriftStatus::Clean => clean += 1,
                DriftStatus::Missing => missing += 1,
                DriftStatus::Diverged => diverged += 1,
            }
        }
        (clean, missing, diverged)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Discovery
// ─────────────────────────────────────────────────────────────────────────

/// Names that drift never reports against. The doctrine repo is the
/// canonical source — its own `.github/workflows/avp.yml` is a self-host
/// build (not the sibling consumer template), and its own ratchet has the
/// self-bootstrap overrides. Any repo also containing a top-level
/// `.avp-drift-skip` file (created by an operator who knows what they're
/// doing) is excluded too.
const ALWAYS_SKIP: &[&str] = &["PlausiDen-AVP-Doctrine"];

/// Discover sibling repo roots under `portfolio_root`. A "sibling" is any
/// directory child that contains a `.git/` entry. Symlinks and hidden
/// directories are skipped.
#[instrument(level = "debug", skip_all, fields(root = %portfolio_root.display()))]
pub(crate) fn discover_siblings(portfolio_root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in fs::read_dir(portfolio_root)
        .with_context(|| format!("read_dir {}", portfolio_root.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let basename_skip = path
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with('.'));
        if basename_skip {
            continue;
        }
        let always_skip = path
            .file_name()
            .is_some_and(|n| ALWAYS_SKIP.contains(&n.to_string_lossy().as_ref()));
        if always_skip {
            debug!(path = %path.display(), "always-skip");
            continue;
        }
        if path.join(".avp-drift-skip").is_file() {
            debug!(path = %path.display(), ".avp-drift-skip present; skip");
            continue;
        }
        if path.join(".git").exists() {
            out.push(path);
        }
    }
    out.sort();
    debug!(count = out.len(), "siblings discovered");
    Ok(out)
}

/// Scan one sibling and produce its drift status.
#[instrument(level = "debug", skip_all, fields(repo = %repo_root.display()))]
pub(crate) fn scan_sibling(repo_root: &Path, version_tag: &str) -> Result<SiblingDrift> {
    let plans = plan_files(repo_root, version_tag)
        .with_context(|| format!("plan {}", repo_root.display()))?;
    let status = SiblingDrift::classify(&plans);
    let name = repo_root.file_name().map_or_else(
        || repo_root.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    Ok(SiblingDrift {
        root: repo_root.to_path_buf(),
        name,
        plans,
        status,
    })
}

/// Scan every sibling under `portfolio_root`.
#[instrument(level = "debug", skip_all)]
pub(crate) fn scan_portfolio(portfolio_root: &Path, version_tag: &str) -> Result<PortfolioDrift> {
    let siblings = discover_siblings(portfolio_root)?;
    let mut repos = Vec::with_capacity(siblings.len());
    for s in siblings {
        match scan_sibling(&s, version_tag) {
            Ok(d) => repos.push(d),
            Err(err) => warn!(repo = %s.display(), ?err, "scan failed; skipping"),
        }
    }
    Ok(PortfolioDrift { repos })
}

// ─────────────────────────────────────────────────────────────────────────
// Reporting
// ─────────────────────────────────────────────────────────────────────────

/// Render a markdown report of the portfolio drift state.
#[must_use]
pub(crate) fn render_markdown(d: &PortfolioDrift) -> String {
    let (clean, missing, diverged) = d.counts();
    let mut s = String::new();
    let _ = writeln!(s, "# AVP portfolio drift report");
    let _ = writeln!(
        s,
        "\n- **Clean**: {clean}\n- **Missing**: {missing}\n- **Diverged**: {diverged}\n- **Total**: {}",
        d.repos.len()
    );
    let _ = writeln!(s, "\n## Per-repo");
    let _ = writeln!(s, "\n| repo | status | missing | diverged |");
    let _ = writeln!(s, "|---|---|---|---|");
    for r in &d.repos {
        let missing_n = r
            .plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Create { .. }))
            .count();
        let diverged_n = r
            .plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Diverged { .. }))
            .count();
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {} |",
            r.name,
            r.status.label(),
            missing_n,
            diverged_n,
        );
    }
    s
}

/// Render a comfy-table summary for terminal output.
#[must_use]
pub(crate) fn render_table(d: &PortfolioDrift) -> Table {
    let mut t = Table::new();
    t.load_preset(presets::UTF8_BORDERS_ONLY);
    t.set_content_arrangement(ContentArrangement::Dynamic);
    t.set_header(vec![
        Cell::new("repo"),
        Cell::new("status"),
        Cell::new("missing"),
        Cell::new("diverged"),
    ]);
    for r in &d.repos {
        let missing = r
            .plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Create { .. }))
            .count();
        let diverged = r
            .plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Diverged { .. }))
            .count();
        t.add_row(vec![
            Cell::new(&r.name),
            Cell::new(r.status.label()),
            Cell::new(missing.to_string()),
            Cell::new(diverged.to_string()),
        ]);
    }
    t
}

// ─────────────────────────────────────────────────────────────────────────
// Issue filing
// ─────────────────────────────────────────────────────────────────────────

/// Open one GitHub issue per drifted sibling via `gh issue create`. Issues
/// are labeled `avp-drift` (auto-created if missing).
///
/// Returns the count of issues filed; errors per repo are logged and
/// skipped so a single failure doesn't block the rest of the portfolio.
#[instrument(level = "debug", skip_all)]
pub(crate) fn open_issues(d: &PortfolioDrift) -> Result<u32> {
    let drifted: Vec<&SiblingDrift> = d.drifted();
    if drifted.is_empty() {
        info!("no drifted repos; nothing to file");
        return Ok(0);
    }
    let mut filed = 0u32;
    for repo in drifted {
        match file_one_issue(repo) {
            Ok(()) => filed += 1,
            Err(err) => warn!(repo = %repo.name, ?err, "gh issue create failed"),
        }
    }
    Ok(filed)
}

fn file_one_issue(repo: &SiblingDrift) -> Result<()> {
    use std::process::Command;
    let title = format!("[avp-drift] {} diverges from canonical", repo.name);
    let body = render_issue_body(repo);
    // Run `gh issue create` from inside the repo so it picks up the
    // origin remote without us needing to know the owner string.
    let status = Command::new("gh")
        .current_dir(&repo.root)
        .arg("issue")
        .arg("create")
        .arg("--title")
        .arg(&title)
        .arg("--body")
        .arg(&body)
        .arg("--label")
        .arg("avp-drift")
        .status()
        .with_context(|| format!("spawn gh issue create for {}", repo.name))?;
    if !status.success() {
        anyhow::bail!("gh issue create exited {:?}", status.code());
    }
    Ok(())
}

fn render_issue_body(repo: &SiblingDrift) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "Filed automatically by `avp drift --open-issues`.\n");
    let _ = writeln!(s, "Status: **{}**", repo.status.label());
    let _ = writeln!(s, "\n## Files\n");
    for plan in &repo.plans {
        match plan {
            FilePlan::Create { rel, .. } => {
                let _ = writeln!(s, "- ❌ **missing** `{}`", rel.display());
            }
            FilePlan::Diverged { rel, .. } => {
                let _ = writeln!(s, "- ⚠️  **diverged** `{}`", rel.display());
            }
            FilePlan::Identical { rel } => {
                let _ = writeln!(s, "- ✅  identical `{}`", rel.display());
            }
        }
    }
    let _ = writeln!(
        s,
        "\nFix: `avp install` (use `--force` if local changes should be discarded)."
    );
    s
}

// ─────────────────────────────────────────────────────────────────────────
// Top-level dispatch
// ─────────────────────────────────────────────────────────────────────────

/// CLI entry — invoked from `crate::cli::run_drift`.
#[instrument(level = "debug", skip_all)]
pub(crate) fn run(
    portfolio_root: &Path,
    version_tag: &str,
    open_issues_flag: bool,
    as_markdown: bool,
) -> Result<ExitCode> {
    let d = scan_portfolio(portfolio_root, version_tag)?;

    let mut out = anstream::stdout().lock();
    if as_markdown {
        IoWrite::write_all(&mut out, render_markdown(&d).as_bytes()).ok();
    } else {
        IoWrite::write_all(&mut out, render_table(&d).to_string().as_bytes()).ok();
        IoWrite::write_all(&mut out, b"\n").ok();
        let (clean, missing, diverged) = d.counts();
        let summary = format!(
            "\nportfolio: {} clean, {} missing, {} diverged ({} total)\n",
            clean,
            missing,
            diverged,
            d.repos.len(),
        );
        IoWrite::write_all(&mut out, summary.as_bytes()).ok();
    }

    if open_issues_flag {
        let filed = open_issues(&d)?;
        let line = format!("filed {filed} drift issue(s)\n");
        IoWrite::write_all(&mut out, line.as_bytes()).ok();
    }

    Ok(if d.drifted().is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn td() -> tempfile::TempDir {
        tempfile::tempdir().expect("tmpdir")
    }

    fn make_sibling(parent: &Path, name: &str) -> PathBuf {
        let dir = parent.join(name);
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        dir
    }

    #[test]
    fn discover_skips_non_git_dirs() {
        let dir = td();
        make_sibling(dir.path(), "git-repo");
        std::fs::create_dir_all(dir.path().join("plain-dir")).unwrap();
        std::fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        let found = discover_siblings(dir.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("git-repo"));
    }

    #[test]
    fn discover_skips_doctrine_repo() {
        let dir = td();
        make_sibling(dir.path(), "PlausiDen-AVP-Doctrine");
        make_sibling(dir.path(), "PlausiDen-Engine");
        let found = discover_siblings(dir.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("PlausiDen-Engine"));
    }

    #[test]
    fn discover_respects_drift_skip_marker() {
        let dir = td();
        let opt_out = make_sibling(dir.path(), "opt-out");
        std::fs::write(opt_out.join(".avp-drift-skip"), "").unwrap();
        make_sibling(dir.path(), "tracked");
        let found = discover_siblings(dir.path()).unwrap();
        assert_eq!(found.len(), 1);
        assert!(found[0].ends_with("tracked"));
    }

    #[test]
    fn missing_files_classify_as_missing() {
        let dir = td();
        let sib = make_sibling(dir.path(), "sib");
        let drift = scan_sibling(&sib, "v0.1.0").unwrap();
        assert_eq!(drift.status, DriftStatus::Missing);
    }

    #[test]
    fn diverged_files_classify_as_diverged() {
        let dir = td();
        let sib = make_sibling(dir.path(), "sib");
        // write a divergent workflow
        let workflow = sib.join(".github/workflows/avp.yml");
        std::fs::create_dir_all(workflow.parent().unwrap()).unwrap();
        std::fs::write(&workflow, "name: not-the-canonical\n").unwrap();
        let drift = scan_sibling(&sib, "v0.1.0").unwrap();
        assert_eq!(drift.status, DriftStatus::Diverged);
    }

    #[test]
    fn clean_classifies_as_clean() {
        let dir = td();
        let sib = make_sibling(dir.path(), "sib");
        // install canonical files
        for (rel, content) in crate::install::canonical_files("v0.1.0") {
            let abs = sib.join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, content).unwrap();
        }
        let drift = scan_sibling(&sib, "v0.1.0").unwrap();
        assert_eq!(drift.status, DriftStatus::Clean);
    }

    #[test]
    fn portfolio_aggregates() {
        let dir = td();
        let _a = make_sibling(dir.path(), "alpha");
        let beta = make_sibling(dir.path(), "beta");
        for (rel, content) in crate::install::canonical_files("v0.1.0") {
            let abs = beta.join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, content).unwrap();
        }

        let p = scan_portfolio(dir.path(), "v0.1.0").unwrap();
        let (clean, missing, diverged) = p.counts();
        assert_eq!(clean, 1);
        assert_eq!(missing, 1);
        assert_eq!(diverged, 0);
        assert_eq!(p.drifted().len(), 1);
    }

    #[test]
    fn markdown_includes_summary_and_table() {
        let dir = td();
        let _ = make_sibling(dir.path(), "alpha");
        let p = scan_portfolio(dir.path(), "v0.1.0").unwrap();
        let md = render_markdown(&p);
        assert!(md.contains("# AVP portfolio drift report"));
        assert!(md.contains("alpha"));
        assert!(md.contains("missing"));
    }
}
