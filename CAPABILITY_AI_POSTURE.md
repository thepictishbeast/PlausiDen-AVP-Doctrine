# CAPABILITY_AI_POSTURE.md

Per-capability AI-posture inventory. For every substrate capability across the PlausiDen ecosystem (Forge / Loom / Crawler / *-core / Annotator / LFI / Forge-LFI / MCP surface), this document classifies which AI posture applies:

- **D — Always deterministic.** No AI involvement; never. Adding AI is a doctrine violation.
- **A — Augmentable.** Deterministic primary, AI secondary. Trait abstraction with deterministic fallback. AI failure is silent + observable.
- **P — Primarily augmented.** AI primary, weaker deterministic fallback. Operator opts out and accepts reduced ergonomics.

> Per `[[deterministic-first-lfi-optional]]` doctrine: every capability lives in exactly one of these buckets, explicit and documented.

> Authored to close `#185 [determ-v2]`. Used by:
> - CI scenarios (#189) — assert posture matches manifest at compile time.
> - Operator posture switch — tenant disables AI; substrate still functions.
> - Forge phase enforcement (#188) — refactor AI-assuming code through the trait abstraction.

---

## Capability inventory

### Forge — build pipeline

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `forge build` pipeline orchestration | **D** | Phase ordering + finding aggregation; mechanical. |
| `forge.toml` parsing | **D** | serde + deny_unknown_fields at boundary. |
| Build report (Merkle chain) | **D** | Hash-chain is cryptographic; AI must never touch. |
| `forge verify` | **D** | Cryptographic integrity check. |
| `forge attest` sign | **D** | Ed25519 / ML-DSA — never AI. |
| `forge attest fingerprint` | **D** | SHA-256 base64url. |
| `forge content validate` | **D** | Schema validation against `cms-schema.json`. |
| `forge content format-list` | **D** | Static enum projection. |
| `forge content project-to-export` | **D** | Typed projection per export format. |
| `forge search` (IndexDoc[] validation) | **D** | Typed validation. |
| `forge assets` (image bundle + alt-text) | **A** | Bundle ladder = deterministic; alt-text *suggestion* = augmentable (operator accepts/edits). |
| `forge fix` | **A** | Mechanical fixes from finding records = deterministic; AI may *propose* additional fixes. |
| `forge watch` | **D** | inotify + debounce. |
| `forge orient` | **D** | Affordance + doctrine projection. Never AI. |

### Forge — audit phases

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `tokens` phase | **D** | Pattern matching on px/hex/rgb. |
| `html_semantic` | **D** | HTML AST walk + landmark check. |
| `contrast` | **D** | WCAG formula on declared colors. |
| `csp` / `csp_devmode` | **D** | Policy directive presence check. |
| `label_consistency` | **D** | Pattern matching on href↔label pairs. |
| `link_check` | **D** | Anchor ↔ id matching. |
| `phantom_button` | **D** | data-backend ↔ backends.toml cross-reference. |
| `substrate_purity` | **D** | Allow-list filesystem walk. Never AI. |
| `external_assets` | **D** | URL pattern match. |
| `asset_optimization` | **D** | ladder presence + WCAG alt-text length check. |
| `carbon_budget` | **D** | Calculated CO2e per asset bundle. |
| `dynamic_runtime` | **D** | AST scan for runtime-only patterns. |
| `id_strategy` | **D** | id-format regex. |
| `jurisdiction_compliance` | **D** | Curated mapping table lookup. |
| `dns_hygiene_lint` | **D** | DNS record format validation. |
| `crawl` (post-build snapshot) | **D** | URL discovery + walk. |
| `locale_html_lang` | **D** | `<html lang>` attribute presence. |
| `annotation_review` | **A** | Author annotations = deterministic; AI may *suggest* additional review prompts. |
| `aesthetic_distinctiveness` | **A** | Structural similarity against corpus = deterministic; semantic similarity via HDC = LFI-augmentable. |
| `backend_coverage` | **D** | data-backend slug enumeration. |
| `manifest-gate` (T33) | **D** | Acyclic + capability-resolution check. |
| `doctrine` phase | **D** | Citation resolution against typed rule DB. |
| `bypasses` phase | **D** | Source-tag ↔ register cross-reference. |

### Forge — doctrine subcommands

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `forge doctrine query` | **D** | Typed query over rule DB. |
| `forge doctrine for` | **D** | Path-context-token match. |
| `forge doctrine check` | **D** | Cross-reference resolution. |
| `forge doctrine exceptions` | **D** | Pattern + register match. |
| `forge doctrine lifecycle` | **D** | Lifecycle-stage grouping. |
| `forge doctrine render` | **D** | Markdown projection. |

### Forge — typed-config gates

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `forge manifest` | **D** | Acyclic + kebab-case + capability resolution. |
| `forge privacy` | **D** | RetentionPolicy + DataCategory validation. |
| `forge trust-safety` | **A** | Validation = deterministic; concern-kind suggestion = LFI-augmentable. |
| `forge domains` | **D** | RFC 1035 + RFC 8555 typed validation. |
| `forge forms` | **D** | WCAG labels + honeypot rules. |
| `forge federation` | **D** | Typed protocol↔address validation. |
| `forge email` | **D** | RFC 8058 list-unsubscribe rules. |
| `forge commerce` | **D** | ISO 4217 + price + SKU validation. |
| `forge memberships` | **D** | Tier validation. |
| `forge audit-log` | **D** | Hash-chain integrity. Cryptographic. Never AI. |
| `forge config` (umbrella) | **D** | Orchestrates the above. |

### Loom — substrate

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `CmsSection` enum + render | **D** | Typed render path. Never AI. |
| `loom-tokens` skin.css emission | **D** | Static token projection. |
| `loom-lint` | **D** | AST + pattern checks. |
| `loom validate` | **D** | Typed schema validation. |
| `loom edit serve` (CMS editor) | **A** | Authoring UI = deterministic; AI-assisted content generation = primarily-augmented (operator opt-in). |
| `loom deploy hetzner` | **D** | rsync + atomic swap. |
| `loom sync --regenerate` | **D** | Mechanical regeneration. |
| Theme variable resolution | **D** | Token lookup. |
| Visual regression baselines | **D** | Pixel-diff at fixed thresholds. |

### CMS authoring

| Capability | Posture | Rationale |
|------------|---------|-----------|
| Typed `CmsPage` parse | **D** | serde + deny_unknown_fields. |
| Heading / paragraph / pull-quote primitives | **D** | Typed primitive emission. |
| Content drift detection | **A** | Token-level diff = deterministic; semantic drift via HDC = LFI-augmentable. |
| Reading-level + decision-density check (a11y-005) | **A** | Token-count heuristics = deterministic; semantic complexity = LFI-augmentable. |
| Originality / similarity scoring | **A** | Structural similarity = deterministic; semantic similarity via HDC = LFI-augmentable. |
| Author-suggested next-section recommendation | **A** | Mapping-table lookup = deterministic; HDC similarity over corpus = LFI-augmentable. |
| Natural-language content generation (drafting) | **P** | LLM primary; fallback = templates + manual authoring. |
| Conversational authoring interface | **P** | LLM primary; fallback = structured form. |

### Crawler — runtime audit

| Capability | Posture | Rationale |
|------------|---------|-----------|
| Journey execution (chromiumoxide) | **D** | Mechanical step execution. |
| Screenshot capture | **D** | Browser API. |
| Detector trait impls (general) | **D** | DOM walk + pattern checks. |
| `contrast_runtime` axis | **D** | WCAG formula on computed colors. |
| `viewport_overflow` axis | **D** | DOM measurement. |
| `hidden_elements` axis | **D** | DOM/CSS state inspection. |
| `font_missing` axis | **D** | FontFaceSet API check. |
| `fouc` (flash of unstyled content) | **D** | First-paint timing. |
| `layout_thrash` axis | **D** | Layout-shift events. |
| `image_desert` axis | **D** | Visual density heuristic. |
| `broken_text_RTLLTR` axis | **D** | bidi-mark + direction state. |
| `gradient_text_mid_word_clip` axis | **D** | Glyph-box measurement. |
| Anomaly summarization for human review | **A** | Mechanical grouping = deterministic; semantic summary = LFI-augmentable. |

### *-core typed surfaces

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `privacy-core` (DataCategory + RetentionPolicy) | **D** | Typed validation. |
| `trust-safety-core` (ConcernKind enum) | **A** | Concern kind enumeration = deterministic; concern detection = LFI-augmentable. |
| `domains-core` | **D** | Typed validation. |
| `forms-core` | **D** | Typed validation + WCAG. |
| `federation-core` | **D** | Typed protocol↔address pairs. |
| `email-core` | **D** | RFC 8058 + DKIM rules. |
| `commerce-storefront-core` | **D** | ISO 4217 + price + SKU. |
| `memberships-core` | **D** | Tier validation. |
| `observability-core` (AuditChain) | **D** | Hash-chain. Cryptographic. Never AI. |
| `importers-core` / `exporters-core` | **D** | Typed conversion. |
| `assets-core` | **D** | Image bundle + WCAG alt-text. |
| `search-core` | **D** | IndexDoc[] validation. |
| `manifest-core` | **D** | Phases.toml + backends.toml typed projection. |
| `doctrine-core` | **D** | Rule DB parser. Pure projection. |
| `forge-critic` | **A** | Critic trait abstraction; D = pattern rules; A = LFI-augmented findings. |

### Annotator

| Capability | Posture | Rationale |
|------------|---------|-----------|
| Annotation primitive layer | **D** | Typed annotation emission. |
| Cross-run anomaly grouping | **A** | Hash-based grouping = deterministic; semantic grouping = LFI-augmentable. |
| Author-assistant prose for annotations | **P** | LLM primary; fallback = template-based. |

### LFI repos (PlausiDen-LFI / Forge-LFI)

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `lfi-core` (NeuPSL evaluator + HDC ops) | **D** | Inside LFI: deterministic numerical operations. From the *consumer's* perspective the *outputs* are augmentation, but the LFI primitives themselves are deterministic. |
| Critic trait impls (deterministic-baseline) | **D** | Pattern + mapping table; the deterministic Critic side. |
| Critic trait impls (LFI-augmented) | **A** | The augmented Critic side. Trait makes the substitution explicit. |
| Critic trait impls (LLM-augmented) | **P** | LLM-primary Critic. |
| Policy evaluation (NeuPSL DSL) | **A** | DSL eval = deterministic per inputs; the *belief encoding* is augmentation. |
| HDC corpus encoding | **A** | Encoding = deterministic given a vector layout; the *vector layout* itself is a learned augmentation. |

### MCP surface

| Capability | Posture | Rationale |
|------------|---------|-----------|
| `forge_orient` MCP tool | **D** | Wraps the deterministic CLI. |
| `forge_build` MCP tool | **D** | Wraps the deterministic CLI. |
| `forge_doctrine_*` MCP tools | **D** | Wrap deterministic CLI subcommands. |
| `forge_audit_*` MCP tools | **D** | Wrap deterministic CLI subcommands. |
| `forge_config` MCP tool | **D** | Wraps deterministic CLI. |
| `crawler_journey` MCP tool | **D** | Wraps deterministic journey runner. |
| `loom_validate` MCP tool | **D** | Wraps deterministic loom validate. |
| Any future MCP tool that wraps a LFI/LLM operation | **A** or **P** | Inherits the posture of the underlying CLI. |

---

## Summary counts

```
Forge build pipeline + audit phases:      30 D, 2 A, 0 P
Forge doctrine subcommands:                6 D, 0 A, 0 P
Forge typed-config gates:                 10 D, 1 A, 0 P
Loom substrate:                            8 D, 1 A, 0 P
CMS authoring:                             3 D, 4 A, 2 P
Crawler runtime audit:                    12 D, 1 A, 0 P
*-core typed surfaces:                    14 D, 2 A, 0 P
Annotator:                                 1 D, 1 A, 1 P
LFI repos:                                 2 D, 3 A, 1 P
MCP surface (current):                    14 D, 0 A, 0 P

Total inventoried capabilities:          ~111
  Always deterministic (D):              ~95   (~86%)
  Augmentable (A):                       ~13   (~12%)
  Primarily augmented (P):                ~4   (~3%)
```

> **86% of the substrate is and remains always-deterministic.** AI augmentation is opt-in for a small, well-defined frontier (similarity / drift / authoring assistance). This is the structural commitment: a sovereignty-focused tenant disabling all AI loses ergonomics in 16% of operations and zero correctness anywhere.

---

## How to read this inventory

For any capability not listed: **default to D (always deterministic)** until a capability request justifies promotion to A or P. The inventory expands by capability request (per `docs/CAPABILITY_REQUEST_WORKFLOW.md`) — never by drift.

For any capability classed A or P: the implementation MUST follow the trait abstraction pattern (per `DETERMINISTIC_FIRST.md` § "The trait abstraction pattern"). If a current implementation doesn't, it's a refactor target (task #188).

---

## Enforcement

- **CI Scenario A** (#189 — AI disabled at compile time): every D + every A capability must function. Every P capability either functions in degraded mode or is gated behind the AI flag.
- **CI Scenario B** (#189 — AI runtime-failed): same expectation; A capabilities silently fall back; P capabilities emit `tracing::warn` + use deterministic fallback.
- **Doctrine check** (`forge doctrine check`): cites this inventory; a finding citing a posture mismatch fails build.

---

## Implementation arc

This inventory (#185, this doc) closes by providing the categorization. Tasks that consume it:

| Task | How it uses this inventory |
|------|----------------------------|
| **#186** [determ-v3] | For each A and P capability, implement the trait abstraction (Critic trait + deterministic impl + augmented impl + composite). |
| **#187** [determ-v4] | Layered config surface (platform / tenant / operation) reads this inventory to know which capabilities to expose toggles for. |
| **#188** [determ-v5] | Audit + refactor existing AI-assuming code; this inventory names the targets. |
| **#189** [determ-v6] | CI scenarios A + B drive their assertions from this inventory's D / A / P distribution. |

---

## See also

- `DETERMINISTIC_FIRST.md` — the architectural doctrine (this is its actionable companion).
- `TRAIT_DAG.md` — trait system; orthogonal to AI posture.
- `N_ORIENTATION_SUBSTRATE.md` — Risk + Sovereignty + Compliance orientations interact with AI posture decisions.
- `SUBSTRATE_DISCIPLINE.md` — substrate-only-path; hand-coding around a deterministic baseline is still forbidden.
- `[[deterministic-first-lfi-optional]]` memory — the founding doctrine inversion.
