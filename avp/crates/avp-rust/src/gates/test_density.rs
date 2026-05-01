//! `test-density-aggregate` gate — workspace-level `(tests / public_fns)`
//! ratio must meet a configured floor (doctrine default: 4.0).
//!
//! Counting rules:
//! - **Tests** (numerator): any function carrying one of
//!   `#[test]`, `#[tokio::test]`, `#[async_std::test]`, `#[rstest]`,
//!   `#[proptest]`, `#[quickcheck]`, `#[test_log::test]`. Plus each
//!   `proptest! { … }` block counts as a single test (we don't try to
//!   count the inner test fns).
//! - **Public fns** (denominator): every `pub fn` and `pub` method on an
//!   inherent `impl` inside [`SourceClass::Library`] files, mirroring
//!   the BugAssumption gate's scope. Matching scopes is important: a
//!   denominator that includes test-only fns would inflate naturally.
//!
//! A workspace with zero public fns is **vacuously clean** (ratio
//! undefined → no finding emitted).

use avp_core::{Context, Finding, Gate, GateId, Location};
use syn::{
    Attribute, ImplItem, ImplItemFn, ItemFn, ItemImpl, Visibility,
    visit::{self, Visit},
};
use tracing::{debug, instrument};

use crate::source::RustSource;

const TEST_ATTRS: &[&str] = &[
    "test",
    "tokio::test",
    "async_std::test",
    "rstest",
    "proptest",
    "quickcheck",
    "test_log::test",
];

/// `test-density-aggregate` implementation.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct TestDensityAggregate {
    /// Minimum acceptable `tests / public_fns` ratio. Doctrine default 4.0.
    pub min_ratio: f64,
}

impl Default for TestDensityAggregate {
    fn default() -> Self {
        Self {
            min_ratio: Self::DEFAULT_MIN_RATIO,
        }
    }
}

impl TestDensityAggregate {
    /// Doctrine default ratio.
    pub const DEFAULT_MIN_RATIO: f64 = 4.0;

    /// Construct with a custom floor.
    #[must_use]
    pub const fn with_min(min_ratio: f64) -> Self {
        Self { min_ratio }
    }

    /// Run on pre-loaded sources. Returns at most one finding.
    #[must_use]
    pub fn run_on(self, sources: &[RustSource]) -> Vec<Finding> {
        let counts = count(sources);
        debug!(?counts, min = self.min_ratio, "test-density counts");
        if counts.public_fns == 0 {
            return Vec::new();
        }
        let ratio = f64::from(counts.tests) / f64::from(counts.public_fns);
        if ratio >= self.min_ratio {
            return Vec::new();
        }
        vec![Finding::error(
            GateId::TestDensityAggregate,
            Location::file("/"),
            format!(
                "test-density {:.3} below floor {:.3} (tests={}, public_fns={})",
                ratio, self.min_ratio, counts.tests, counts.public_fns,
            ),
        )]
    }
}

impl Gate for TestDensityAggregate {
    fn id(&self) -> GateId {
        GateId::TestDensityAggregate
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
        findings
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Counts {
    tests: u32,
    public_fns: u32,
}

fn count(sources: &[RustSource]) -> Counts {
    let mut counts = Counts::default();
    for src in sources {
        let mut v = CountVisitor {
            counts: Counts::default(),
            in_lib: src.class.is_library(),
        };
        v.visit_file(&src.ast);
        counts.tests = counts.tests.saturating_add(v.counts.tests);
        counts.public_fns = counts.public_fns.saturating_add(v.counts.public_fns);
        // proptest!{} blocks
        for line in src.text.lines() {
            let l = line.trim_start();
            if l.starts_with("proptest!") || l.starts_with("proptest! ") {
                counts.tests = counts.tests.saturating_add(1);
            }
        }
    }
    counts
}

#[derive(Debug, Default)]
struct CountVisitor {
    counts: Counts,
    in_lib: bool,
}

impl<'ast> Visit<'ast> for CountVisitor {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        // cfg(test) — count tests inside but not public fns
        let cfg_test = node.attrs.iter().any(is_cfg_test_attr);
        let prev_in_lib = self.in_lib;
        if cfg_test {
            self.in_lib = false;
        }
        visit::visit_item_mod(self, node);
        self.in_lib = prev_in_lib;
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if has_test_attr(&node.attrs) {
            self.counts.tests = self.counts.tests.saturating_add(1);
        } else if self.in_lib && matches!(node.vis, Visibility::Public(_)) {
            self.counts.public_fns = self.counts.public_fns.saturating_add(1);
        }
        // don't descend into bodies for nested fn defs (rare but safe)
        visit::visit_item_fn(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        if node.trait_.is_some() {
            // Skip trait impls — count of pub_impl methods would double-count
            // contracts already declared on the trait.
            visit::visit_item_impl(self, node);
            return;
        }
        for item in &node.items {
            if let ImplItem::Fn(f) = item {
                self.visit_impl_item_fn(f);
            }
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if has_test_attr(&node.attrs) {
            self.counts.tests = self.counts.tests.saturating_add(1);
        } else if self.in_lib && matches!(node.vis, Visibility::Public(_)) {
            self.counts.public_fns = self.counts.public_fns.saturating_add(1);
        }
        // intentionally don't recurse — we don't care about fn bodies
    }
}

fn has_test_attr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        let path = a.path();
        if let Some(ident) = path.get_ident() {
            return TEST_ATTRS.iter().any(|t| ident == t);
        }
        // multi-segment, e.g. tokio::test
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        let joined = segs.join("::");
        TEST_ATTRS.contains(&joined.as_str())
    })
}

fn is_cfg_test_attr(a: &Attribute) -> bool {
    a.path().is_ident("cfg")
        && a.parse_args::<syn::Meta>().is_ok_and(|m| match m {
            syn::Meta::Path(p) => p.is_ident("test"),
            _ => false,
        })
}

// ─────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::source::RustSource;

    fn src(rel: &str, text: &str) -> RustSource {
        RustSource::parse(
            PathBuf::from(format!("/tmp/{rel}")),
            PathBuf::from(rel),
            text.to_string(),
        )
        .expect("parses")
    }

    fn run(min: f64, sources: &[RustSource]) -> Vec<Finding> {
        TestDensityAggregate::with_min(min).run_on(sources)
    }

    #[test]
    fn empty_workspace_is_vacuously_clean() {
        assert!(TestDensityAggregate::default().run_on(&[]).is_empty());
    }

    #[test]
    fn workspace_with_zero_pub_fns_is_clean() {
        let s = src("src/lib.rs", "fn private() {}\n");
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn meets_floor_clean() {
        let s = src(
            "src/lib.rs",
            "
            pub fn x() {}

            #[cfg(test)]
            mod tests {
                #[test] fn t1() {}
                #[test] fn t2() {}
                #[test] fn t3() {}
                #[test] fn t4() {}
            }
            ",
        );
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn below_floor_fails() {
        let s = src(
            "src/lib.rs",
            "
            pub fn a() {}
            pub fn b() {}

            #[cfg(test)]
            mod tests {
                #[test] fn t1() {}
            }
            ",
        );
        let findings = run(4.0, &[s]);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("below"));
    }

    #[test]
    fn tokio_test_counts() {
        let s = src(
            "src/lib.rs",
            "
            pub fn a() {}
            #[cfg(test)]
            mod tests {
                #[tokio::test] async fn t1() {}
                #[tokio::test] async fn t2() {}
                #[tokio::test] async fn t3() {}
                #[tokio::test] async fn t4() {}
            }
            ",
        );
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn impl_methods_counted() {
        let s = src(
            "src/lib.rs",
            "
            pub struct S;
            impl S {
                pub fn a(&self) {}
                pub fn b(&self) {}
            }
            #[cfg(test)]
            mod tests {
                #[test] fn t1() {}
                #[test] fn t2() {}
                #[test] fn t3() {}
                #[test] fn t4() {}
                #[test] fn t5() {}
                #[test] fn t6() {}
                #[test] fn t7() {}
                #[test] fn t8() {}
            }
            ",
        );
        // 2 pub fns, 8 tests → ratio 4.0 → at floor → clean
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn trait_impl_methods_not_counted() {
        let s = src(
            "src/lib.rs",
            "
            pub trait T { fn x(&self); }
            pub struct S;
            impl T for S { fn x(&self) {} }
            #[cfg(test)]
            mod tests {
                #[test] fn t1() {}
                #[test] fn t2() {}
                #[test] fn t3() {}
                #[test] fn t4() {}
            }
            ",
        );
        // T::x not counted; only the `pub trait T { fn x }` body itself doesn't
        // declare a pub fn item. So pub_fns = 0 → vacuous clean.
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn proptest_macro_counted() {
        let s = src(
            "src/lib.rs",
            "
            pub fn x() {}
            #[cfg(test)]
            mod tests {
                use proptest::prelude::*;
                proptest! {
                    fn one(_v in 0u32..) {}
                }
                proptest! {
                    fn two(_v in 0u32..) {}
                }
                proptest! {
                    fn three(_v in 0u32..) {}
                }
                proptest! {
                    fn four(_v in 0u32..) {}
                }
            }
            ",
        );
        assert!(run(4.0, &[s]).is_empty());
    }

    #[test]
    fn ratio_message_includes_numbers() {
        let s = src(
            "src/lib.rs",
            "
            pub fn a() {}
            #[cfg(test)] mod t { #[test] fn t1() {} }
            ",
        );
        let findings = run(4.0, &[s]);
        assert_eq!(findings.len(), 1);
        let m = &findings[0].message;
        assert!(m.contains("tests=1"));
        assert!(m.contains("public_fns=1"));
    }

    #[test]
    fn non_library_pub_fns_excluded_from_denominator() {
        let lib = src("src/lib.rs", "pub fn one() {}\n");
        let bin = src(
            "src/main.rs",
            "
            pub fn extra1() {}
            pub fn extra2() {}
            pub fn extra3() {}
            ",
        );
        let test_file = src(
            "tests/it.rs",
            "
            #[test] fn t1() {}
            #[test] fn t2() {}
            #[test] fn t3() {}
            #[test] fn t4() {}
            ",
        );
        // 1 lib pub fn, 4 tests = 4.0 ratio → clean
        assert!(run(4.0, &[lib, bin, test_file]).is_empty());
    }
}
