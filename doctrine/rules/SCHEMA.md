# Doctrine Rules Schema

Each rule lives in `doctrine/rules/<domain>/<rule-id>.toml` OR (initial batch) is grouped in `doctrine/rules/<domain>.toml` under `[[rule]]` arrays for compactness. Future iterations split per-file once tooling supports it.

## Triple format (NON-NEGOTIABLE per rule)

```toml
[[rule]]
id        = "rule-042"                          # globally unique kebab-case
name      = "Mobile Friendly Required"          # short human label
domain    = "primitives"                        # build | primitives | security | testing | docs | logging | perf
statement = "All Loom primitives must satisfy the MobileFriendly trait."
rationale = """
Mobile is the primary access pattern for a meaningful fraction of users.
Primitives that don't work on mobile produce sites that don't work on
mobile, regardless of other quality. This rule prevents the substrate
from accumulating desktop-only artifacts that fail half its audience.
"""
enforcement = [
  "forge phase: mobile_friendly_audit (build-time tap-target check at 390/768/1280)",
  "crawler runtime: tap-target measurement at 390px viewport on every public route",
]
applies_to    = ["all Loom primitive crates", "loom-cms-render"]
severity      = "strict"                        # strict | warn | informational
lifecycle     = "stable"                        # experimental | stable | deprecated
related_traits = ["MobileFriendly"]             # cross-ref to trait system (#166-172)
references    = ["WCAG 2.5.5 AAA", "ADR-017"]
```

## Fields

| Field | Required | Notes |
|-------|----------|-------|
| `id` | yes | Globally unique. kebab-case. `rule-NNN` numeric for stable rules; `rule-experimental-X` while drafting. |
| `name` | yes | Short label. Title-case. |
| `domain` | yes | One of: build, primitives, security, testing, docs, logging, perf, content, accessibility, sovereignty, compliance, risk, resource. |
| `statement` | yes | Precise sentence. The rule itself. |
| `rationale` | yes | Triple-quoted multi-paragraph. Why this rule exists. A rule without rationale becomes cargo-cult. |
| `enforcement` | yes | Array of human-readable enforcement mechanisms. Each entry names a Forge phase / Crawler axis / lint / schema check / test pattern. A rule without enforcement is a wish, not a rule. |
| `applies_to` | yes | Array of path globs, crate names, or artifact classes. Where the rule applies. |
| `severity` | yes | `strict` (build fail) / `warn` (build warning, doesn't gate) / `informational` (surface only). |
| `lifecycle` | yes | `experimental` (trialed, warnings only) / `stable` (binding) / `deprecated` (being removed, sunset date in `deprecated_at`). |
| `related_traits` | no | Array of trait names from the trait system (#166-172). Cross-references object orientation. |
| `references` | no | Array of external references: RFCs, ADRs, WCAG sections, prior incidents. |
| `deprecated_at` | conditional | Required if `lifecycle = "deprecated"`. ISO 8601 date when sunset begins. |
| `replaced_by` | conditional | If deprecated and a replacement exists, the replacement rule's id. |

## Parser

`doctrine-core` crate (PENDING, task #174) parses these TOML files into typed Rust structs and exposes a query API. CI verifies:

- Every rule has the full triple (statement + rationale + enforcement). Incomplete = build fail.
- `id` is globally unique across all files.
- `domain` is one of the known values.
- `severity` and `lifecycle` parse cleanly.
- `deprecated` rules have `deprecated_at` set.
- `replaced_by` references resolve to a real rule id.

## Exceptions

Per `EXCEPTIONS.md`. Inline tag: `// DOCTRINE-EXCEPTION: rule-042 — <reason>, see ADR-XXX`. Grep-able. Accumulating exceptions per rule = signal to revise the rule or extend the substrate.

## Lifecycle transitions

State changes go through PR review with explicit decision. `CHANGELOG.toml` tracks transitions per rule.

- experimental → stable: rule has been trialed; enforcement is reliable; promoted to binding.
- stable → deprecated: replacement exists or rule no longer applies; sunset window begins.
- deprecated → removed: end of sunset window; rule's TOML file deleted; tracked in CHANGELOG.

## Relationship to existing files

The pre-existing `doctrine/anti-patterns.toml` + `doctrine/principles.toml` + `doctrine/maturity-model.toml` predate this schema. They use a simpler `statement` + `why_bad` format. Migration plan:

1. Existing anti-patterns become rules in `doctrine/rules/<their-domain>.toml` with the full triple, with their `why_bad` text becoming `rationale`. Each anti-pattern's enforcement mechanism is named explicitly.
2. Existing principles stay as principles in `doctrine/principles.toml` — they are doctrine-level, not rule-level. Principles motivate rules; rules enforce them.
3. The maturity-model stays as-is — it's per-consumer adoption tracking, not a rule database.

Migration is task #173's in-progress work.
