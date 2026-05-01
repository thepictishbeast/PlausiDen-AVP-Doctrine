//! `avp install` — drop the canonical `.github/workflows/avp.yml` (and
//! a starter `avp-ratchet.toml`) into a sibling repo.
//!
//! Three modes:
//! - default: write any missing files, refuse to overwrite locally-modified
//!   ones, leave identical-content files alone.
//! - `--dry-run`: print a diff per touched file and exit; never writes.
//! - `--force`: overwrite even when the existing file diverges from the
//!   canonical template.
//!
//! Templates live in [`templates`] as `include_str!` constants so the
//! shipped binary is self-contained — siblings don't need to clone the
//! doctrine repo to run `avp install`.

#![allow(clippy::disallowed_macros)]

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context as _, Result};
use tracing::{debug, info, instrument, warn};

/// Per-file plan computed before any write happens. Storing the plan
/// separately makes `--dry-run` trivially correct and gives the user a
/// clean "what changed" summary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub(crate) enum FilePlan {
    /// Path doesn't exist; we'll create it.
    Create { rel: PathBuf, content: String },
    /// Path exists and matches the canonical bytes — no action.
    Identical { rel: PathBuf },
    /// Path exists with different content. Requires `--force` to overwrite.
    Diverged {
        rel: PathBuf,
        existing: String,
        canonical: String,
    },
}

impl FilePlan {
    #[cfg(test)] // currently used only by integration tests
    pub(crate) const fn rel(&self) -> &PathBuf {
        match self {
            Self::Create { rel, .. } | Self::Identical { rel } | Self::Diverged { rel, .. } => rel,
        }
    }

    /// Whether applying this plan in default mode would block.
    pub(crate) const fn blocks_default(&self) -> bool {
        matches!(self, Self::Diverged { .. })
    }
}

/// Compute the canonical file set that `avp install` would write.
///
/// `version_tag` is the avp release tag the workflow should pin (e.g. `"v0.1.0"`).
///
/// The set includes:
/// - `.github/workflows/avp.yml` — calls the composite action.
/// - `avp-ratchet.toml` — empty starter (commented example).
/// - `deny.toml`, `clippy.toml`, `rustfmt.toml`, `rust-toolchain.toml`
///   — the canonical lint configs from the avp workspace itself, so
///   every sibling lints with the same rules `avp check rust` enforces.
#[must_use]
pub(crate) fn canonical_files(version_tag: &str) -> Vec<(PathBuf, String)> {
    let workflow = templates::AVP_WORKFLOW.replace("{{VERSION}}", version_tag);
    vec![
        (PathBuf::from(".github/workflows/avp.yml"), workflow),
        (
            PathBuf::from("avp-ratchet.toml"),
            templates::RATCHET_STARTER.to_owned(),
        ),
        (PathBuf::from("deny.toml"), templates::DENY.to_owned()),
        (PathBuf::from("clippy.toml"), templates::CLIPPY.to_owned()),
        (PathBuf::from("rustfmt.toml"), templates::RUSTFMT.to_owned()),
        (
            PathBuf::from("rust-toolchain.toml"),
            templates::RUST_TOOLCHAIN.to_owned(),
        ),
    ]
}

/// Build a `FilePlan` per canonical file given the target repo root.
#[instrument(level = "debug", skip_all)]
pub(crate) fn plan(repo_root: &Path, version_tag: &str) -> io::Result<Vec<FilePlan>> {
    let mut plans = Vec::new();
    for (rel, canonical) in canonical_files(version_tag) {
        let abs = repo_root.join(&rel);
        let plan = if abs.exists() {
            let existing = fs::read_to_string(&abs)?;
            if existing == canonical {
                FilePlan::Identical { rel }
            } else {
                FilePlan::Diverged {
                    rel,
                    existing,
                    canonical,
                }
            }
        } else {
            FilePlan::Create {
                rel,
                content: canonical,
            }
        };
        plans.push(plan);
    }
    Ok(plans)
}

/// Apply a slice of [`FilePlan`]s. Returns the count of files actually
/// written.
#[instrument(level = "debug", skip_all)]
pub(crate) fn apply(repo_root: &Path, plans: &[FilePlan], force: bool) -> Result<u32> {
    let mut written = 0u32;
    for plan in plans {
        match plan {
            FilePlan::Create { rel, content } => {
                let abs = repo_root.join(rel);
                if let Some(parent) = abs.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("mkdir -p {}", parent.display()))?;
                }
                fs::write(&abs, content).with_context(|| format!("write {}", abs.display()))?;
                info!(path = %rel.display(), "created");
                written += 1;
            }
            FilePlan::Identical { rel } => {
                debug!(path = %rel.display(), "identical, skip");
            }
            FilePlan::Diverged { rel, canonical, .. } => {
                if !force {
                    return Err(anyhow::anyhow!(
                        "{} diverges from canonical; pass --force to overwrite",
                        rel.display()
                    ));
                }
                let abs = repo_root.join(rel);
                fs::write(&abs, canonical).with_context(|| format!("write {}", abs.display()))?;
                warn!(path = %rel.display(), "force-overwritten");
                written += 1;
            }
        }
    }
    Ok(written)
}

/// Render a unified-diff-ish view of a `FilePlan` for `--dry-run`.
#[must_use]
pub(crate) fn render_plan(plan: &FilePlan) -> String {
    match plan {
        FilePlan::Create { rel, content } => {
            format!(
                "+ would create {}\n{}\n",
                rel.display(),
                prefix_lines(content, "  + ")
            )
        }
        FilePlan::Identical { rel } => format!("= identical {}\n", rel.display()),
        FilePlan::Diverged {
            rel,
            existing,
            canonical,
        } => format!(
            "! diverged {} (use --force to overwrite)\n  existing ({} bytes):\n{}\n  canonical ({} bytes):\n{}\n",
            rel.display(),
            existing.len(),
            prefix_lines(existing, "  - "),
            canonical.len(),
            prefix_lines(canonical, "  + "),
        ),
    }
}

fn prefix_lines(s: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(s.len() + prefix.len());
    for line in s.lines() {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Top-level entry called by the CLI dispatcher.
///
/// `version_tag` defaults to the env var `AVP_INSTALL_VERSION` if
/// unset upstream; the CLI passes a final string in.
#[instrument(level = "debug", skip_all, fields(repo = %repo_root.display(), dry_run, force))]
pub(crate) fn run(
    repo_root: &Path,
    version_tag: &str,
    dry_run: bool,
    force: bool,
) -> Result<ExitCode> {
    let plans = plan(repo_root, version_tag)
        .with_context(|| format!("plan install at {}", repo_root.display()))?;

    if dry_run {
        for p in &plans {
            print!("{}", render_plan(p));
        }
        let any_blocked = plans.iter().any(FilePlan::blocks_default);
        return Ok(if any_blocked {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        });
    }

    if !force {
        let blockers: Vec<&FilePlan> = plans.iter().filter(|p| p.blocks_default()).collect();
        if !blockers.is_empty() {
            for b in blockers {
                eprintln!("avp install: {}", render_plan(b));
            }
            return Err(anyhow::anyhow!(
                "{} file(s) diverge — re-run with --force or --dry-run",
                plans.iter().filter(|p| p.blocks_default()).count()
            ));
        }
    }

    let written = apply(repo_root, &plans, force)?;
    println!(
        "avp install: {} file(s) written, {} identical, {} diverged",
        written,
        plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Identical { .. }))
            .count(),
        plans
            .iter()
            .filter(|p| matches!(p, FilePlan::Diverged { .. }))
            .count(),
    );
    Ok(ExitCode::SUCCESS)
}

// ─────────────────────────────────────────────────────────────────────────
// Embedded templates
// ─────────────────────────────────────────────────────────────────────────

mod templates {
    /// The canonical sibling-repo workflow. `{{VERSION}}` is replaced
    /// with the avp release tag (e.g. `v0.1.0`).
    pub(super) const AVP_WORKFLOW: &str = include_str!("../../../templates/sibling-avp.yml");
    /// Starter ratchet file with example commented out — siblings start
    /// from a clean template and add overrides as needed.
    pub(super) const RATCHET_STARTER: &str =
        include_str!("../../../templates/sibling-avp-ratchet.toml");
    /// Canonical cargo-deny config, sourced from the avp workspace.
    pub(super) const DENY: &str = include_str!("../../../deny.toml");
    /// Canonical clippy tuning, sourced from the avp workspace.
    pub(super) const CLIPPY: &str = include_str!("../../../clippy.toml");
    /// Canonical rustfmt config, sourced from the avp workspace.
    pub(super) const RUSTFMT: &str = include_str!("../../../rustfmt.toml");
    /// Canonical rust-toolchain pin, sourced from the avp workspace.
    pub(super) const RUST_TOOLCHAIN: &str = include_str!("../../../rust-toolchain.toml");
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

    #[test]
    fn canonical_files_substitutes_version() {
        let files = canonical_files("v0.9.9");
        let workflow = files
            .iter()
            .find(|(p, _)| p.ends_with("avp.yml"))
            .map(|(_, c)| c)
            .unwrap();
        assert!(workflow.contains("v0.9.9"), "{workflow}");
        assert!(!workflow.contains("{{VERSION}}"));
    }

    #[test]
    fn plan_creates_when_absent() {
        let dir = td();
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        for p in &plans {
            assert!(matches!(p, FilePlan::Create { .. }), "{p:?}");
        }
    }

    #[test]
    fn plan_identical_when_match() {
        let dir = td();
        for (rel, content) in canonical_files("v0.1.0") {
            let abs = dir.path().join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, content).unwrap();
        }
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        for p in &plans {
            assert!(matches!(p, FilePlan::Identical { .. }), "{p:?}");
        }
    }

    #[test]
    fn plan_diverged_when_modified() {
        let dir = td();
        for (rel, _) in canonical_files("v0.1.0") {
            let abs = dir.path().join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, "local changes").unwrap();
        }
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        for p in &plans {
            assert!(matches!(p, FilePlan::Diverged { .. }), "{p:?}");
        }
    }

    #[test]
    fn apply_creates_when_absent() {
        let dir = td();
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        let n = apply(dir.path(), &plans, false).unwrap();
        assert_eq!(n, u32::try_from(plans.len()).unwrap());
        for p in &plans {
            assert!(dir.path().join(p.rel()).is_file());
        }
    }

    #[test]
    fn apply_refuses_diverged_without_force() {
        let dir = td();
        for (rel, _) in canonical_files("v0.1.0") {
            let abs = dir.path().join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, "local changes").unwrap();
        }
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        let err = apply(dir.path(), &plans, false).unwrap_err();
        assert!(format!("{err}").contains("--force"));
    }

    #[test]
    fn apply_overwrites_with_force() {
        let dir = td();
        for (rel, _) in canonical_files("v0.1.0") {
            let abs = dir.path().join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(abs, "local changes").unwrap();
        }
        let plans = plan(dir.path(), "v0.1.0").unwrap();
        apply(dir.path(), &plans, true).unwrap();
        // After force-apply, contents match canonical.
        for (rel, expected) in canonical_files("v0.1.0") {
            let actual = std::fs::read_to_string(dir.path().join(&rel)).unwrap();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn render_plan_shapes() {
        let create = FilePlan::Create {
            rel: PathBuf::from("a.toml"),
            content: "x".into(),
        };
        let identical = FilePlan::Identical {
            rel: PathBuf::from("b.toml"),
        };
        let diverged = FilePlan::Diverged {
            rel: PathBuf::from("c.toml"),
            existing: "old".into(),
            canonical: "new".into(),
        };
        assert!(render_plan(&create).starts_with("+ would create"));
        assert!(render_plan(&identical).starts_with("= identical"));
        assert!(render_plan(&diverged).starts_with("! diverged"));
    }
}
