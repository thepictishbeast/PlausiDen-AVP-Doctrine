# Adversarial Validation Doctrine

**Doctrine version:** 1.0
**Engine version:** specification-only (no executable engine; gates + scripts are reference impls).
**Status:** normative for every PlausiDen consumer repo at adoption.

This file is the *philosophy*. The *rules* live in [`doctrine/`](doctrine/) as
machine-readable TOML. The *full protocol* lives in
[`AVP2_PROTOCOL.md`](AVP2_PROTOCOL.md). Conformance to this doctrine is
audited by [`PlausiDen-Audits`](https://github.com/thepictishbeast/PlausiDen-Audits)
and tier-graded against [`tiers/`](tiers/).

---

## Tenets

### 1. Code is guilty until proven innocent.

Every line is broken until it has survived adversarial testing. Tests, tools,
dependencies, OS, and compiler are all assumed compromised. Innocence is
provisional and revoked on the next commit.

### 2. The adversary always bothers.

The threat model is a state-actor with full source access, supply-chain
compromise, hardware implants, AI-assisted vuln discovery, and an active
breach assumed in progress now. "Nobody would bother" is not a valid
assessment.

### 3. The loop is six tiers, minimum thirty-six passes.

A body of code is not fit to ship until it has been through Tier 1
(existence), Tier 2 (failure resilience), Tier 3 (adversarial security),
Tier 4 (UX/UI adversarial), Tier 5 (integration & ecosystem), Tier 6
(meta-validation). The pass count is the floor, not the ceiling.

### 4. Absorb FOSS; do not depend on it.

Capabilities present in the FOSS ecosystem are vendored, hardened, and
brought under the project's own AVP loop per the
[`foss-absorption/`](foss-absorption/) protocol. Upstream is monitored;
upstream is not trusted.

### 5. Cross-repo fixes are not someone else's problem.

A bug found in one repo that applies to a sibling is fixed in the sibling
within the same session, tagged `AVP-CROSSFIX from <source-repo>`. Past-self
in another repo is treated as an untrusted external contributor.

### 6. Every annotation is machine-grepable.

The annotation taxonomy (`BUG ASSUMPTION:`, `SAFETY:`, `SECURITY:`,
`UX-DEBT:`, `REGRESSION-GUARD:`, `FOSS-ABSORBED:`, `SUPERSOCIETY:`,
`DEBUG-REMOVE:`, `SHIP-DECISION:`, `CROSSFIX:`, `LEAK-JUSTIFIED:`,
`AVP-PASS-N:`) is closed and grep-discoverable. New annotation types go
through the doctrine amendment process.

### 7. Every public function carries `BUG ASSUMPTION:`.

A public function without a documented failure-mode comment is not yet
audited. Every `unsafe` block carries a `SAFETY:` proof. Every secret
comparison is constant-time and annotated.

### 8. The ship verdict is always STILL BROKEN.

`SHIP-DECISION:` annotations list accepted residual risks, mutation-score,
coverage, and the human who signed. Shipping is explicit risk acceptance,
not a declaration of correctness. The loop resumes on the next commit.

### 9. Standing orders are read at session start.

Every AI agent reads its role file in [`standing-orders/`](standing-orders/)
at session start and re-reads on every context compaction. Roles are
stack-neutral; agent-specific behaviors are derived from the role, not
hardcoded.

### 10. Coding gates run before commits, not after.

Per-language gates in [`gates/`](gates/) execute before commits land.
A failed gate is not bypassed via `--no-verify`; it is fixed at root cause.

### 11. AI assists; AI does not gate.

LLMs may draft tests, explain failures, propose codemods. AI never
autonomously waives a violation, mutates a baseline, runs as a CI gate, or
issues a ship verdict. Determinism over plausibility.

### 12. The recursion stops at the axiom floor.

Per [`PlausiDen-Meta/AXIOM_FLOOR.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/AXIOM_FLOOR.md),
AVP Tier 0 is the axiom floor. Below it, claims are asserted by fiat. This
prevents infinite meta-recursion and weaponized meta-skepticism.

---

## Anti-patterns (see also [`doctrine/anti-patterns.toml`](doctrine/anti-patterns.toml))

- **`--no-verify` to skip a failing gate.** The gate caught a real problem; bypassing is not a fix.
- **`#[allow(...)]` without rationale.** Suppressed warnings rot into silent regressions.
- **`unwrap()` / `expect()` in library code without `SAFETY:`.** Async crashes propagate; library panics are external bugs.
- **Vendoring without hardening.** Absorbed FOSS that retains its original `unwrap` density inherits its incidents.
- **`SHIP-DECISION:` without listed residual risks.** Ship verdicts must enumerate what is being accepted.
- **AI-generated code merged without human ratification.** AI assists; humans ratify.
- **Ahead-of-trigger doctrine work.** Per [`PlausiDen-Meta/PRIORITY.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/PRIORITY.md), building before a trigger fires is a doctrine violation against the meta-layer.
- **Cross-repo bug ignored because "not my repo."** A sibling-applicable fix not applied is a propagation failure.
- **Tests of tests as a substitute for tests.** Meta-validation supplements, never replaces, primary tests.

---

## Maturity model

Per-consumer-repo maturity. Tracked in each consumer's `integrations/avp.toml`.

| Tier | Criterion |
|---|---|
| **0 — Axiom floor** | Repo exists; CLAUDE.md installed; standing orders linked. |
| **1 — Existence proof** | Tier 1 passes (existence, null/empty, boundary, error paths, type tightening, dep audit) green in CI. |
| **2 — Failure resilience** | Tier 2 passes (fault injection, concurrency chaos, exhaustion, degradation, integrity, combined chaos) green in CI. |
| **3 — Adversarial security** | Tier 3 passes (deser, injection, auth/authz, crypto, side-channel, supply-chain, network, data-at-rest, data-in-transit, opsec, fuzzing) green in CI. |
| **4 — UX/UI adversarial** | Tier 4 passes (first-contact, error UX, accessibility, performance UX, adversarial user, design consistency) green for any user-facing surface. |
| **5 — Integration & ecosystem** | Tier 5 passes (cross-repo integration, contribute back, sibling test suite) green; CROSSFIX commits land in siblings. |
| **6 — Meta-validation** | Tier 6 passes (mutation testing <5% survival, property-based ≥10k cases, formal verification where feasible) green. |

A repo is `passing` at a tier when every numbered pass at that tier is
either green in CI or has a filed exception per
[`PlausiDen-Meta/EXCEPTIONS.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/EXCEPTIONS.md).

---

## Companion doctrine: language selection

[`SOVEREIGN_POLYGLOT_STACK.md`](SOVEREIGN_POLYGLOT_STACK.md) — the canonical "best language for each domain" reference, sovereignty-weighted. Quarterly review enforced via `crons/quarterly-sovereign-stack-review.cron`. Every choice in this doc applies the AVP filter: fitness × governance/capture-risk × maturity. Update any time a watched language hits 1.0, a dependency relicenses, or a captive platform changes our deploy surface.

---

## How to propose a doctrine change

1. PR editing this file plus the relevant `doctrine/*.toml`.
2. **Why** in the PR body — cite a concrete incident, near-miss, external authority change, or cross-repo inconsistency. Speculative amendments are rejected per [`PlausiDen-Meta/GOVERNANCE.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/GOVERNANCE.md).
3. Bump the doctrine version in this file's header per the SemVer-for-doctrine rules in `GOVERNANCE.md`.
4. The PR sits in the public-comment period (default 7 days, 30 for axiom-floor changes, 24h for `EMERGENCY:` security amendments).
5. The PR runs the AVP doctrine-conformance + generality tiers against itself.
6. One maintainer ratifies.
