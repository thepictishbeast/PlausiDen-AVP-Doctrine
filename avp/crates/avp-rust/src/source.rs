//! Loading + classifying Rust source files for AST-based gates.
//!
//! [`RustSource`] owns three things: the original UTF-8 text, the parsed
//! `syn::File`, and a lazily-built line index that converts byte offsets
//! and `proc_macro2::Span` line numbers to 1-based line numbers + window
//! slices. Gates use the AST for accurate item detection and the line
//! index to grep for comments (which `syn` does not preserve).
//!
//! [`SourceClass`] classifies each file by canonical Cargo layout —
//! library / binary / test / bench / example. Different gates apply to
//! different classes (e.g. `forbidden-call` is library-only because
//! `bin/` and `tests/` are allowed to `println!`).

use std::{
    fs,
    path::{Path, PathBuf},
};

use ignore::WalkBuilder;
use thiserror::Error;
use tracing::{debug, instrument, trace, warn};

// ─────────────────────────────────────────────────────────────────────────
// Classification
// ─────────────────────────────────────────────────────────────────────────

/// Canonical Cargo source classification.
///
/// Rules:
/// - `Bin` = `src/main.rs` or any file under `src/bin/`.
/// - `Test` = any file under `tests/` (integration tests).
/// - `Bench` = under `benches/`.
/// - `Example` = under `examples/`.
/// - `Library` = under `src/` and not `Bin`.
/// - `Other` = files outside the above (build scripts, fuzz harnesses,
///   xtask, etc.) — gates skip these by default.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum SourceClass {
    /// Library code under `src/` (the most-restricted class).
    Library,
    /// Binary entry point: `src/main.rs` or `src/bin/*.rs`.
    Bin,
    /// Integration test under `tests/`.
    Test,
    /// Benchmark under `benches/`.
    Bench,
    /// Example under `examples/`.
    Example,
    /// Anything else (build.rs, xtask, fuzz/, etc.).
    Other,
}

impl SourceClass {
    /// Classify a *repo-relative* path.
    #[must_use]
    pub fn classify(rel: &Path) -> Self {
        // Convert to forward-slash string for stable matching.
        let s = rel.to_string_lossy();
        let s = s.replace('\\', "/");

        if s.contains("/tests/") || s.starts_with("tests/") {
            return Self::Test;
        }
        if s.contains("/benches/") || s.starts_with("benches/") {
            return Self::Bench;
        }
        if s.contains("/examples/") || s.starts_with("examples/") {
            return Self::Example;
        }
        if s.contains("/src/bin/") || s.starts_with("src/bin/") {
            return Self::Bin;
        }
        if s.ends_with("/src/main.rs") || s == "src/main.rs" {
            return Self::Bin;
        }
        if s.contains("/src/") || s.starts_with("src/") {
            return Self::Library;
        }
        Self::Other
    }

    /// Whether this class is "library-style" code that the strict gates
    /// (`forbidden-call`, `bug-assumption` for `pub fn` density) apply to.
    #[must_use]
    pub const fn is_library(self) -> bool {
        matches!(self, Self::Library)
    }

    /// Whether this class produces tests (counts toward test-density numerator).
    #[must_use]
    pub const fn is_test(self) -> bool {
        matches!(self, Self::Test)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// RustSource
// ─────────────────────────────────────────────────────────────────────────

/// One Rust file: absolute path, repo-relative path, original text, parsed
/// AST, classification, and a line-start byte index.
#[derive(Debug)]
#[non_exhaustive]
pub struct RustSource {
    /// Absolute path on disk.
    pub abs_path: PathBuf,
    /// Path relative to the repo root.
    pub rel_path: PathBuf,
    /// Classification (library / bin / test / …).
    pub class: SourceClass,
    /// Original UTF-8 source text.
    pub text: String,
    /// Parsed AST.
    pub ast: syn::File,
    /// Byte offset at the start of each 1-based line. Index 0 holds line 1.
    /// `line_starts.len()` = total line count + 1 (sentinel for the EOF line).
    line_starts: Vec<usize>,
}

impl RustSource {
    /// Load + parse a single file.
    #[instrument(level = "debug", skip_all, fields(abs = %abs_path.as_ref().display(), rel = %rel_path.as_ref().display()))]
    pub fn load(
        abs_path: impl AsRef<Path>,
        rel_path: impl AsRef<Path>,
    ) -> Result<Self, SourceError> {
        let abs_path = abs_path.as_ref();
        let rel_path = rel_path.as_ref();
        let text = fs::read_to_string(abs_path).map_err(|source| SourceError::Read {
            path: abs_path.to_path_buf(),
            source,
        })?;
        Self::parse(abs_path.to_path_buf(), rel_path.to_path_buf(), text)
    }

    /// Parse pre-loaded text. Useful for tests.
    pub fn parse(abs_path: PathBuf, rel_path: PathBuf, text: String) -> Result<Self, SourceError> {
        let ast = syn::parse_file(&text).map_err(|source| SourceError::Parse {
            path: rel_path.clone(),
            source,
        })?;
        let mut class = SourceClass::classify(&rel_path);
        // Refine: if the file is on disk under a crate with no `src/lib.rs`,
        // promote Library → Bin (catches `crates/<bin>/src/cli.rs`-style
        // helper modules of binary-only crates). The `abs_path.exists()`
        // guard keeps test fixtures with synthetic paths from being
        // misclassified.
        if class == SourceClass::Library && abs_path.exists() && !crate_has_lib_rs(&abs_path) {
            class = SourceClass::Bin;
        }
        let line_starts = compute_line_starts(&text);
        debug!(class = ?class, lines = line_starts.len() - 1, "parsed");
        Ok(Self {
            abs_path,
            rel_path,
            class,
            text,
            ast,
            line_starts,
        })
    }

    /// 1-based line for a 0-based byte offset.
    #[must_use]
    pub fn line_for_offset(&self, byte_offset: usize) -> u32 {
        match self.line_starts.binary_search(&byte_offset) {
            Ok(idx) => u32::try_from(idx + 1).unwrap_or(u32::MAX),
            Err(idx) => u32::try_from(idx).unwrap_or(u32::MAX).max(1),
        }
    }

    /// Slice of source text for a *1-based, inclusive* line range.
    /// Out-of-range bounds are clamped.
    #[must_use]
    pub fn line_window(&self, start_line: u32, end_line: u32) -> &str {
        let total = self.line_count();
        let start = start_line.clamp(1, total);
        let end = end_line.clamp(start, total);
        let s_idx = self.line_starts[(start - 1) as usize];
        let e_idx = if (end as usize) < self.line_starts.len() {
            self.line_starts[end as usize]
        } else {
            self.text.len()
        };
        // Safe slice — line_starts are character boundaries by construction.
        &self.text[s_idx..e_idx]
    }

    /// Total 1-based line count.
    #[must_use]
    pub fn line_count(&self) -> u32 {
        // line_starts has total_lines + 1 entries; the last is text.len() sentinel.
        u32::try_from(self.line_starts.len().saturating_sub(1)).unwrap_or(u32::MAX)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Walking
// ─────────────────────────────────────────────────────────────────────────

/// Walk a repo root, returning `(abs, rel)` pairs for every `.rs` file
/// that's tracked under the canonical Cargo layout. Respects `.gitignore`,
/// skips `target/`, and skips hidden directories.
#[instrument(level = "debug", skip_all, fields(root = %root.as_ref().display()))]
pub fn discover_rs_files(root: impl AsRef<Path>) -> Result<Vec<(PathBuf, PathBuf)>, SourceError> {
    let root = root.as_ref();
    let mut out = Vec::new();
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .hidden(false)
        .add_custom_ignore_filename(".avp-ignore")
        .build();
    for dent in walker {
        let dent = match dent {
            Ok(d) => d,
            Err(err) => {
                warn!(?err, "walker error");
                continue;
            }
        };
        let path = dent.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        // Skip target/ even if not gitignored (Cargo build dir).
        if path
            .strip_prefix(root)
            .ok()
            .is_some_and(|rel| rel.components().any(|c| c.as_os_str() == "target"))
        {
            trace!(path = %path.display(), "skip target/");
            continue;
        }
        let rel = path
            .strip_prefix(root)
            .map_or_else(|_| path.to_path_buf(), Path::to_path_buf);
        out.push((path.to_path_buf(), rel));
    }
    debug!(count = out.len(), "discovered rust files");
    Ok(out)
}

/// Discover + load every Rust source under `root`.
///
/// Files that fail to parse emit a `SourceError::Parse` per file but do
/// not abort — gates that need the AST will skip those files; gates that
/// only need text (debug-remove) can still run via the raw text in
/// subsequent passes.
#[must_use]
pub fn collect_sources(root: impl AsRef<Path>) -> (Vec<RustSource>, Vec<SourceError>) {
    let root = root.as_ref();
    let entries = match discover_rs_files(root) {
        Ok(v) => v,
        Err(err) => return (Vec::new(), vec![err]),
    };
    let mut sources = Vec::with_capacity(entries.len());
    let mut errors = Vec::new();
    for (abs, rel) in entries {
        match RustSource::load(&abs, &rel) {
            Ok(s) => sources.push(s),
            Err(err) => errors.push(err),
        }
    }
    (sources, errors)
}

// ─────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────

/// Find the Cargo crate name that owns the file at `abs_path`.
///
/// Reads the closest ancestor `Cargo.toml`. Returns `None` if no ancestor
/// has a Cargo.toml or the manifest doesn't declare a `[package].name`.
#[must_use]
pub fn crate_name_for_path(abs_path: &Path) -> Option<String> {
    let crate_dir = abs_path
        .ancestors()
        .find(|a| a.join("Cargo.toml").is_file())?;
    let manifest = std::fs::read_to_string(crate_dir.join("Cargo.toml")).ok()?;
    parse_crate_name(&manifest)
}

/// Tiny TOML peeker that reads `[package] name = "x"`. We avoid a full
/// `toml` dep here because every gate calls this for every finding —
/// keeping it cheap matters.
fn parse_crate_name(manifest: &str) -> Option<String> {
    let mut in_package = false;
    for raw_line in manifest.lines() {
        let line = raw_line.split('#').next()?.trim();
        if line.starts_with('[') {
            in_package = line == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        if let Some(rest) = line.strip_prefix("name") {
            let rest = rest.trim_start();
            let rest = rest.strip_prefix('=')?.trim_start();
            // Accept "name" (quoted) or { workspace = true }.
            if let Some(end_of_value) = rest.strip_prefix('"') {
                let end = end_of_value.find('"')?;
                return Some(end_of_value[..end].to_owned());
            }
        }
    }
    None
}

/// Walk `abs_path` ancestors until we find a `Cargo.toml`. If that crate
/// directory has a `src/lib.rs`, the crate exports a library. Returns
/// `false` for binary-only crates and when no Cargo.toml is found.
fn crate_has_lib_rs(abs_path: &Path) -> bool {
    let Some(crate_dir) = abs_path
        .ancestors()
        .find(|a| a.join("Cargo.toml").is_file())
    else {
        return false;
    };
    crate_dir.join("src/lib.rs").is_file()
}

/// Compute 0-based byte offsets of each line start. The returned vec has
/// `lines + 1` entries — the last is the byte length of the input
/// (sentinel for slicing past the last line).
fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::with_capacity(text.len() / 32 + 1);
    starts.push(0);
    for (i, b) in text.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts.push(text.len());
    starts
}

// ─────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────

/// Errors raised while loading or parsing Rust sources.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SourceError {
    /// Reading the file failed.
    #[error("read {path}: {source}")]
    Read {
        /// Offending path.
        path: PathBuf,
        /// Underlying io error.
        source: std::io::Error,
    },
    /// `syn::parse_file` rejected the input.
    #[error("parse {path}: {source}")]
    Parse {
        /// Offending path.
        path: PathBuf,
        /// Underlying syn error.
        source: syn::Error,
    },
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_library() {
        assert_eq!(
            SourceClass::classify(Path::new("src/lib.rs")),
            SourceClass::Library
        );
        assert_eq!(
            SourceClass::classify(Path::new("crates/foo/src/inner.rs")),
            SourceClass::Library
        );
    }

    #[test]
    fn classify_bin() {
        assert_eq!(
            SourceClass::classify(Path::new("src/main.rs")),
            SourceClass::Bin
        );
        assert_eq!(
            SourceClass::classify(Path::new("src/bin/tool.rs")),
            SourceClass::Bin
        );
        assert_eq!(
            SourceClass::classify(Path::new("crates/foo/src/bin/x.rs")),
            SourceClass::Bin
        );
    }

    #[test]
    fn classify_test_bench_example() {
        assert_eq!(
            SourceClass::classify(Path::new("tests/it.rs")),
            SourceClass::Test
        );
        assert_eq!(
            SourceClass::classify(Path::new("benches/bench.rs")),
            SourceClass::Bench
        );
        assert_eq!(
            SourceClass::classify(Path::new("examples/demo.rs")),
            SourceClass::Example
        );
    }

    #[test]
    fn classify_other() {
        assert_eq!(
            SourceClass::classify(Path::new("build.rs")),
            SourceClass::Other
        );
        assert_eq!(
            SourceClass::classify(Path::new("xtask/main.rs")),
            SourceClass::Other
        );
    }

    #[test]
    fn parse_minimal() {
        let s = RustSource::parse(
            PathBuf::from("/tmp/x.rs"),
            PathBuf::from("src/lib.rs"),
            "pub fn one() {}\npub fn two() {}\n".into(),
        )
        .unwrap();
        assert_eq!(s.class, SourceClass::Library);
        // 3 entries: [0, len_of_line1, len_of_lines12, sentinel]
        assert!(s.line_starts.len() >= 3);
        assert_eq!(s.line_count(), 3); // last empty line counts
    }

    #[test]
    fn line_for_offset_round_trip() {
        // line 1 starts at 0; line 2 at 2 ("a\n"); line 3 at 6 ("a\nb\n").
        let text = "a\nb\nc\n";
        let s = RustSource::parse(
            PathBuf::from("/tmp/x.rs"),
            PathBuf::from("src/lib.rs"),
            // SAFETY: a single ident parses as a (path-only) item-position
            // expression; that's enough for syn to accept. Wrap as constants
            // so we get valid Rust.
            "const A: u8 = 1;\nconst B: u8 = 2;\nconst C: u8 = 3;\n".into(),
        )
        .unwrap();
        let _ = text; // silence unused
        // line 1 starts at 0; line 2 at 17 ("const A: u8 = 1;\n"); etc.
        assert_eq!(s.line_for_offset(0), 1);
        assert_eq!(s.line_for_offset(16), 1);
        assert_eq!(s.line_for_offset(17), 2);
    }

    #[test]
    fn line_window_inclusive() {
        let text = "pub fn one() {}\npub fn two() {}\npub fn three() {}\npub fn four() {}\n";
        let s = RustSource::parse(
            PathBuf::from("/tmp/x.rs"),
            PathBuf::from("src/lib.rs"),
            text.into(),
        )
        .unwrap();
        assert_eq!(s.line_window(2, 3), "pub fn two() {}\npub fn three() {}\n");
        assert_eq!(s.line_window(1, 1), "pub fn one() {}\n");
        // clamped — beyond EOF
        assert_eq!(s.line_window(1, 99), text);
    }

    #[test]
    fn parse_failure_classified_correctly() {
        let err = RustSource::parse(
            PathBuf::from("/tmp/x.rs"),
            PathBuf::from("src/lib.rs"),
            "this is not rust !@#$".into(),
        )
        .unwrap_err();
        assert!(matches!(err, SourceError::Parse { .. }));
    }

    #[test]
    fn parse_crate_name_quoted() {
        let m = "[package]\nname = \"avp-core\"\nversion = \"0.1\"\n";
        assert_eq!(parse_crate_name(m).as_deref(), Some("avp-core"));
    }

    #[test]
    fn parse_crate_name_with_comment() {
        let m = "[package]\nname = \"avp\"  # binary\n";
        assert_eq!(parse_crate_name(m).as_deref(), Some("avp"));
    }

    #[test]
    fn parse_crate_name_skips_other_sections() {
        let m = "[lib]\nname = \"wrong\"\n[package]\nname = \"right\"\n";
        assert_eq!(parse_crate_name(m).as_deref(), Some("right"));
    }

    #[test]
    fn parse_crate_name_workspace_inheritance_returns_none() {
        let m = "[package]\nname = { workspace = true }\n";
        assert_eq!(parse_crate_name(m), None);
    }

    #[test]
    fn discover_walks_tempdir() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(td.path().join("src")).unwrap();
        std::fs::write(td.path().join("src/lib.rs"), "pub fn x() {}").unwrap();
        std::fs::write(td.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        std::fs::create_dir_all(td.path().join("target/debug")).unwrap();
        std::fs::write(td.path().join("target/debug/should-skip.rs"), "fn x() {}").unwrap();

        let pairs = discover_rs_files(td.path()).unwrap();
        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].1.ends_with("src/lib.rs"));
    }
}
