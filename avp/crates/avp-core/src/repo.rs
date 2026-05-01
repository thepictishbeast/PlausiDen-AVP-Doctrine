//! Repository discovery — locate the repo root + classify language(s).

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, instrument};

/// A resolved repository root and the language stack(s) detected at it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RepoRoot {
    /// Absolute path to the root.
    pub path: PathBuf,
    /// Languages detected by manifest probing.
    pub languages: Vec<RepoLanguage>,
}

impl RepoRoot {
    /// Find the closest ancestor of `start` that contains a `.git` directory.
    #[instrument(level = "debug", skip_all, fields(start = %start.as_ref().display()))]
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, RepoError> {
        let mut here: PathBuf = fs::canonicalize(start.as_ref())
            .map_err(|source| RepoError::Canonicalize { source })?;
        loop {
            if here.join(".git").exists() {
                debug!(root = %here.display(), "repo root located");
                let languages = Self::detect_languages(&here);
                return Ok(Self {
                    path: here,
                    languages,
                });
            }
            if !here.pop() {
                return Err(RepoError::NoGitRoot {
                    started_at: start.as_ref().to_path_buf(),
                });
            }
        }
    }

    /// Probe top-level manifest files. We deliberately don't recurse — this
    /// is a fast detector, not a survey. A repo can be multi-language; the
    /// returned vec preserves probe order.
    fn detect_languages(root: &Path) -> Vec<RepoLanguage> {
        let mut langs = Vec::new();
        if root.join("Cargo.toml").is_file() {
            langs.push(RepoLanguage::Rust);
        }
        if root.join("package.json").is_file() {
            langs.push(RepoLanguage::TypeScript);
        }
        if root.join("pyproject.toml").is_file() || root.join("setup.py").is_file() {
            langs.push(RepoLanguage::Python);
        }
        if root.join("go.mod").is_file() {
            langs.push(RepoLanguage::Go);
        }
        langs
    }
}

/// Languages the toolchain knows how to gate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(rename_all = "lowercase")]
pub enum RepoLanguage {
    /// Cargo workspace or single crate.
    Rust,
    /// `package.json`-driven JS/TS surface.
    TypeScript,
    /// `pyproject.toml` Python project.
    Python,
    /// `go.mod` module. Deliberately no gates yet — listed so drift-check
    /// can warn about Go siblings until the supersociety stack absorbs them.
    Go,
}

impl RepoLanguage {
    /// Whether this language has gate implementations available today.
    #[must_use]
    pub const fn has_gates(self) -> bool {
        matches!(self, Self::Rust | Self::TypeScript | Self::Python)
    }
}

/// Errors raised by repo discovery.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RepoError {
    /// Path could not be canonicalized.
    #[error("canonicalize failed: {source}")]
    Canonicalize {
        /// Underlying io error.
        source: std::io::Error,
    },
    /// No `.git` directory in any ancestor.
    #[error("no .git/ found in any ancestor of {started_at:?}")]
    NoGitRoot {
        /// Path the search began at.
        started_at: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_rust() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        std::fs::write(td.path().join("Cargo.toml"), "[package]\nname=\"x\"\n").unwrap();
        let r = RepoRoot::discover(td.path()).unwrap();
        assert_eq!(r.languages, vec![RepoLanguage::Rust]);
    }

    #[test]
    fn detects_polyglot() {
        let td = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        std::fs::write(td.path().join("Cargo.toml"), "x").unwrap();
        std::fs::write(td.path().join("package.json"), "{}").unwrap();
        std::fs::write(td.path().join("pyproject.toml"), "[project]").unwrap();
        let r = RepoRoot::discover(td.path()).unwrap();
        assert_eq!(
            r.languages,
            vec![
                RepoLanguage::Rust,
                RepoLanguage::TypeScript,
                RepoLanguage::Python
            ]
        );
    }

    #[test]
    fn no_git_errors() {
        let td = tempfile::tempdir().unwrap();
        let err = RepoRoot::discover(td.path()).unwrap_err();
        assert!(matches!(err, RepoError::NoGitRoot { .. }));
    }

    #[test]
    fn ascends_to_find_git() {
        let td = tempfile::tempdir().unwrap();
        let nested = td.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(td.path().join(".git")).unwrap();
        let r = RepoRoot::discover(&nested).unwrap();
        assert_eq!(r.path, td.path().canonicalize().unwrap());
    }
}
