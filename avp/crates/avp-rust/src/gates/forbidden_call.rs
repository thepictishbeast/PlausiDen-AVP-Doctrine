//! `forbidden-call` gate — bans direct use of crash-prone calls in
//! library code.
//!
//! Banned:
//! - `.unwrap()` / `.expect(…)` — panic on `None`/`Err`.
//! - `panic!()` / `todo!()` / `unimplemented!()` — explicit panics.
//! - `dbg!(…)` / `println!(…)` / `eprintln!(…)` — uninstrumented IO.
//!
//! Allowed when the same line carries a justification comment:
//! - `// SAFETY:`
//! - `// test-only`
//! - `// AVP-PASS-` (any AVP-PASS-YYYY-MM-DD annotation)
//! - `// SHIP-DECISION:`
//! - `// FOSS-ABSORBED:`
//!
//! Scope: [`SourceClass::Library`] only. Bin/test/bench/example files are
//! free to print and panic — that's the whole point of those layouts.

use avp_core::{Context, Finding, Gate, GateId, Location};
use syn::{
    Expr, ExprMacro, ExprMethodCall, ItemMacro, Macro, StmtMacro,
    visit::{self, Visit},
};
use tracing::{debug, instrument, trace};

use crate::source::{RustSource, SourceClass};

const JUSTIFICATIONS: &[&str] = &[
    "// SAFETY:",
    "// test-only",
    "// AVP-PASS-",
    "// SHIP-DECISION:",
    "// FOSS-ABSORBED:",
];

const BANNED_METHODS: &[&str] = &["unwrap", "expect"];
const BANNED_MACROS: &[&str] = &[
    "panic",
    "todo",
    "unimplemented",
    "dbg",
    "println",
    "eprintln",
];

/// `forbidden-call` implementation.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct ForbiddenCall;

impl ForbiddenCall {
    /// Run on a pre-loaded slice of sources. Test entry point.
    #[must_use]
    pub fn run_on(self, sources: &[RustSource]) -> Vec<Finding> {
        let mut out = Vec::new();
        for src in sources {
            if src.class != SourceClass::Library {
                trace!(path = %src.rel_path.display(), class = ?src.class, "skip non-library");
                continue;
            }
            scan_source(src, &mut out);
        }
        out
    }
}

impl Gate for ForbiddenCall {
    fn id(&self) -> GateId {
        GateId::ForbiddenCall
    }

    #[instrument(level = "debug", skip_all)]
    fn run(&self, ctx: &Context<'_>) -> Vec<Finding> {
        let (sources, errors) = crate::source::collect_sources(&ctx.repo.path);
        let mut findings: Vec<Finding> = errors
            .iter()
            .map(|err| {
                Finding::warning(
                    self.id(),
                    Location::file("/"),
                    format!("source error during scan: {err}"),
                )
            })
            .collect();
        findings.extend((*self).run_on(&sources));
        debug!(found = findings.len(), "forbidden-call scan complete");
        findings
    }
}

fn scan_source(src: &RustSource, out: &mut Vec<Finding>) {
    let mut visitor = ForbiddenVisitor { hits: Vec::new() };
    visitor.visit_file(&src.ast);

    for hit in visitor.hits {
        if line_has_justification(src, hit.line) {
            trace!(line = hit.line, "justified");
            continue;
        }
        out.push(Finding::error(
            GateId::ForbiddenCall,
            Location::line(src.rel_path.clone(), hit.line),
            format!(
                "forbidden call `{}` in library code (no justification comment on line)",
                hit.kind,
            ),
        ));
    }
}

#[derive(Debug)]
struct Hit {
    line: u32,
    kind: String,
}

#[derive(Debug, Default)]
struct ForbiddenVisitor {
    hits: Vec<Hit>,
}

impl<'ast> Visit<'ast> for ForbiddenVisitor {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let name = node.method.to_string();
        if BANNED_METHODS.contains(&name.as_str()) {
            let line = u32::try_from(node.method.span().start().line).unwrap_or(1);
            self.hits.push(Hit {
                line: line.max(1),
                kind: format!(".{name}()"),
            });
        }
        visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        flag_banned_macro(&node.mac, &mut self.hits);
        visit::visit_expr_macro(self, node);
    }

    fn visit_stmt_macro(&mut self, node: &'ast StmtMacro) {
        flag_banned_macro(&node.mac, &mut self.hits);
        visit::visit_stmt_macro(self, node);
    }

    fn visit_item_macro(&mut self, node: &'ast ItemMacro) {
        flag_banned_macro(&node.mac, &mut self.hits);
        visit::visit_item_macro(self, node);
    }

    fn visit_expr(&mut self, node: &'ast Expr) {
        // Default recursion handles nested expressions including macros &
        // method calls inside other expressions.
        visit::visit_expr(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        // Skip cfg(test) modules.
        if node.attrs.iter().any(is_cfg_test_attr) {
            return;
        }
        visit::visit_item_mod(self, node);
    }
}

fn flag_banned_macro(mac: &Macro, hits: &mut Vec<Hit>) {
    let Some(ident) = mac.path.get_ident() else {
        return;
    };
    let name = ident.to_string();
    if !BANNED_MACROS.contains(&name.as_str()) {
        return;
    }
    let line = u32::try_from(ident.span().start().line).unwrap_or(1).max(1);
    hits.push(Hit {
        line,
        kind: format!("{name}!"),
    });
}

fn is_cfg_test_attr(a: &syn::Attribute) -> bool {
    a.path().is_ident("cfg")
        && a.parse_args::<syn::Meta>().is_ok_and(|m| match m {
            syn::Meta::Path(p) => p.is_ident("test"),
            _ => false,
        })
}

fn line_has_justification(src: &RustSource, line: u32) -> bool {
    let win = src.line_window(line, line);
    JUSTIFICATIONS.iter().any(|j| win.contains(j))
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::source::RustSource;

    fn lib(text: &str) -> RustSource {
        RustSource::parse(
            PathBuf::from("/tmp/lib.rs"),
            PathBuf::from("src/lib.rs"),
            text.to_string(),
        )
        .expect("parses")
    }

    fn run(text: &str) -> Vec<Finding> {
        ForbiddenCall.run_on(&[lib(text)])
    }

    #[test]
    fn unwrap_without_justification_fails() {
        let text = "pub fn x() -> i32 { Some(1).unwrap() }\n";
        let findings = run(text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unwrap"));
    }

    #[test]
    fn unwrap_with_safety_justification_clean() {
        let text = "pub fn x() -> i32 { Some(1).unwrap() } // SAFETY: literal\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn unwrap_with_test_only_justification_clean() {
        let text = "pub fn x() -> i32 { Some(1).unwrap() } // test-only fixture\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn unwrap_with_avp_pass_justification_clean() {
        let text = "pub fn x() -> i32 { Some(1).unwrap() } // AVP-PASS-2026-04-30 reviewed\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn expect_flagged() {
        let text = "pub fn x() -> i32 { Some(1).expect(\"why\") }\n";
        assert_eq!(run(text).len(), 1);
    }

    #[test]
    fn panic_macro_flagged() {
        let text = "pub fn x() { panic!(\"nope\"); }\n";
        let findings = run(text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("panic!"));
    }

    #[test]
    fn dbg_macro_flagged() {
        let text = "pub fn x() { dbg!(1); }\n";
        assert_eq!(run(text).len(), 1);
    }

    #[test]
    fn println_macro_flagged() {
        let text = "pub fn x() { println!(\"hi\"); }\n";
        assert_eq!(run(text).len(), 1);
    }

    #[test]
    fn todo_unimplemented_flagged() {
        let text = "pub fn a() { todo!() }\npub fn b() { unimplemented!() }\n";
        assert_eq!(run(text).len(), 2);
    }

    #[test]
    fn cfg_test_module_skipped() {
        let text = "#[cfg(test)]\nmod t { pub fn x() { panic!(\"ok in test\") } }\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn non_library_class_skipped() {
        let s = RustSource::parse(
            PathBuf::from("/tmp/main.rs"),
            PathBuf::from("src/main.rs"),
            "pub fn x() { println!(\"hi\"); }\n".into(),
        )
        .unwrap();
        assert!(ForbiddenCall.run_on(&[s]).is_empty());
    }

    #[test]
    fn nested_unwrap_inside_block_flagged() {
        let text = "pub fn x() { if true { Some(1).unwrap(); } }\n";
        assert_eq!(run(text).len(), 1);
    }

    #[test]
    fn multiple_violations_each_reported() {
        let text = "pub fn x() { Some(1).unwrap(); println!(\"a\"); panic!(\"b\"); }\n";
        let findings = run(text);
        assert_eq!(findings.len(), 3);
    }
}
