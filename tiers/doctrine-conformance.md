# Doctrine Conformance Tier

**Added:** 2026-04-24 (v1.1).
**Applies to:** every PlausiDen doctrine repo (Audits, Tests, Obs, Canon, Harvest, Meta, AVP-Doctrine itself).

## Question

Does this doctrine repo internally conform to the structure required of all
PlausiDen doctrine repos?

## Required artifacts

A doctrine repo passes Doctrine Conformance Tier when it ships:

- `DOCTRINE.md` at repo root with versioned tenets.
- `doctrine/principles.toml` mirroring the tenets in machine-readable form.
- `doctrine/anti-patterns.toml` with named anti-patterns + rationale.
- `doctrine/maturity-model.toml` with tiered adoption levels.
- `integrations/avp.toml` declaring tier targets + current state.
- `integrations/harvest.toml` (or root `harvest.toml`) declaring upstream candidates (may be empty `[meta]` only).
- `LICENSE` (MIT for ecosystem-wide compatibility).
- `README.md` with the standardized repo-label HTML comment header (see [`PlausiDen-Meta/ECOSYSTEM_GUIDE.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/ECOSYSTEM_GUIDE.md)).

## Passing criterion

All artifacts present + each is well-formed (TOML parses, markdown renders,
no broken cross-links).

## Enforcement

Verified by `audit_tool.py` from `PlausiDen-Audits` once the corresponding
audit rule ships. Until then, manually verified per release.
