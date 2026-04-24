# Incident Log

Per [`PlausiDen-Meta/OPERATING_PRINCIPLES.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/OPERATING_PRINCIPLES.md)
§1: every doctrine repo maintains an append-only log of incidents that prompted
amendments. This file is the evidence base for doctrine net-negative-work
measurement.

Format per entry:

```markdown
## YYYY-MM-DD — One-line incident title

**Symptom**: What was observed.

**Root cause**: Why it happened.

**Doctrine response**: Which tenet, principle, or anti-pattern (existing or new) addresses this. Link to amendment if applicable.

**Hours saved going forward** (estimate, falsifiable): N hours per occurrence × estimated future occurrences.
```

---

## 2026-04-24 — AVP-Doctrine repo declared self-non-conformant

**Symptom**: Doctrine Conformance Tier (`tiers/doctrine-conformance.md`) declared itself binding on every doctrine repo "(Audits, Tests, Obs, Canon, Harvest, Meta, AVP-Doctrine itself)" requiring `DOCTRINE.md`, `doctrine/principles.toml`, `doctrine/anti-patterns.toml`, `doctrine/maturity-model.toml`, `integrations/avp.toml`, `integrations/harvest.toml`. AVP-Doctrine itself shipped without any of these files.

**Root cause**: Tier was added in v1.1 against a then-current repo state that hadn't been retroactively fixed. The conformance check was a forward gate without a self-audit pass.

**Doctrine response**: Added the missing artifacts. New principle (forthcoming amendment): doctrine repos that introduce a binding conformance rule must self-audit against that rule in the same PR.

**Hours saved going forward**: ~2 hours per future doctrine-conformance addition (audit pass + retrofit pass collapsed into one).

---

## 2026-04-24 — AVP-Doctrine README enumerated 17 product repos

**Symptom**: `README.md` "Repos this governs" section listed 17 specific PlausiDen product repos, violating `PlausiDen-Meta/SCOPE.md` independence test ("if a stranger cloned this repo tomorrow and knew nothing about the maintainer's specific projects — would this file make complete sense as a standalone artifact?").

**Root cause**: Doctrine repos pre-date the SCOPE.md independence test. The product list felt like helpful documentation; it was contamination.

**Doctrine response**: Replaced enumerated product list with a one-sentence reference to `PlausiDen-Meta/REPO_LABEL_REGISTRY.md` (informational ecosystem index, scoped as such). Anti-pattern entry: "doctrine-without-citation" already covers speculative amendments; new anti-pattern "consumer-enumeration-in-doctrine" candidate for next amendment.

**Hours saved going forward**: ~30 minutes per future doctrine-repo creation (de-contamination sweep happens up-front, not as remediation).
