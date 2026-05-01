//! `unsafe-proof` gate — every `unsafe { … }` block, `unsafe fn`
//! declaration, and `unsafe impl` must be paired with a `// SAFETY:`
//! comment within the 5-line window preceding it.
//!
//! Crates with `#![forbid(unsafe_code)]` at the crate root cannot have
//! `unsafe` at all; we still scan them defensively (in case someone adds
//! `#![allow(unsafe_code)]` to a submodule), but a clean `forbid` should
//! produce no findings.
//!
//! Scope: applies to library AND binary code. `unsafe` is dangerous
//! anywhere it appears in shipped code. Test files are exempt because
//! tests routinely use `unsafe { … }` to construct edge-case fixtures.

use avp_core::{Context, Finding, Gate, GateId, Location};
use syn::{
    ExprUnsafe, ItemFn, ItemImpl, ItemTrait, Token,
    visit::{self, Visit},
};
use tracing::{debug, instrument, trace};

use crate::source::{RustSource, SourceClass};

/// `unsafe-proof` implementation.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct UnsafeProof {
    /// Lines of context to search for the `// SAFETY:` proof.
    pub window: u32,
}

impl UnsafeProof {
    /// Doctrine default window size.
    pub const DEFAULT_WINDOW: u32 = 5;

    /// The expected proof comment marker.
    pub const NEEDLE: &'static str = "// SAFETY:";

    const fn effective_window(self) -> u32 {
        if self.window == 0 {
            Self::DEFAULT_WINDOW
        } else {
            self.window
        }
    }

    /// Run on a pre-loaded slice of sources. Test entry point.
    #[must_use]
    pub fn run_on(self, sources: &[RustSource]) -> Vec<Finding> {
        let mut out = Vec::new();
        for src in sources {
            if matches!(src.class, SourceClass::Test | SourceClass::Bench) {
                trace!(path = %src.rel_path.display(), "skip test/bench");
                continue;
            }
            scan_source(self, src, &mut out);
        }
        out
    }
}

impl Gate for UnsafeProof {
    fn id(&self) -> GateId {
        GateId::UnsafeProof
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
        debug!(found = findings.len(), "unsafe-proof scan complete");
        findings
    }
}

fn scan_source(gate: UnsafeProof, src: &RustSource, out: &mut Vec<Finding>) {
    let mut v = UnsafeVisitor { hits: Vec::new() };
    v.visit_file(&src.ast);

    let window = gate.effective_window();
    for hit in v.hits {
        if window_has_safety(src, hit.line, window) {
            continue;
        }
        out.push(Finding::error(
            GateId::UnsafeProof,
            Location::line(src.rel_path.clone(), hit.line),
            format!("`{}` without `// SAFETY:` proof comment", hit.kind),
        ));
    }
}

#[derive(Debug)]
struct UnsafeHit {
    line: u32,
    kind: &'static str,
}

#[derive(Debug, Default)]
struct UnsafeVisitor {
    hits: Vec<UnsafeHit>,
}

impl<'ast> Visit<'ast> for UnsafeVisitor {
    fn visit_expr_unsafe(&mut self, node: &'ast ExprUnsafe) {
        let line = u32::try_from(node.unsafe_token.span.start().line)
            .unwrap_or(1)
            .max(1);
        self.hits.push(UnsafeHit {
            line,
            kind: "unsafe block",
        });
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if let Some(line) = unsafe_fn_line(node.sig.unsafety.as_ref()) {
            self.hits.push(UnsafeHit {
                line,
                kind: "unsafe fn",
            });
        }
        visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if let Some(line) = unsafe_fn_line(node.sig.unsafety.as_ref()) {
            self.hits.push(UnsafeHit {
                line,
                kind: "unsafe fn",
            });
        }
        visit::visit_impl_item_fn(self, node);
    }

    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if let Some(line) = unsafe_fn_line(node.sig.unsafety.as_ref()) {
            self.hits.push(UnsafeHit {
                line,
                kind: "unsafe fn",
            });
        }
        visit::visit_trait_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        if let Some(unsafe_kw) = node.unsafety.as_ref() {
            let line = u32::try_from(unsafe_kw.span.start().line)
                .unwrap_or(1)
                .max(1);
            self.hits.push(UnsafeHit {
                line,
                kind: "unsafe impl",
            });
        }
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast ItemTrait) {
        if let Some(unsafe_kw) = node.unsafety.as_ref() {
            let line = u32::try_from(unsafe_kw.span.start().line)
                .unwrap_or(1)
                .max(1);
            self.hits.push(UnsafeHit {
                line,
                kind: "unsafe trait",
            });
        }
        visit::visit_item_trait(self, node);
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if node.attrs.iter().any(is_cfg_test_attr) {
            return;
        }
        visit::visit_item_mod(self, node);
    }
}

fn unsafe_fn_line(unsafe_kw: Option<&Token![unsafe]>) -> Option<u32> {
    let kw = unsafe_kw?;
    let line = u32::try_from(kw.span.start().line).unwrap_or(1).max(1);
    Some(line)
}

fn is_cfg_test_attr(a: &syn::Attribute) -> bool {
    a.path().is_ident("cfg")
        && a.parse_args::<syn::Meta>().is_ok_and(|m| match m {
            syn::Meta::Path(p) => p.is_ident("test"),
            _ => false,
        })
}

fn window_has_safety(src: &RustSource, line: u32, window: u32) -> bool {
    let start = line.saturating_sub(window).max(1);
    src.line_window(start, line).contains(UnsafeProof::NEEDLE)
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
        UnsafeProof::default().run_on(&[lib(text)])
    }

    #[test]
    fn unsafe_block_without_proof_fails() {
        let text = "pub fn x() { unsafe { } }\n";
        let findings = run(text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("unsafe block"));
    }

    #[test]
    fn unsafe_block_with_safety_proof_clean() {
        let text = "pub fn x() {\n    // SAFETY: trivially sound\n    unsafe { }\n}\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn unsafe_fn_without_proof_fails() {
        let text = "pub unsafe fn x() {}\n";
        assert_eq!(run(text).len(), 1);
        assert!(run(text)[0].message.contains("unsafe fn"));
    }

    #[test]
    fn unsafe_fn_with_proof_clean() {
        let text = "// SAFETY: caller upholds invariants.\npub unsafe fn x() {}\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn unsafe_impl_without_proof_fails() {
        let text = "pub struct S; unsafe impl Send for S {}\n";
        assert_eq!(run(text).len(), 1);
        assert!(run(text)[0].message.contains("unsafe impl"));
    }

    #[test]
    fn unsafe_trait_without_proof_fails() {
        let text = "pub unsafe trait T {}\n";
        assert_eq!(run(text).len(), 1);
        assert!(run(text)[0].message.contains("unsafe trait"));
    }

    #[test]
    fn nested_blocks_each_checked() {
        let text = "
            pub fn x() {
                unsafe { /* one */ }
                // SAFETY: ok
                unsafe { /* two */ }
            }
        ";
        let findings = run(text);
        assert_eq!(findings.len(), 1, "only first should fail");
    }

    #[test]
    fn marker_outside_window_fails() {
        let mut text = String::from("// SAFETY: stale\n");
        for _ in 0..6 {
            text.push('\n');
        }
        text.push_str("pub fn x() { unsafe { } }\n");
        // marker now ≥6 lines above unsafe; default window=5 → fail
        assert_eq!(run(&text).len(), 1);
    }

    #[test]
    fn cfg_test_module_skipped() {
        let text = "#[cfg(test)]\nmod t { pub fn x() { unsafe { } } }\n";
        assert!(run(text).is_empty());
    }

    #[test]
    fn test_class_skipped() {
        let s = RustSource::parse(
            PathBuf::from("/tmp/it.rs"),
            PathBuf::from("tests/it.rs"),
            "fn x() { unsafe { } }\n".into(),
        )
        .unwrap();
        assert!(UnsafeProof::default().run_on(&[s]).is_empty());
    }

    #[test]
    fn unsafe_in_bin_still_checked() {
        let s = RustSource::parse(
            PathBuf::from("/tmp/main.rs"),
            PathBuf::from("src/main.rs"),
            "pub fn x() { unsafe { } }\n".into(),
        )
        .unwrap();
        assert_eq!(UnsafeProof::default().run_on(&[s]).len(), 1);
    }
}
