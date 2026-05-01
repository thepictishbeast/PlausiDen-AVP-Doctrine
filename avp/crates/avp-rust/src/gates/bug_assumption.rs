//! `bug-assumption` gate — every `pub fn` (and `pub` method on inherent
//! impls) must have a `BUG ASSUMPTION:` comment within the 20-line window
//! preceding its signature.
//!
//! Why AST + comment grep instead of a regex:
//! - Regex over raw source false-positives on commented-out code, doc
//!   examples, and macro-expanded `pub fn`s.
//! - `syn` accurately identifies real pub fns (including impl methods).
//! - But `syn` does *not* preserve comments, so we still scan the source
//!   text in a window above each detected fn.
//!
//! Scope:
//! - Files classified [`SourceClass::Library`] only — bin/test/bench/example
//!   code is exempt.
//! - Public-by-restriction (`pub(crate)`, `pub(super)`, `pub(in path)`) is
//!   *not* covered today — only fully-public surface that other crates can
//!   depend on. Trait impl methods inherit the trait's contract, so they
//!   are covered only when the surrounding `impl` is `pub` (we treat any
//!   inherent `impl Type { pub fn ... }` and any `impl Trait for Type` with
//!   `Type` being publicly reachable as in-scope; for v0.1 we use a simple
//!   conservative heuristic: include any `pub fn` body inside an `impl`
//!   block whose self-type isn't an obvious test-only newtype).
//!
//! Ratcheting: yes (gate id [`GateId::BugAssumption`]). Per-crate and
//! per-file overrides honored via [`Context`]'s ratchet machinery (wired
//! by the CLI at run-time).

use avp_core::{Context, Finding, Gate, GateId, Location};
use proc_macro2::Span;
use syn::{ImplItem, ImplItemFn, Item, ItemFn, ItemImpl, Visibility};
use tracing::{debug, instrument, trace};

use crate::source::{RustSource, SourceClass};

/// `bug-assumption` implementation.
#[derive(Debug, Default, Clone, Copy)]
#[non_exhaustive]
pub struct BugAssumption {
    /// How many lines above a `pub fn` to search for the comment.
    pub window: u32,
}

impl BugAssumption {
    /// Doctrine default window size.
    pub const DEFAULT_WINDOW: u32 = 20;

    /// The literal comment marker.
    pub const NEEDLE: &'static str = "BUG ASSUMPTION:";

    /// Construct with a custom window size.
    #[must_use]
    pub const fn with_window(window: u32) -> Self {
        Self { window }
    }

    const fn effective_window(self) -> u32 {
        if self.window == 0 {
            Self::DEFAULT_WINDOW
        } else {
            self.window
        }
    }

    /// Run on a pre-loaded slice of sources. Test-friendly entry point.
    #[must_use]
    pub fn run_on(self, sources: &[RustSource]) -> Vec<Finding> {
        let mut out = Vec::new();
        for src in sources {
            if src.class != SourceClass::Library {
                trace!(path = %src.rel_path.display(), class = ?src.class, "skip non-library file");
                continue;
            }
            scan_source(self, src, &mut out);
        }
        out
    }
}

impl Gate for BugAssumption {
    fn id(&self) -> GateId {
        GateId::BugAssumption
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
        debug!(found = findings.len(), "bug-assumption scan complete");
        findings
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Internals
// ─────────────────────────────────────────────────────────────────────────

fn scan_source(gate: BugAssumption, src: &RustSource, out: &mut Vec<Finding>) {
    let mut collector = PubFnCollector::default();
    walk(&src.ast, &mut collector);

    let window = gate.effective_window();
    for (line, name) in &collector.entries {
        if line_window_has_needle(src, *line, window) {
            continue;
        }
        out.push(Finding::error(
            GateId::BugAssumption,
            Location::line(src.rel_path.clone(), *line),
            format!("missing BUG ASSUMPTION: comment for public fn `{name}`"),
        ));
    }
}

/// Whether the marker is attached to *this* fn — i.e., present in the
/// comment block immediately above the fn signature, with only blank
/// lines and attribute lines (`#[…]`) in between.
///
/// This is stricter (and more accurate) than a flat 20-line grep: a
/// marker for a *different* fn 5 lines above doesn't satisfy the gate
/// for the current fn.
fn line_window_has_needle(src: &RustSource, fn_line: u32, max_search: u32) -> bool {
    if fn_line <= 1 {
        return false;
    }
    let mut current = fn_line - 1;
    let mut steps = 0u32;
    loop {
        let line = src.line_window(current, current);
        let trimmed = line.trim();
        if trimmed.is_empty() {
            // blank line — allowed within attached block
        } else if trimmed.starts_with("//") {
            // any comment style: line, doc, or inner doc.
            if trimmed.contains(BugAssumption::NEEDLE) {
                return true;
            }
        } else if trimmed.starts_with("#[") || trimmed.starts_with("#![") {
            // attribute lines pass through
        } else {
            // any other code halts the climb
            return false;
        }
        if current == 1 || steps + 1 >= max_search {
            return false;
        }
        current -= 1;
        steps += 1;
    }
}

#[derive(Default, Debug)]
struct PubFnCollector {
    /// (1-based line of the `fn` keyword, fn name).
    entries: Vec<(u32, String)>,
}

/// Manual AST walk — we avoid `syn::visit::Visit` here so we can prune
/// `#[cfg(test)]` modules and inherent-impl bodies that are clearly tests.
fn walk(file: &syn::File, out: &mut PubFnCollector) {
    for item in &file.items {
        walk_item(item, out);
    }
}

fn walk_item(item: &Item, out: &mut PubFnCollector) {
    match item {
        Item::Fn(f) if is_pub(&f.vis) && !is_cfg_test(&f.attrs) => {
            out.entries.push(pub_fn_record(f));
        }
        Item::Impl(im) if !is_cfg_test_impl(im) => walk_impl(im, out),
        Item::Mod(m) if !is_cfg_test(&m.attrs) => {
            if let Some((_, items)) = &m.content {
                for inner in items {
                    walk_item(inner, out);
                }
            }
        }
        _ => {}
    }
}

fn walk_impl(im: &ItemImpl, out: &mut PubFnCollector) {
    // Trait impls *for* a type don't add fresh public surface beyond what
    // the trait already required — skip them in v0.1 to avoid double-counting
    // and to keep ratchet semantics simple.
    if im.trait_.is_some() {
        return;
    }
    for item in &im.items {
        if let ImplItem::Fn(f) = item
            && is_pub(&f.vis)
            && !is_cfg_test(&f.attrs)
        {
            out.entries.push(pub_impl_fn_record(f));
        }
    }
}

fn pub_fn_record(f: &ItemFn) -> (u32, String) {
    (fn_line(f.sig.fn_token.span), f.sig.ident.to_string())
}

fn pub_impl_fn_record(f: &ImplItemFn) -> (u32, String) {
    (fn_line(f.sig.fn_token.span), f.sig.ident.to_string())
}

fn fn_line(span: Span) -> u32 {
    let start = span.start();
    u32::try_from(start.line.max(1)).unwrap_or(1)
}

const fn is_pub(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg")
            && a.parse_args::<syn::Meta>().is_ok_and(|meta| {
                if let syn::Meta::Path(p) = meta {
                    p.is_ident("test")
                } else {
                    false
                }
            })
    })
}

fn is_cfg_test_impl(im: &ItemImpl) -> bool {
    is_cfg_test(&im.attrs)
}

// Marker comments on lines outside a fn's attached block are ignored —
// see `line_window_has_needle` for the attached-block walker.

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

    fn run_one(text: &str) -> Vec<Finding> {
        BugAssumption::default().run_on(&[lib(text)])
    }

    #[test]
    fn pub_fn_with_marker_is_clean() {
        let text = "
            // BUG ASSUMPTION: x is non-empty.
            pub fn x() {}
        ";
        assert!(run_one(text).is_empty());
    }

    #[test]
    fn pub_fn_without_marker_fails() {
        let text = "pub fn x() {}\n";
        let findings = run_one(text);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].gate, GateId::BugAssumption);
        assert!(findings[0].message.contains('x'));
    }

    #[test]
    fn private_fn_ignored() {
        let text = "fn x() {}\n";
        assert!(run_one(text).is_empty());
    }

    #[test]
    fn pub_crate_ignored_today() {
        // pub(crate) is intentionally not in v0.1 scope.
        let text = "pub(crate) fn x() {}\n";
        assert!(run_one(text).is_empty());
    }

    #[test]
    fn marker_outside_window_fails() {
        // Marker 21 lines above a fn; window default is 20.
        let mut text = String::from("// BUG ASSUMPTION: stale\n");
        for _ in 0..21 {
            text.push('\n');
        }
        text.push_str("pub fn x() {}\n");
        assert_eq!(run_one(&text).len(), 1);
    }

    #[test]
    fn marker_inside_window_clean() {
        let mut text = String::new();
        for _ in 0..10 {
            text.push('\n');
        }
        text.push_str("// BUG ASSUMPTION: ok\n");
        for _ in 0..5 {
            text.push('\n');
        }
        text.push_str("pub fn x() {}\n");
        assert!(run_one(&text).is_empty());
    }

    #[test]
    fn pub_method_in_inherent_impl_covered() {
        let text = "
            pub struct S;
            impl S {
                pub fn x(&self) {}
            }
        ";
        let findings = run_one(text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains('x'));
    }

    #[test]
    fn pub_method_in_trait_impl_skipped() {
        let text = "
            pub struct S;
            pub trait T { fn x(&self); }
            impl T for S {
                fn x(&self) {}
            }
        ";
        // trait impl methods aren't double-counted in v0.1
        assert!(run_one(text).is_empty());
    }

    #[test]
    fn cfg_test_module_skipped() {
        let text = "
            #[cfg(test)]
            mod tests {
                pub fn helper() {}
            }
        ";
        assert!(run_one(text).is_empty());
    }

    #[test]
    fn non_library_class_skipped() {
        // Same source flagged as a binary entrypoint — gate must skip it.
        let s = RustSource::parse(
            PathBuf::from("/tmp/main.rs"),
            PathBuf::from("src/main.rs"),
            "pub fn x() {}\n".into(),
        )
        .unwrap();
        let findings = BugAssumption::default().run_on(&[s]);
        assert!(findings.is_empty());
    }

    #[test]
    fn multiple_pub_fns_each_independently_checked() {
        let text = "
            // BUG ASSUMPTION: a is fine.
            pub fn a() {}

            pub fn b() {}

            // BUG ASSUMPTION: c is fine.
            pub fn c() {}
        ";
        let findings = run_one(text);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains('b'));
    }

    #[test]
    fn custom_window_honored() {
        let mut text = String::from("// BUG ASSUMPTION: distant\n");
        for _ in 0..50 {
            text.push('\n');
        }
        text.push_str("pub fn x() {}\n");

        // Default window=20: stale marker out of range → fails.
        assert_eq!(BugAssumption::default().run_on(&[lib(&text)]).len(), 1);
        // Custom window=100: in range → clean.
        let g = BugAssumption::with_window(100);
        assert_eq!(g.run_on(&[lib(&text)]).len(), 0);
    }
}
