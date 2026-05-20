# AGENTS.md — PlausiDen-AVP-Doctrine

Orientation for any AI agent (Claude or otherwise) working in this repository. Read **before** writing any code or running any script.

> Cross-repo orientation: see [../PlausiDen-Forge/PLAUSIDEN_ECOSYSTEM.md](../PlausiDen-Forge/PLAUSIDEN_ECOSYSTEM.md) for how AVP-Doctrine relates to the substrate.

---

## RULE 0 — Doctrine is canonical; code cites it

This repo IS the doctrine. Substrate code (Forge / Loom / Crawler / Annotator / *-core / LFI) cites rules from this repo via `Finding.citing(["<rule-id>"])`. Doctrine NEVER imports code from those repos — strict one-way dependency per `PLAUSIDEN_ECOSYSTEM.md`.

**Forbidden:**
- Editing rule TOML files without going through the doctrine review flow + ADR.
- Removing a rule (use `lifecycle = "deprecated"` + `replaced_by`; never delete).
- Adding a rule without the full triple (`statement` + `rationale` + `enforcement`) per rule `docs-003`.
- Importing substrate code into doctrine — doctrine is pure declarative TOML + Markdown.

**Canonical:**
- 71 rules across 9 domains: `build` / `primitives` / `security` / `testing` / `docs` / `logging` / `perf` / `content` / `accessibility`.
- Each rule has a triple: statement (one sentence) + rationale (multi-paragraph) + enforcement (list of mechanisms — Forge phase / Crawler axis / loom-lint / property test / etc).
- Lifecycle: `experimental` → `stable` → `deprecated`. Per `VERSION_DISCIPLINE.md`.

---

## RULE 1 — Look before you build

Before adding / modifying doctrine:

1. **`forge doctrine query --rule <id>`** — fetch existing rule.
2. **`forge doctrine query --domain <name>`** — list all rules in a domain.
3. **`forge doctrine lifecycle`** — audit lifecycle state.
4. **`forge doctrine for <path>`** — see which rules apply to a path.
5. **Read `doctrine/rules/SCHEMA.md`** — the canonical rule shape.

---

## Repository layout

```
PlausiDen-AVP-Doctrine/
├── AVP2_PROTOCOL.md            Adversarial Validation Protocol v2 — the 36-pass validation gate
├── SUBSTRATE_DISCIPLINE.md     Rule 0: substrate-only-path doctrine (load-bearing)
├── DETERMINISTIC_FIRST.md      LFI/LLM as opt-in augmentation; deterministic baseline mandatory
├── VERSION_DISCIPLINE.md       Backward-compat: version tuples + 4-category change taxonomy + migration registry
├── TRAIT_DAG.md                54 traits across 11 categories — typed projection of entity capabilities
├── N_ORIENTATION_SUBSTRATE.md  12 orthogonal substrate orientations + cross-orientation queries
├── MAPPING_TABLES.md           Cross-orientation curated mappings (domain→compliance, etc.)
├── CAPABILITY_AI_POSTURE.md    Per-capability D/A/P inventory (deterministic / augmentable / primarily-augmented)
├── CONFIG_SURFACE.md           3-layer AI config surface (platform / tenant / operation)
├── DOCTRINE.md                 The rule-database overview + rendering instructions
├── INCIDENT_LOG.md             Doctrine-relevant incidents
├── CHANGELOG.toml              Doctrine-level change log
├── doctrine/
│   ├── anti-patterns.toml      Forbidden patterns enumerated
│   ├── maturity-model.toml     Maturity tiers
│   ├── principles.toml         First-principles statements
│   └── rules/                  The canonical rule database
│       ├── SCHEMA.md           Rule shape definition
│       ├── build.toml          build-001 through build-008
│       ├── primitives.toml     prim-001 through prim-012
│       ├── security.toml       sec-001 through sec-010
│       ├── testing.toml        test-001 through test-008
│       ├── docs.toml           docs-001 through docs-008
│       ├── logging.toml        log-001 through log-006
│       ├── perf.toml           perf-001 through perf-008
│       ├── content.toml        content-001 through content-006
│       └── accessibility.toml  a11y-001 through a11y-005
├── standing-orders/            Multi-Claude coordination protocols
├── crons/                      Scheduled job specs
├── annotations/                Annotation primitives
├── gates/                      Gate definitions
├── ipc/                        Inter-process coordination
├── loops/                      Loop / cron / cycle specs
├── tiers/                      AVP-2 tier ladder definitions
├── timers/                     Timer specs
├── prompts/                    Canonical prompts
├── integrations/               External integration specs
├── ship-decision/              SHIP-DECISION: gate definitions
└── foss-absorption/            FOSS-absorption review notes
```

---

## Anti-patterns — do NOT do these

- ❌ Add a rule without `enforcement: [...]` — without enforcement it's a wish, not a rule (rule `docs-003`).
- ❌ Add a rule without `rationale` — future contributors can't judge edge cases (rule `docs-003`).
- ❌ Delete a rule — use `lifecycle = "deprecated"` + set `deprecated_at` + optionally `replaced_by`.
- ❌ Promote `experimental` → `stable` without a trial period — verify enforcement is reliable first.
- ❌ Cite an external doctrine rule by URL — cite the rule id; doctrine is the source of truth.
- ❌ Add a doctrine rule without an ADR (rule `docs-004`).
- ❌ Edit a `.toml` rule file without dual-signing the change (per `MAPPING_TABLES.md` § signature requirement).

---

## Doctrine references

- `AVP2_PROTOCOL.md` — the 36-pass validation protocol (read this before any doctrine change)
- `SUBSTRATE_DISCIPLINE.md` — Rule 0; the load-bearing doctrine
- `DETERMINISTIC_FIRST.md` — LFI/LLM posture
- `VERSION_DISCIPLINE.md` — backward-compat + 4-category change taxonomy
- `TRAIT_DAG.md` — typed entity capabilities
- `N_ORIENTATION_SUBSTRATE.md` — 12 orthogonal substrate axes
- `MAPPING_TABLES.md` — cross-orientation curation workflow
- `CAPABILITY_AI_POSTURE.md` — D/A/P inventory
- `CONFIG_SURFACE.md` — 3-layer AI config
- `doctrine/rules/SCHEMA.md` — rule shape

---

## First steps when starting work in this repo

1. **Read `AVP2_PROTOCOL.md`** — the validation framework all doctrine changes pass through.
2. **Read the doctrine doc closest to your change** (SUBSTRATE_DISCIPLINE / DETERMINISTIC_FIRST / VERSION_DISCIPLINE / TRAIT_DAG / etc).
3. **Run `forge doctrine query --rule <id>`** to fetch any existing rule you're modifying.
4. **Run `forge doctrine lifecycle`** to see current lifecycle health before adding new rules.
5. **Open an ADR** for any architectural decision (rule `docs-004`).
6. **Get dual-signature** on the entry per `MAPPING_TABLES.md` § curation workflow.

This repo's commits should never break the substrate that depends on it. CI in PlausiDen-Forge runs `forge doctrine check` which refuses orphan citations — be aware that removing a rule id breaks downstream.

If you are about to edit `doctrine/rules/*.toml`, stop and re-read RULE 0 + the AVP-2 protocol.
