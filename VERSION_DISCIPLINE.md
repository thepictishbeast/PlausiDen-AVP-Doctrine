# VERSION_DISCIPLINE.md

Versioning + backward-compat discipline for every PlausiDen substrate artifact. Every artifact carries a version tuple; every change classifies into one of four categories; pin-by-default + renderability guarantee make platform upgrades safe even across breaking changes.

> Per `[[backward-compat-version-discipline]]` doctrine: migration-friendly, NOT future-proof. We don't try to predict future requirements — we commit to making *every change* safely migratable.

> Authored to close `#137 [backcompat-v1]`. Gates the rest of the backcompat arc: #138 (semver lint), #139 (migration registry), #140 (CI compat matrix), #141 (operator UI), #142 (deprecation tooling), #143 (phase-level version ranges).

> Companion to `SUBSTRATE_DISCIPLINE.md`, `DETERMINISTIC_FIRST.md`, `TRAIT_DAG.md`, `N_ORIENTATION_SUBSTRATE.md`, `CAPABILITY_AI_POSTURE.md`.

---

## The rule

**Every platform artifact (configuration, schema, manifest, output, report, doctrine rule, trait, primitive, theme, CMS shape, MCP tool) carries a version tuple. Every change classifies into one of four categories. Pinning declares which version an artifact targets. The substrate refuses to consume an artifact whose declared version is incompatible.**

Three corollaries:

1. **Pin by default.** Operators pin the substrate version they target. Upgrades are explicit, not silent.
2. **Renderability guarantee.** Every artifact ever published can be rendered by *some* substrate version (current or specified-historical). No artifact becomes unrenderable.
3. **Signed migration registry.** Every cross-version migration is signed (Ed25519 + ML-DSA dual). Migration provenance is verifiable.

---

## The version tuple

Every artifact carries a `version` field with this shape:

```jsonc
{
  "version": {
    "spec":    "1.0.0",          // semver of the artifact's own schema
    "engine":  ">=0.4.0,<0.6.0", // substrate-version range the artifact targets
    "schema":  "v1",             // optional discriminant for major schema variants
    "minted":  "<RFC 3339>",     // ISO 8601 timestamp of artifact creation
    "minted_by": "<entity-id>"   // optional: tenant/operator/CI run id
  }
}
```

- `spec` follows SemVer 2.0.0 (major.minor.patch); applies to the artifact's own evolution.
- `engine` is a version-range expression against the substrate (Forge / Loom / Crawler / etc.); the substrate refuses to consume the artifact if its self-version doesn't satisfy this range.
- `schema` carries the major-schema-variant when SemVer alone isn't sufficient (rare; used when *parser identity* needs to switch entirely, e.g. v1 JSON vs v2 binary).
- `minted` is RFC 3339 (per rule pending — to be filed) for audit + sort.
- `minted_by` is optional but recommended for multi-tenant deployments.

---

## The 4 change categories

Every change to an artifact's schema (or to the substrate's reading of it) classifies into one category. The category determines the migration posture.

### Category 1 — Invisible

Internal refactors that don't change observable behavior or schema. Operators never see them. Examples:

- Renaming a private struct field.
- Restructuring an internal module.
- Rewording a `tracing::info!` message.
- Adding a new internal cache.

**Required action:** none.
**SemVer:** patch bump (or no bump if pre-1.0).

### Category 2 — Additive

New optional field, new variant in an open enum that maintains exhaustiveness through a wildcard arm, new subcommand. The artifact remains valid for both old + new substrate.

- Adding an optional `align: Option<HorizontalAlign>` to a primitive.
- Adding `forge orient` subcommand (existing scripts unaffected).
- Adding a new doctrine rule to the database.
- Adding a new MCP tool to `mcp/manifest.json`.

**Required action:** none for consumers; new substrate gains capability.
**SemVer:** minor bump.

### Category 3 — Auto-migration

Schema changes that require transformation but the transformation is mechanical and complete. The substrate carries the migration code; reading an old artifact emits the new shape transparently.

- Renaming a field with a deterministic rule (e.g., `bg_color` → `background_color`).
- Splitting a string into a structured value with a clear parser.
- Promoting a flat list into a nested list with a single default group.

**Required action:** the substrate runs the migration on read; the on-disk artifact may be rewritten on next save (optional, operator-controlled).
**SemVer:** minor bump if backward-readable, major bump if forward-only.
**Registry entry:** required (see § Migration registry).

### Category 4 — Operator action

Changes that cannot be mechanically migrated. The operator must intervene (review, decide, edit). The substrate refuses to read the old artifact and emits a diagnostic that names the migration playbook.

- Removing a feature that operators depend on.
- Changing semantics of an existing field in a way no automatic mapping captures.
- Replacing a deterministic mapping with one requiring policy choice.

**Required action:** operator follows the playbook; the diagnostic identifies the field + playbook URL.
**SemVer:** major bump.
**Registry entry:** required, with playbook URL.

---

## Per-artifact-class versioning

Each substrate artifact class declares which categories it supports + how it carries its version tuple.

| Artifact class | Path / location | Version tuple location | Categories used |
|----------------|-----------------|------------------------|-----------------|
| Forge `forge.toml` | `<site>/forge.toml` | `[platform] version = "..."` | 1, 2, 3, 4 |
| Forge `backends.toml` | `<site>/backends.toml` | `[meta] schema_version = "..."` | 1, 2, 3 |
| Forge build report | `reports/build-<id>.json` | top-level `output_schema_version` | 1, 2 |
| Forge doctrine rule | `doctrine/rules/<domain>.toml` | `[meta] version = "..."` per rule | 1, 2, 3 |
| Forge phase output | embedded in build report | inherits report version | 1, 2 |
| Loom primitive | `loom-cms-render/src/lib.rs` (variants) | manifest entry `version` | 1, 2, 3, 4 |
| Loom theme | `loom-tokens/themes/*.toml` | `[meta] version = "..."` | 1, 2, 3 |
| Loom skin.css | emitted at build time | `--/* schema-version: ... */` header | 1, 2 |
| CMS page | `cms/*.json` | `$schema` URI + `"version"` field | 1, 2, 3, 4 |
| CMS schema | `cms-schema.json` | `$id` URI carries version | 2, 3, 4 |
| Manifest projection | `manifest.toml` / `manifest.json` | `[meta] manifest_version = "..."` | 1, 2, 3 |
| Trait declaration | manifest-projection | per-trait `version` field | 1, 2, 3 |
| Orientation declaration | manifest-projection | per-orientation `version` field | 1, 2, 3 |
| MCP tool def | `mcp/tools/*.json` | `inputSchema` `$schema` URI | 2, 3 |
| MCP manifest | `mcp/manifest.json` | top-level `version` | 2, 3 |
| Audit chain entry | `observability-core` types | embedded `chain_schema_version` | 1, 2 only (never 3/4 — chain immutable) |
| Capability request | GitHub Issue template | template `version` in YAML | 2 only |
| Bypass register entry | `bypass-register.toml` | `[meta] register_version = "..."` | 1, 2, 3 |

**Audit chain immutability**: the audit chain is the one artifact class that NEVER takes Category 3 or 4 changes. Once an entry is committed, its schema is frozen forever; new entries use new schemas, old entries remain readable indefinitely. Per `[[manifest-layer-is-the-keystone]]` + Merkle-chain integrity.

---

## Pin-by-default

Every Forge build declares which substrate version it targets:

```toml
# <site>/forge.toml
[platform]
forge_version  = "1.4.2"           # exact pin (preferred)
loom_version   = ">=0.7.0,<0.9.0"  # range (acceptable when no breaking change in range)
crawler_version = "0.12.*"          # wildcard minor (acceptable when API stable)
```

The substrate refuses to build if any pinned-component is unavailable, mis-versioned, or has incompatible `engine` ranges with the consumed artifacts. Upgrade explicitly — never silently.

**Operator override:**

```bash
forge build --engine-override-forge=1.5.0   # operator accepts the risk
```

The override is logged in the build report (`platform_override` field) so the deviation is traceable in audit.

---

## Renderability guarantee

> **Every published artifact remains renderable by some substrate version.**

Concretely:

1. The substrate maintains a `support_window` of declared versions (e.g., "1.0 through 1.6 are renderable in 1.6"). The window is published in each release's `RELEASE.md`.
2. Artifacts pinned to a version *inside* the window render natively.
3. Artifacts pinned to a version *outside* the window can be migrated via the signed migration registry (see below) — operator runs `forge migrate <artifact> --from <ver> --to <ver>`.
4. Long-archived artifacts (pinned to versions older than the support window) require either a multi-step migration through registry-recorded intermediate versions OR an explicit `forge migrate --legacy` flag.

The renderability guarantee means: **a published doctrine rule from 2026 must still be queryable from a 2036 substrate.** The substrate carries the migration chain for every supported transition.

---

## Migration registry

Every Category 3 or 4 change adds an entry to a signed migration registry:

```toml
# PlausiDen-AVP-Doctrine/migrations/registry.toml
[[migration]]
id            = "loom-primitive-hero-v1-v2-2026-08"
from_version  = "1.x"
to_version    = "2.0.0"
artifact_class = "Loom.Primitive.Hero"
category      = "auto-migration"        # 3 or 4
description   = "Split `background_color` into structured `background: { kind: ColorBackground, color: ... }`."
playbook      = "docs/migrations/hero-v1-v2.md"
implementation = "loom-migrations/src/hero_v1_v2.rs"
test_corpus   = "loom-migrations/fixtures/hero/"
signatures    = ["ed25519:<base64url>", "ml-dsa:<base64url>"]
signed_by     = "paul"
signed_at     = "2026-08-15T12:00:00Z"
```

**Mandatory fields**: id + from_version + to_version + artifact_class + category + signatures. Without signatures the registry rejects the entry.

**Verification**: `forge migrate verify` walks the entire registry, checks every signature, asserts every implementation exists, runs test corpora. CI runs this on every PR per task #140.

---

## Lifecycle of a deprecation (Category 4 prep)

Category 4 (operator-action) changes go through a multi-step lifecycle:

```
   announce              flag                require
   ────────              ────                ───────
                  ┌─→ flagged in new release ─→ refused in next major
   PR opens an   →│  with `deprecated = true`   with `removed = true`
   ADR + capability  emits tracing::warn       reads emit fatal diagnostic
   request           old artifacts still valid pointing at migration playbook
```

Per `[[backward-compat-version-discipline]]`:
- **Announce:** ADR + capability-request before any code change.
- **Flag:** ship the new schema as additive; old continues to work; deprecation warning emitted.
- **Require:** in the next major release, the old schema is refused; the registry entry's migration code applies.

The flag → require window is at least one minor release cycle long. Operators see the deprecation warning before they're forced to act.

---

## Implementation arc

This design (#137, this doc) gates the rest of the backcompat arc:

| Task | Deliverable |
|------|-------------|
| **#137** (this) | Design doc + 4-category taxonomy + version-tuple schema + registry shape + lifecycle |
| **#138** [backcompat-v2] | Semver enforcement lint phase (Forge phase that reads every artifact's version, checks classification correctness, refuses unclassified breaking changes) |
| **#139** [backcompat-v3] | Migration registry crate (`migration-core`) + typed framework (Migration trait + auto-migration runner + Category 3/4 dispatch) |
| **#140** [backcompat-v4] | CI compatibility matrix: every fixture × every supported substrate version asserts renderability |
| **#141** [backcompat-v5] | Operator version-management UI in `loom edit serve` (show pinned versions, available upgrades, pending migrations) |
| **#142** [backcompat-v6] | Deprecation lifecycle as policy + tooling (`forge deprecate <field> --replaced-by <field>` cli + ADR auto-template) |
| **#143** [backcompat-v7] | Phase-level version-range declarations (each Forge phase declares which engine versions it supports + lints) |

---

## Cross-cutting integration

Versioning interacts with every other doctrine system:

- **Doctrine rules** (`doctrine/rules/*.toml`): each rule carries `version` per its lifecycle stage (experimental → stable → deprecated). The rule lifecycle parallels artifact lifecycle.
- **Trait declarations** (per `TRAIT_DAG.md`): each trait has a `version` field; trait deprecations follow this taxonomy.
- **Orientation values** (per `N_ORIENTATION_SUBSTRATE.md`): each orientation's closed enum is versioned per axis; adding a value is additive (Category 2), removing is operator-action (Category 4).
- **AI-posture inventory** (per `CAPABILITY_AI_POSTURE.md`): capability postures versioned; demoting D→A is operator-action (sovereignty implications).
- **MCP tool schemas** (`mcp/tools/*.json`): each `inputSchema` carries `$schema` URI tied to the substrate version it targets.
- **Substrate bypass register**: register format versioned; bypass-deadline semantics versioned (Category 1+2 changes).

---

## Anti-patterns

| ❌ Don't | ✅ Do |
|---------|------|
| Ship a Category 3 change without registry entry | Registry entry MANDATORY before merge |
| Use `~=` / open-ended ranges by default | Pin exact versions; ranges only when API stability proven |
| Skip the deprecation flag step + go straight to removal | Announce → flag → require; minimum one minor cycle between flag + require |
| Make audit chain entries renamable | Audit chain is Category 1+2 only; never 3 or 4 |
| Encode versions only in commit messages | Version tuple lives IN the artifact (TOML/JSON), not just in git history |
| Use floating "latest" pointers | Every pin is exact or range — never "latest" |
| Apply a registry-driven migration silently without telemetry | Emit `tracing::info!` + audit-chain entry recording the migration |
| Make migration signatures optional | Mandatory; the registry rejects unsigned entries |

---

## Why migration-friendly, NOT future-proof

We do not try to predict future requirements. Trying to design *now* for unknown future shape is how schemas accumulate "just in case" extension points that age badly.

Instead: we commit to making *every* breaking change safe via the migration registry. Trade attempted future-proofing for guaranteed migratability. The four-category taxonomy is the contract that lets us evolve aggressively while keeping the renderability guarantee.

---

## See also

- `SUBSTRATE_DISCIPLINE.md` — Rule 0; hand-coding is forbidden, every gap is a substrate change (and every substrate change is versioned).
- `DETERMINISTIC_FIRST.md` — deterministic baseline first; migrations themselves are deterministic Rust code.
- `TRAIT_DAG.md` — trait lifecycle parallels this taxonomy.
- `N_ORIENTATION_SUBSTRATE.md` — Lifecycle orientation = artifact lifecycle stage.
- `CAPABILITY_AI_POSTURE.md` — capability AI posture is versioned.
- `[[backward-compat-version-discipline]]` memory — the founding directive.
- `[[manifest-layer-is-the-keystone]]` — manifest projection includes version tuples.
- SemVer 2.0.0 — https://semver.org/
- RFC 3339 — timestamps in version tuples.
