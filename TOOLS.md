# TOOLS.md — PlausiDen-AVP-Doctrine

Canonical command index. The doctrine repo is consumed via Forge's `forge doctrine *` subcommands; this file points at the typed surface.

> Cross-repo TOOLS reference: see [../PlausiDen-Forge/TOOLS.md](../PlausiDen-Forge/TOOLS.md). Doctrine is read-only consumed by Forge.

---

## Doctrine queries (from PlausiDen-Forge)

```
forge doctrine query --rule <id>           Fetch a rule by id with full triple
forge doctrine query --domain <name>       List rules in a domain (build / primitives /
                                           security / testing / docs / logging / perf /
                                           content / accessibility)
forge doctrine query --lifecycle <state>   Filter by lifecycle (experimental / stable / deprecated)
forge doctrine query --search <substring>  Substring search across statement + rationale

forge doctrine for <path>                  Surface rules applicable to a workspace path
forge doctrine check                       Verify .citing(...) references resolve
forge doctrine exceptions                  Audit DOCTRINE-EXCEPTION tags
forge doctrine render --out docs/doctrine.md   Render full doctrine as Markdown
forge doctrine lifecycle                   Audit lifecycle state across rules
forge doctrine deprecation-audit           Flag .citing(...) of deprecated rules (per #142)
```

---

## Doctrine database layout

```
doctrine/rules/SCHEMA.md            Canonical rule shape (statement + rationale + enforcement)
doctrine/rules/build.toml           build-001 through build-008
doctrine/rules/primitives.toml      prim-001 through prim-012
doctrine/rules/security.toml        sec-001 through sec-010
doctrine/rules/testing.toml         test-001 through test-008
doctrine/rules/docs.toml            docs-001 through docs-008
doctrine/rules/logging.toml         log-001 through log-006
doctrine/rules/perf.toml            perf-001 through perf-008
doctrine/rules/content.toml         content-001 through content-006
doctrine/rules/accessibility.toml   a11y-001 through a11y-005
```

---

## Top-level doctrine documents

```
AVP2_PROTOCOL.md                The 36-pass adversarial validation protocol
SUBSTRATE_DISCIPLINE.md         Rule 0: substrate-only-path (load-bearing)
DETERMINISTIC_FIRST.md          LFI/LLM as opt-in augmentation
VERSION_DISCIPLINE.md           Backward-compat: version tuples + 4-cat taxonomy + migration registry
TRAIT_DAG.md                    54 traits across 11 categories
N_ORIENTATION_SUBSTRATE.md      12 orthogonal substrate orientations
MAPPING_TABLES.md               Cross-orientation curated mappings
CAPABILITY_AI_POSTURE.md        Per-capability D/A/P inventory
CONFIG_SURFACE.md               3-layer AI config surface
DOCTRINE.md                     Rule-database overview
INCIDENT_LOG.md                 Doctrine-relevant incidents
CHANGELOG.toml                  Doctrine-level change log
```

---

## Adding / modifying rules (from this repo)

There's no CLI directly inside this repo. Edits to `doctrine/rules/*.toml` flow through:

1. Open an ADR per rule `docs-004` (in this repo's `ship-decision/` or `gates/`).
2. Edit the TOML.
3. Verify via `forge doctrine query --rule <id>` from PlausiDen-Forge.
4. CI runs `forge doctrine check` on every PR (ensures citations downstream still resolve).
5. Dual-sign per `MAPPING_TABLES.md` § signature requirement.

---

## See also

- `AGENTS.md` — repo orientation
- `AVP2_PROTOCOL.md` — read before any doctrine change
- `../PlausiDen-Forge/TOOLS.md` — Forge-side TOOLS (where `forge doctrine` lives)
