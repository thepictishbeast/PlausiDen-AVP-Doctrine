//! `debug-remove` gate — fail on any `DEBUG-REMOVE:` literal.
//!
//! **Not ratchetable.** This is the doctrine's pre-release lint; an
//! in-flight `// DEBUG-REMOVE:` comment must always block merge.
//!
//! Implementation: text grep over every loaded [`RustSource`]. We do not
//! parse Rust before searching — `DEBUG-REMOVE:` could appear in a string
//! literal or a doc comment and still represent dev-time debugging that
//! should be stripped before release. The conservative choice is to flag
//! all occurrences regardless of syntactic position.
//!
//! Exclusions:
//! - The literal `DEBUG-REMOVE:` is allowed inside this very source file
//!   (we have to write the marker to grep for it). We skip files whose
//!   relative path is exactly the gate's own implementation path.

use avp_core::{Context, Finding, Gate, GateId, Location};
use tracing::{debug, instrument};

use crate::source::RustSource;

/// `debug-remove` implementation.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct DebugRemove;

impl DebugRemove {
    /// The literal we're hunting for. Defined as a const-built byte slice
    /// so the gate's own source doesn't accidentally trip itself.
    const NEEDLE: &'static str = concat!("DEBUG", "-REMOVE:");

    /// Run the gate against an explicit slice of pre-loaded sources.
    /// This shape exists so unit tests can synthesize sources without
    /// touching the filesystem.
    #[must_use]
    pub fn run_on(&self, sources: &[RustSource]) -> Vec<Finding> {
        let mut findings = Vec::new();
        for src in sources {
            scan_source(src, &mut findings);
        }
        findings
    }
}

impl Gate for DebugRemove {
    fn id(&self) -> GateId {
        GateId::DebugRemove
    }

    #[instrument(level = "debug", skip_all)]
    fn run(&self, ctx: &Context<'_>) -> Vec<Finding> {
        let (sources, errors) = crate::source::collect_sources(&ctx.repo.path);
        let mut findings = Vec::with_capacity(errors.len());
        for err in &errors {
            // Parse failures don't block debug-remove (we operate on raw text),
            // but they're still worth surfacing as warnings.
            findings.push(Finding::warning(
                self.id(),
                Location::file("/"),
                format!("source error during scan: {err}"),
            ));
        }
        findings.extend(self.run_on(&sources));
        debug!(found = findings.len(), "debug-remove scan complete");
        findings
    }
}

/// Scan a single source, appending any findings.
fn scan_source(src: &RustSource, out: &mut Vec<Finding>) {
    if is_self_path(&src.rel_path) {
        return;
    }
    for (line_idx_zero, line) in src.text.split_inclusive('\n').enumerate() {
        if line.contains(DebugRemove::NEEDLE) {
            let line_no = u32::try_from(line_idx_zero + 1).unwrap_or(u32::MAX);
            out.push(Finding::error(
                GateId::DebugRemove,
                Location::line(src.rel_path.clone(), line_no),
                "DEBUG-REMOVE marker present (must be stripped before merge)",
            ));
        }
    }
}

/// Files that mention the `DEBUG-REMOVE:` marker in their own doc/source
/// because they're part of the gate's implementation or its public type
/// surface. Doc comments describing the gate must be allowed to use the
/// literal; the gate cannot reasonably distinguish a *describing* mention
/// from a *forgotten* one without parsing the comment kind.
const SELF_EXEMPT_SUFFIXES: &[&str] = &[
    "crates/avp-rust/src/gates/debug_remove.rs",
    "crates/avp-rust/src/lib.rs",
    "crates/avp-core/src/gate.rs",
];

/// True if the given repo-relative path is one of this gate's documenting
/// surfaces (allowed to mention the literal in prose).
fn is_self_path(rel: &std::path::Path) -> bool {
    let s = rel.to_string_lossy().replace('\\', "/");
    SELF_EXEMPT_SUFFIXES
        .iter()
        .any(|suffix| s.ends_with(suffix))
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use avp_core::Severity;

    use super::*;
    use crate::source::RustSource;

    fn src(rel: &str, text: &str) -> RustSource {
        RustSource::parse(
            PathBuf::from(format!("/tmp/{rel}")),
            PathBuf::from(rel),
            text.to_string(),
        )
        .expect("test source parses")
    }

    #[test]
    fn clean_source_finds_nothing() {
        let s = src("src/lib.rs", "pub fn ok() {}\n");
        let g = DebugRemove;
        let findings = g.run_on(&[s]);
        assert!(findings.is_empty());
    }

    #[test]
    fn line_with_marker_is_flagged() {
        let s = src(
            "src/lib.rs",
            // DEBUG-REMOVE marker injected via concat to avoid self-trip
            &format!(
                "// {}: tidy this up\npub fn x() {{}}\n",
                concat!("DEBUG", "-REMOVE")
            ),
        );
        let g = DebugRemove;
        let findings = g.run_on(&[s]);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(findings[0].location.line, Some(1));
        assert_eq!(findings[0].gate, GateId::DebugRemove);
    }

    #[test]
    fn marker_in_string_literal_still_flagged() {
        // Doctrine: any occurrence is an error, even in strings — devs use
        // string-embedded markers as a workaround that the gate must reject.
        let needle = concat!("DEBUG", "-REMOVE:");
        let s = src(
            "src/lib.rs",
            &format!("pub fn x() {{ let _ = \"{needle} hi\"; }}\n"),
        );
        let g = DebugRemove;
        let findings = g.run_on(&[s]);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn multiple_lines_all_flagged() {
        let needle = concat!("DEBUG", "-REMOVE:");
        let text = format!("// {needle} a\npub fn x() {{}}\n// {needle} b\npub fn y() {{}}\n",);
        let s = src("src/lib.rs", &text);
        let g = DebugRemove;
        let findings = g.run_on(&[s]);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].location.line, Some(1));
        assert_eq!(findings[1].location.line, Some(3));
    }

    #[test]
    fn self_path_skipped() {
        let needle = concat!("DEBUG", "-REMOVE:");
        let s = src(
            "crates/avp-rust/src/gates/debug_remove.rs",
            &format!("// {needle} ignore\n"),
        );
        let g = DebugRemove;
        let findings = g.run_on(&[s]);
        assert!(findings.is_empty());
    }
}
