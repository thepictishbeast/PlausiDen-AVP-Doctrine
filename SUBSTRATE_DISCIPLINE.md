# Substrate Discipline

**Doctrine version:** 1.0
**Status:** normative for every PlausiDen consumer repo at adoption.
**Authored:** 2026-05-20.
**Source:** paul's directive 2026-05-20. Companion to `DOCTRINE.md`, alongside `AVP2_PROTOCOL.md`.

This document states the load-bearing architectural commitment for everything in the PlausiDen ecosystem. Read it before contributing code to any substrate or consumer repo.

---

## The rule

**The substrate is the only path.**

- Site work is done by editing CMS content (typed TOML / JSON).
- Capability work is done by editing the substrate (Forge / Loom / `*-core` / CMS schema).
- Hand-authoring CSS, HTML, or site-specific JavaScript is **forbidden**.
- If you find yourself wanting to hand-code anything, **stop**. That's a substrate gap. File a capability request, implement in the appropriate substrate repo, then exercise via CMS content.
- Substrate-bypass exists as a heavyweight, visible, tracked emergency mechanism. It is not a habit.

The corollary: if the substrate can't do something, the work is NOT "do it some other way." The work is to make the substrate able to do it, then use the substrate.

## Why this rule is uncompromising

Discipline collapses entirely with even one "in this case we just need a one-off" exception, because cumulatively every site has cases that feel one-off. Tilting all the way to "no exceptions" forces every gap to surface and be fixed, which is what matures the substrate. Without the rule, the substrate stays underdeveloped while sites accumulate ad-hoc bypass code that compounds drift across the ecosystem.

## Content vs code (the only allowed distinction)

| Layer | What it is | Where it goes | Status |
|-------|------------|---------------|--------|
| Site-specific content | Unique editorial writing, photographs, client logos | CMS content (typed TOML / JSON) | **Allowed** |
| Substrate code | Rust crates implementing capabilities, primitives, phases | Forge / Loom / `*-core` repos | **Allowed and required** |
| Hand-authored CSS / HTML / JS | Anything that bypasses Loom primitives or Forge rendering | **Nowhere** | **Forbidden** |
| Site-specific Rust | Code that exists only to serve one site | Nowhere; must be generalized into substrate | **Forbidden** |

If you're writing Rust, it goes in Forge / Loom / `*-core`. If you're writing TOML / JSON, it goes in CMS as typed content. If you're writing CSS / HTML / JS / a hand-rolled template, you're at the wrong layer and the fact that you needed to do it is a substrate bug.

## Canonical substrate defaults (do not relitigate)

These choices are made. Reaching for an alternative requires explicit written justification in a separate ADR.

- **HTTP server:** Axum. Not Actix, not Rocket, not Warp.
- **Async runtime:** Tokio. Not async-std, not smol.
- **HTML emission:** Maud, or Loom's typed primitives. Not handlebars, askama, or raw string formatting.
- **Database:** PostgreSQL via sqlx with compile-time query verification. Not diesel, sea-orm, rusqlite.
- **Serialization:** serde with `deny_unknown_fields` at every input boundary. No arbitrary JSON parsing.
- **Cryptography:** lift from PlausiDen-Engine (erasure / duress / deadman); Ed25519 for signatures per the Forge attestation chain; ML-DSA / ML-KEM for post-quantum per the Sacred.Vote substrate. No new cryptographic primitives invented per project.
- **CLI:** clap with the existing argument-pattern conventions from forge-cli and loom-bridge.
- **Errors:** anyhow at binary boundaries, thiserror within libraries, `?` everywhere, no `unwrap` / `expect` in non-test paths.
- **Property testing:** proptest with mandatory targets at input boundaries.
- **Logging:** tracing with structured fields, OTLP-compatible.
- **AI invocation:** through the LFI critic trait abstraction, never raw LLM output. And per `feedback_deterministic_first_lfi_optional`, LFI is opt-in augmentation, never load-bearing.
- **Document generation (docx / pdf / pptx):** via the skills system, which knows about the canonical formats. Not by reinventing the format.

Cargo workspace templates at `<workspace>/templates/<kind>/` ship Cargo.toml entries pre-pinned to the canonical defaults so new substrate crates start in the right place by construction.

## The four enforcement mechanisms

1. **Repository structure.** Site repos contain only CMS content, configuration, and build outputs. No `src/`, `styles/`, `scripts/`, or `templates/`. Filesystem layout forbids hand-coded artifacts by construction.
2. **Forge phase.** A Forge audit phase walks site repos and fails the build on detection of hand-authored CSS / JS / HTML or any other forbidden artifact. The phase is strict in production mode and cannot be suppressed. Finding emit cites this doctrine.
3. **PR review checklist.** Every PR is reviewed for: (a) code added outside substrate repos, (b) new substrate capabilities versus reused ones, (c) hand-authored artifacts in site repos, (d) violations of the canonical defaults. Generic substitutes are rejected with a pointer to the right substrate tool.
4. **Session-start orientation.** AGENTS.md in every PlausiDen repo leads with this rule. Claude (and any other agent) reads it before doing any work. The rule is the first thing surfaced; everything else follows from it.

## The capability-request workflow

When a substrate gap appears mid-work, the response is mechanical:

1. Stop the in-progress work.
2. File a capability request with the issue template (what's needed, why, what's blocked, proposed contract).
3. Either implement the capability in the appropriate substrate repo (if simple enough) or defer the blocked work (if not).
4. Exercise the new capability when it lands.

The capability request goes into a tracked queue visible to all contributors and AI agents. Backlog → prioritization → implementation → unblocking. The queue is the substrate's evolution driver.

## The substrate-bypass workflow (emergency mechanism)

Genuine cases will exist where the substrate cannot be extended in time and an operator needs a site to ship. The bypass is heavyweight and visible by design:

1. The operator approves the bypass in writing, scoped to a specific file and specific behavior.
2. The bypass is tagged in code: `// SUBSTRATE-BYPASS(issue-id): <reason>`.
3. A tracked issue exists from day one to backfill the bypass into the substrate proper.
4. The bypass appears in the build's audit report as a known exception, with the issue link.
5. The site's `bypass-register.toml` lists every active bypass with its tracking issue.
6. CI alerts when bypasses are 30 or more days old without backfill progress.
7. Removing a bypass (by backfilling into substrate) is a tracked milestone.

Bypasses exist but are visible enough that they do not become a habit. Accumulating bypasses for any single rule signal that the rule needs revision or that the substrate needs to support both cases properly.

## Friction acknowledgement (honest accounting)

The first ten to thirty sites built under this discipline are slower than hand-coded sites would be. Every site reveals substrate gaps that have to be filled before the site can ship. The substrate work is real engineering time that does not directly produce the site. This is the friction.

After thirty to fifty sites, the substrate is mature enough that most new sites compose from existing primitives. Friction inverts. Sites build very quickly because the hard work was done in the substrate, and the substrate's quality compounds across every future site.

Operator impatience does not override the discipline. Optimize for substrate maturation, not site-completion speed. The friction is the price of the substrate's eventual quality, and the eventual quality is the commercial value proposition.

## What customers see

Customers paying for the platform are paying for the substrate's maturity. Their site's quality is bounded by the substrate's quality. The platform's investment in substrate quality is the value customers receive. The discipline that forces every action to feed the substrate is the discipline that produces a platform worth paying for.

"Build your site on this platform and we guarantee operability through our evolution" is a rare commitment. Customers who care about sovereignty and longevity hear it as durable trust. The substrate discipline is what makes the commitment honorable.

## Connection to the rest of the doctrine

- **AVP-2 STILL BROKEN default:** every commit is guilty until proven innocent, including substrate commits. This doctrine extends that to: every hand-coded asset is a substrate failure until proven exceptional.
- **Backward-compatibility version discipline:** the substrate evolves through versioned migrations. Substrate-bypass is the explicit exception path; it does not become a parallel substrate.
- **Tool-starvation anti-pattern:** the discipline against reaching for generic tools is the same shape as the discipline against hand-coding sites. Both pressures favor the substrate.
- **Trait system + doctrine:** the trait system gives the substrate its typed inheritance. This doctrine gives the substrate its mandate.
- **Deterministic-first, LFI-optional:** the substrate works deterministically without AI. This doctrine ensures the substrate stays the platform's foundation, not AI augmentation.

## How AI agents apply this

Every AI agent working in PlausiDen reads `AGENTS.md` at session start. `AGENTS.md` leads with this rule. The agent's decision flow becomes:

1. Am I about to write CSS, HTML, JavaScript, or site-specific Rust? Stop.
2. What primitive / phase / capability would the substrate need so this work becomes pure CMS content authoring?
3. File the capability request. Implement in the appropriate substrate repo.
4. Then exercise the new substrate by authoring CMS content.
5. Never fall back to "the operator is impatient, let me just hand-code this one."

The discipline shapes Claude's, Gemini's, and any other AI agent's behavior because the substrate is structurally the path of least resistance, and the gate is visible, and the bypass workflow is heavyweight enough to deter casual use.

## Status

Adoption tier per `tiers/`:

- **Tier 1 (existence):** the doctrine is written and referenced. ✓ (this file).
- **Tier 2 (failure resilience):** the Forge phase detecting bypasses is implemented and strict in production. PENDING (substrate-discipline task #156).
- **Tier 3 (adversarial security):** the discipline survives adversarial pressure (impatient operators, productivity-vs-quality tension, AI agents tempted by easy paths). ONGOING — the practice is the verification.
- **Tier 4 (UX adversarial):** the discipline does not degrade operator experience over time as the substrate matures. ONGOING.
- **Tier 5 (integration):** the discipline is referenced from every AGENTS.md across PlausiDen repos. PENDING (#145, #165).
- **Tier 6 (meta-validation):** the discipline itself is auditable. ✓ (this document is normative; deviations from it are tracked exceptions per `EXCEPTIONS.md`).

When all six tiers reach checkmark status, the doctrine moves from `experimental` to `stable` in the AVP-Doctrine versioning per `CHANGELOG.toml`.
