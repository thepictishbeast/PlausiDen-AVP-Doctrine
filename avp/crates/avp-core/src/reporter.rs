//! Reporters — turn [`Finding`]s into output.
//!
//! Three concrete impls for v0.1:
//! - [`GithubActionsReporter`] — `::error::`/`::warning::`/`::notice::`
//!   annotations with `file=…,line=…` so the GH UI shows them inline.
//! - [`HumanReporter`] — colored, terminal-friendly summary (one line per
//!   finding plus a trailing aggregate).
//! - [`JsonReporter`] — JSON-lines, suitable for `--format=json` and for
//!   editor LSP-ish consumption.
//!
//! Reporters are stateful (they may keep counters, table builders, etc.)
//! and finalize at the end of a run.

use std::io::{self, Write};

use crate::finding::{Finding, Severity};

/// Reporter trait — pluggable output formatters.
pub trait Reporter: Send {
    /// Emit a single finding.
    fn emit(&mut self, finding: &Finding) -> io::Result<()>;

    /// Flush + write any closing summary.
    fn finalize(&mut self) -> io::Result<()>;
}

// ─────────────────────────────────────────────────────────────────────────
// GithubActionsReporter
// ─────────────────────────────────────────────────────────────────────────

/// GitHub Actions annotation reporter — writes `::level file=…,line=…::msg`
/// lines that the Actions runner picks up and surfaces inline on PR diffs.
#[derive(Debug)]
pub struct GithubActionsReporter<W: Write + Send> {
    out: W,
    counts: SeverityCounts,
}

impl<W: Write + Send> GithubActionsReporter<W> {
    /// Wrap any `Write` (typically `io::stdout()`).
    pub fn new(out: W) -> Self {
        Self {
            out,
            counts: SeverityCounts::default(),
        }
    }
}

impl<W: Write + Send> Reporter for GithubActionsReporter<W> {
    fn emit(&mut self, f: &Finding) -> io::Result<()> {
        self.counts.bump(f.severity);
        write!(self.out, "::{}", f.severity.gh_keyword())?;
        write!(
            self.out,
            " file={}",
            escape_property(&f.location.file.to_string_lossy())
        )?;
        if let Some(line) = f.location.line {
            write!(self.out, ",line={line}")?;
        }
        if let Some(col) = f.location.column {
            write!(self.out, ",col={col}")?;
        }
        // GH annotations cap message size; we don't truncate but we do escape.
        writeln!(
            self.out,
            "::{}",
            escape_data(&format!("[{}] {}", f.gate, f.message))
        )?;
        Ok(())
    }

    fn finalize(&mut self) -> io::Result<()> {
        writeln!(
            self.out,
            "::notice::AVP summary — error={} warning={} notice={}",
            self.counts.error, self.counts.warning, self.counts.notice
        )?;
        self.out.flush()
    }
}

/// GH Actions workflow-command property escaping.
fn escape_property(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
        .replace(':', "%3A")
        .replace(',', "%2C")
}

/// GH Actions workflow-command data escaping.
fn escape_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

// ─────────────────────────────────────────────────────────────────────────
// HumanReporter
// ─────────────────────────────────────────────────────────────────────────

/// Terminal-friendly reporter. ANSI color is opt-in via the `colored` flag
/// — callers should consult `anstream` / `NO_COLOR` and pass it through.
#[derive(Debug)]
pub struct HumanReporter<W: Write + Send> {
    out: W,
    colored: bool,
    counts: SeverityCounts,
}

impl<W: Write + Send> HumanReporter<W> {
    /// Construct with explicit color setting.
    pub fn new(out: W, colored: bool) -> Self {
        Self {
            out,
            colored,
            counts: SeverityCounts::default(),
        }
    }

    const fn paint(&self, sev: Severity) -> &'static str {
        if !self.colored {
            return sev.gh_keyword();
        }
        match sev {
            Severity::Error => "\x1b[31merror\x1b[0m",
            Severity::Warning => "\x1b[33mwarning\x1b[0m",
            Severity::Notice => "\x1b[36mnotice\x1b[0m",
        }
    }
}

impl<W: Write + Send> Reporter for HumanReporter<W> {
    fn emit(&mut self, f: &Finding) -> io::Result<()> {
        self.counts.bump(f.severity);
        let loc = match (f.location.line, f.location.column) {
            (Some(l), Some(c)) => format!("{}:{l}:{c}", f.location.file.display()),
            (Some(l), None) => format!("{}:{l}", f.location.file.display()),
            _ => f.location.file.display().to_string(),
        };
        writeln!(
            self.out,
            "[{}] {} {}: {}",
            self.paint(f.severity),
            f.gate,
            loc,
            f.message
        )
    }

    fn finalize(&mut self) -> io::Result<()> {
        writeln!(
            self.out,
            "── AVP summary: {} error / {} warning / {} notice ──",
            self.counts.error, self.counts.warning, self.counts.notice
        )?;
        self.out.flush()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// JsonReporter
// ─────────────────────────────────────────────────────────────────────────

/// JSON-lines reporter — one finding per line. Useful for editor diagnostics
/// and downstream tooling that wants structured input.
#[derive(Debug)]
pub struct JsonReporter<W: Write + Send> {
    out: W,
    counts: SeverityCounts,
}

impl<W: Write + Send> JsonReporter<W> {
    /// Wrap any `Write`.
    pub fn new(out: W) -> Self {
        Self {
            out,
            counts: SeverityCounts::default(),
        }
    }
}

impl<W: Write + Send> Reporter for JsonReporter<W> {
    fn emit(&mut self, f: &Finding) -> io::Result<()> {
        self.counts.bump(f.severity);
        let line =
            serde_json::to_string(f).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        writeln!(self.out, "{line}")
    }

    fn finalize(&mut self) -> io::Result<()> {
        let summary = serde_json::json!({
            "kind": "summary",
            "error": self.counts.error,
            "warning": self.counts.warning,
            "notice": self.counts.notice,
        });
        writeln!(self.out, "{summary}")?;
        self.out.flush()
    }
}

// ─────────────────────────────────────────────────────────────────────────
// shared
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone, Copy)]
struct SeverityCounts {
    error: u32,
    warning: u32,
    notice: u32,
}

impl SeverityCounts {
    const fn bump(&mut self, s: Severity) {
        match s {
            Severity::Error => self.error += 1,
            Severity::Warning => self.warning += 1,
            Severity::Notice => self.notice += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{finding::Location, gate::GateId};

    fn sample() -> Finding {
        Finding::error(
            GateId::BugAssumption,
            Location::line("src/lib.rs", 42),
            "missing BUG ASSUMPTION",
        )
    }

    #[test]
    fn gh_reporter_emits_annotation() {
        let mut buf = Vec::new();
        {
            let mut r = GithubActionsReporter::new(&mut buf);
            r.emit(&sample()).unwrap();
            r.finalize().unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("::error file=src/lib.rs,line=42::"));
        assert!(s.contains("[bug-assumption]"));
        assert!(s.contains("AVP summary"));
    }

    #[test]
    fn gh_reporter_escapes_special_chars() {
        let mut buf = Vec::new();
        let f = Finding::error(
            GateId::BugAssumption,
            Location::line("src/path,with:special.rs", 1),
            "msg with %percent",
        );
        {
            let mut r = GithubActionsReporter::new(&mut buf);
            r.emit(&f).unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("path%2Cwith%3Aspecial.rs"));
        assert!(s.contains("%25percent"));
    }

    #[test]
    fn human_reporter_uncolored() {
        let mut buf = Vec::new();
        {
            let mut r = HumanReporter::new(&mut buf, false);
            r.emit(&sample()).unwrap();
            r.finalize().unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("[error]"));
        assert!(s.contains("bug-assumption"));
    }

    #[test]
    fn human_reporter_colored() {
        let mut buf = Vec::new();
        {
            let mut r = HumanReporter::new(&mut buf, true);
            r.emit(&sample()).unwrap();
            r.finalize().unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[31m"));
    }

    #[test]
    fn json_reporter_emits_json_lines() {
        let mut buf = Vec::new();
        {
            let mut r = JsonReporter::new(&mut buf);
            r.emit(&sample()).unwrap();
            r.finalize().unwrap();
        }
        let s = String::from_utf8(buf).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len(), 2);
        let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed["gate"], "bug-assumption");
        assert_eq!(parsed["severity"], "error");
        let summary: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(summary["kind"], "summary");
        assert_eq!(summary["error"], 1);
    }
}
