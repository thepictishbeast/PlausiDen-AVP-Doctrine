# Deterministic First — LFI / LLM as Optional Augmentation

**Doctrine version:** 1.0
**Status:** normative for every PlausiDen substrate repo at adoption. SUPERSEDES the earlier "LFI is the brain, LLM is peripheral" framing.
**Authored:** 2026-05-20.
**Source:** paul's directive 2026-05-20. Companion to `SUBSTRATE_DISCIPLINE.md` and `DOCTRINE.md`.

This document states the architectural inversion that governs how AI participates in the PlausiDen platform. Read it before adding AI capabilities to any substrate repo and before treating AI as load-bearing for any platform behavior.

---

## The rule

**Every substrate capability has a deterministic baseline. LFI and LLM are opt-in augmentation layers. The platform works correctly without AI; AI improves specific outcomes when present; disabling AI never breaks the platform.**

Three corollaries:

1. The deterministic baseline is implemented and tested *before* any AI augmentation is considered.
2. AI augmentation is added through a typed trait abstraction; calling code never imports AI dependencies directly.
3. AI failure (service down, model crash, rate limit, configuration disabled) falls back to the deterministic baseline silently — never panics, never raises.

The earlier framing of "LFI is the brain, LLM is peripheral" treated AI as load-bearing. This doctrine inverts that: the substrate's deterministic mechanisms are load-bearing; AI augments them.

## Why the inversion

Three independent reasons compound:

**AVP-2 alignment.** The STILL BROKEN default presumes every commit guilty until proven innocent. LFI itself is a commit; depending on unproven LFI would itself violate the doctrine. The deterministic baseline can be proven innocent through traditional verification (types, tests, audits, formal methods where applicable). LFI proves itself separately on its own timeline; its proof does not gate the platform's proof.

**Operational confidence.** LFI is not yet production-trusted. Building dependence on a not-yet-trusted component compounds risk — every LFI mistake propagates through the platform, debugging becomes confusing because bugs could be platform bugs or LFI bugs. The deterministic baseline is auditable in a way AI-augmented behavior is not. When something goes wrong, the deterministic path is traceable and the failing rule is identifiable.

**Commercial flexibility.** Sovereignty-conscious customers can disable AI entirely; convenience-focused customers can enable it. The same platform serves both audiences. This is meaningful differentiation: "the platform you're paying for works deterministically and verifiably; AI is value-add making it work better in specific ways, but you're not depending on it to function" is a rare commitment.

## The three layers

| Layer | Always present? | Role |
|-------|-----------------|------|
| Deterministic baseline | Yes, always | The platform itself. Trait system, orientation declarations, audit phases, schema validation, lints, type system, curated mapping tables, human-curated style packs. |
| LFI augmentation | Opt-in per platform / tenant / operation | HDC similarity, NeuPSL policy evaluation, interpretable drift detection, lexicon search. Each LFI capability has a deterministic fallback. Enriches when present. |
| LLM augmentation | Opt-in per platform / tenant / operation | Natural language generation, conversational interfaces, free-form reasoning. Fallbacks are weaker (manual content authoring vs generation) but the substrate still works. |

The architectural commitment is that augmentation is strictly additive. Disabling either layer reduces functionality but does not break correctness.

## Per-capability categorization

Every substrate capability lands in one of three categories. The categorization is explicit and documented per capability.

**Always deterministic — no augmentation layer:**
- Type system + schema validation
- Build-time correctness checks
- Cryptographic operations
- Permission + authentication checks
- Audit log integrity
- Version migrations
- The capability manifest itself

These are correctness-critical. They never use AI. An attempt to add AI augmentation to one of these is a doctrine violation.

**Augmentable — deterministic primary, AI secondary:**
- Originality and similarity checks
- Content drift detection
- Policy evaluation for soft rules
- Recommendation systems
- Quality scoring
- Anomaly detection

These have useful deterministic baselines that AI can enrich. The deterministic implementation is the default; AI augmentation runs alongside when enabled.

**Primarily augmentation — deterministic fallback exists but is weaker:**
- Natural language generation (LLM primary; fallback = templates + manual author input)
- Conversational interfaces (LLM primary; fallback = structured forms)
- Free-form structural reasoning (LLM primary; fallback = explicit author decision)

The deterministic fallback ensures the platform still works without AI but is less ergonomic. Operators opting out of AI work harder; the platform stays functional.

## The trait abstraction pattern

For every augmentable capability, the implementation pattern is consistent:

1. **Capability trait** defines the contract. Methods return findings, recommendations, scores, whatever the capability produces.
2. **Deterministic implementation** does the work using only baseline mechanisms: pattern matching, schema validation, mapping table lookups, statistical comparison against reference corpora. No AI dependency. Always returns useful output.
3. **AI-augmented implementation** does the same work plus AI's contribution. Returns richer output when available.
4. **Composite implementation** runs both when AI is configured; only deterministic otherwise.
5. **Runtime configuration** determines which implementation is active. Calling code interacts only through the trait — never knows or cares which implementation ran.

This is dependency injection or strategy pattern, well-understood, no special architecture. The discipline is committing to the pattern consistently across every augmentable capability. Never embed AI directly in calling code; always route through the trait abstraction.

CI enforces the pattern: lint rules detect direct AI imports outside designated augmentation implementations. Violations fail the build.

## Layered configuration surface

Configuration determines whether AI is used, at three levels:

**Platform-level.** Whether AI is compiled into this deployment at all. Self-hosted deployments can disable AI entirely — no integration compiled in, no runtime dependency. Managed-hosting deployments can have AI compiled in but tenant-configurable.

**Tenant-level.** Whether this tenant uses AI augmentation. Sovereignty-focused tenants disable everything beyond the deterministic baseline. Convenience-focused tenants enable AI capabilities they want.

**Operation-level.** For specific operations, whether to use AI augmentation. Some operations are augmentation-eligible; others are not. The tenant can opt in or out per operation type.

**Fail-closed defaults.** If AI is unavailable for any reason — service down, model failed to load, rate limit hit, configuration disabled — operations fall back to the deterministic baseline silently. The platform emits a `tracing::warn` for observability and continues. The platform does not error because AI is unavailable; it operates at the baseline level for that operation.

This is the structural guarantee that makes AI augmentation rather than dependency: if AI fails, the platform keeps working.

## CI verification

Two permanent CI scenarios verify the architectural commitment:

**Scenario A — AI disabled at compile time.** Build the platform with the AI integration feature flag off. Run the full Forge build pipeline plus Crawler journeys plus audit phases against a reference site. Assert zero errors, all expected outputs present, baseline findings match the expected set.

**Scenario B — AI compiled in but runtime-failed.** Build the platform with AI enabled. Run the pipeline with `LFI_FORCE_FAIL=1` (and equivalent for LLM) simulating runtime AI failure. Assert zero errors, graceful degradation, warnings emitted but no panics, baseline findings still present.

Both scenarios block PR merge. The platform's correctness floor is enforced by these tests permanently.

## Deterministic enforcement mechanisms (the substrate's actual toolkit)

The deterministic baseline is not just "no AI" — it is a specific set of enforcement mechanisms that work without AI:

- **Type system.** Rust traits implementing substrate-level traits. Typestate patterns where invalid states are unrepresentable. `Form<Unvalidated>` is a different type than `Form<Validated>`; passing unvalidated to a method expecting validated is a compile error.
- **Schema validation at every input boundary.** serde with `deny_unknown_fields`. Typed clap derivations. Typed HTTP extractors. Boundary rejects invalid input; downstream code assumes validity.
- **Property-based testing.** proptest at every input-accepting function. Invariants asserted across the space of valid inputs. Catches the class of bugs where a property holds for tested cases but breaks for generated cases.
- **Audit phases as plain Rust code.** Read inputs, apply checks, emit findings. No AI judgment, no probabilistic evaluation. The mapping tables are hand-curated; the lookups are mechanical.
- **Configuration-driven rules.** Doctrine as structured TOML. Tools query the rule database; queries deterministic; rule applications deterministic.
- **Custom lint rules.** Clippy plus project-specific lints. Pattern violations caught at static analysis. No raw class strings outside Loom. No HTTP framework other than Axum. No cryptographic code outside designated crypto crates.
- **Pattern matching at multiple levels.** HTML / CSS / filesystem / output text. Regex or AST-based. Easy to add and test.

The substrate's correctness depends on these mechanisms, not on AI evaluation. Quality through curation, enforcement through code.

## What AI augmentation actually provides

When LFI is enabled:

- **Originality and similarity:** HDC similarity over the encoded corpus catches sites that are structurally different but semantically similar (same vocabulary, same arguments, same examples). Structural similarity (always present) catches the easier case; semantic similarity adds depth.
- **Policy evaluation for soft rules:** NeuPSL evaluates rules that don't fit cleanly into deterministic predicates — style fit, brand voice alignment. Hard rules (must satisfy) stay deterministic; soft rules can use probabilistic evaluation.
- **Recommendations:** HDC similarity over the corpus suggests primitives that have been used effectively in similar contexts. Combined with the mapping-table lookups (always present), the suggestion surface is richer.
- **Drift detection:** HDC-based drift catches subtle drift that explicit threshold checks miss. Threshold-based drift (always present) catches the gross cases; HDC adds nuance.

When LLM is enabled:

- **Natural language generation.** Generate placeholder copy, expand outlines, suggest variations. Author still curates; LLM accelerates the first draft.
- **Conversational interfaces.** Authoring through dialogue rather than forms. Available when configured; forms remain the deterministic fallback.
- **Free-form structural reasoning.** Some authoring decisions benefit from open-ended generation. The LLM proposes; the author decides; the substrate enforces.

## Quality through curation, not AI evaluation

A pattern across all these mechanisms: human curation produces the rules; deterministic mechanisms enforce them. This is the inverse of AI-driven approaches that try to learn rules from examples.

The curation work is real and substantial:

- Orientation taxonomies (what objectives, audiences, domains, compliance regimes exist).
- Mapping tables (which primitives serve which objectives, which primitives are appropriate for which audiences).
- Reference corpora (which sites are exemplary, what their density properties are).
- Placeholder dictionaries (what generic language to detect).
- Cliché patterns (what structural patterns indicate emptiness).
- Trait hierarchy (what traits exist, what their contracts are).
- Doctrine rules (what rules apply, with what rationales).
- Style packs (what aesthetic ranges are supported).

This is design work, taxonomy work, editorial work. It requires people with relevant expertise — design taste for the style packs, editorial judgment for the cliché patterns, security expertise for the security traits, accessibility expertise for the accessibility traits, compliance expertise for the compliance mappings. The substrate's quality is the curation's quality.

The mechanical enforcement is comparatively easy. Once the curated artifacts exist, writing the Forge phase that reads them and applies them is straightforward Rust.

## Honest trade-offs

Without AI involvement, some things are harder:

- **Generation requires explicit specifications.** Claude cannot be asked to "make a hero that fits the brand" and have AI evaluate brand-fit. Instead, the brand declaration is explicit — specific tokens, specific primitive variants permitted, specific voice constraints. Claude composes from the explicit specification; the audit verifies adherence.
- **Subtle judgments are not automated.** "Is this writing too generic" is harder without AI evaluation. The substrate detects explicit clichés through pattern matching; it cannot easily detect prose that is technically not clichéd but still feels generic. Human review catches what mechanical detection misses.
- **Personalization is constrained to explicit declarations.** Without AI learning user preferences, personalization is rule-based: declared audience, declared context, declared preference.
- **Recommendation requires explicit mappings.** "Suggest primitives that fit this objective" works by lookup against curated tables. New objectives require new mapping entries.

These trade-offs are real but they are not necessarily losses for the sovereignty-conscious audience. The deterministic mechanisms are predictable, debuggable, auditable, and do not depend on infrastructure that could change or fail or behave unexpectedly. The sovereignty value of "the substrate's behavior is fully determined by its declared rules and curated tables" is significant — there is no probabilistic component that could behave differently across sessions, no model that could regress, no cloud dependency that could fail.

## How customers see this

The customer-facing surface is consistent. Whether AI is augmenting or not, the platform works the same way at the user level. The differences are in richness of recommendations, sophistication of similarity matching, depth of policy evaluation — but never in whether the platform functions correctly.

Tenants opting out of AI see fewer recommendations, simpler similarity checks, more reliance on explicit human curation. Their sites build, deploy, audit, ship correctly. They miss subtle suggestions AI would have made; they get a more predictable platform.

Tenants opting into AI see richer recommendations, semantic similarity matching, soft-rule policy evaluation. Their experience is enhanced.

Both tenants see the same correctness floor. Both tenants' sites pass the same audit gates. Both tenants have access to the same primitives, the same themes, the same deployment targets, the same compliance regimes. The differentiation is in the augmentation layer, not in the substrate.

## How AI agents apply this

When Claude (or Gemini, or any other AI agent) is implementing or modifying substrate capabilities, the rule is sequential:

1. The deterministic implementation comes first. Every capability gets its deterministic baseline before any AI augmentation is considered.
2. AI augmentation is added only after the baseline works and is tested.
3. The augmentation lives behind the trait abstraction, never embedded directly in calling code.

When Claude is using the substrate — building sites, authoring content, debugging — the platform's behavior is the deterministic behavior. If AI is providing additional signal, Claude can use it; if it is not available, the baseline mechanisms are sufficient.

Reaching for AI augmentation when the baseline would suffice is overreach. Reaching for the baseline first and only invoking AI when it adds value is correct discipline.

The doctrine surfaces this in `AGENTS.md` across every PlausiDen repo. The agent internalizes the pattern through repeated exposure and through code review feedback when the pattern is violated.

## Cost / benefit accounting

The cost is real: every augmentable capability requires implementing both the deterministic baseline and the AI augmentation. That is roughly 1.5x to 2x the implementation work per capability compared to AI-only. The deterministic baseline is the more important half because it has to work correctly without any safety net.

The benefit is also real: the platform ships and works correctly without AI being production-ready. Customers get a sovereign deterministic substrate they can audit and trust. AI improves independently and rolls out gradually as it matures. The platform's behavior under AI failure is graceful degradation, not catastrophic failure. Customers can opt in or out per their values.

For commercial scope, this architecture is the strongest position. It says: "the platform you are paying for works deterministically and verifiably; AI is a value-add that makes it work better in specific ways, but you are not depending on it to function." Customers who care about sovereignty hear that as a strong commitment. Customers who care about productivity hear that AI augmentation is available when they want it. The same architecture serves both audiences.

## Connection to the rest of the doctrine

- **Substrate Discipline (`SUBSTRATE_DISCIPLINE.md`):** the substrate is the only path. This doctrine ensures the substrate stays the foundation rather than AI becoming load-bearing.
- **AVP-2 STILL BROKEN default:** the deterministic baseline can be proven innocent through traditional verification; AI cannot. Depending on AI would violate the STILL BROKEN default.
- **Backward-compatibility version discipline:** the deterministic baseline is the renderability guarantee. AI augmentation matures independently.
- **Trait system + doctrine:** the trait system enforces contracts deterministically. AI cannot evaluate "does this primitive satisfy SensitiveInput"; the audit phases verify each property mechanically.
- **N-orientation substrate:** all 11 orientations are enforced through deterministic mechanisms — typed manifest declarations, mapping table lookups, schema validation. AI augments specific orientations (similarity, recommendation) without becoming load-bearing for any.
- **Tool-starvation anti-pattern:** the same discipline that pushes Claude to reach for substrate tools over generic ones pushes Claude to reach for the deterministic baseline over AI augmentation.

## What this supersedes

The earlier memory `feedback_lfi_as_core_llm_as_peripheral` framed LFI as the brain with LLM as peripheral. That framing was too LFI-centric. This doctrine inverts it: the deterministic mechanisms are the substrate; the substrate is the platform; LFI and LLM are augmentations that improve specific outcomes when present.

The earlier framing assumed LFI's maturity to ship the platform. This doctrine ships the platform on the deterministic baseline and lets LFI mature with real but bounded responsibilities. AI's commercial story becomes "additional value" rather than "the entire platform's brain."

## Status

Adoption tier per `tiers/`:

- **Tier 1 (existence):** the doctrine is written and referenced. ✓ (this file).
- **Tier 2 (failure resilience):** every augmentable capability has the trait abstraction; CI verifies AI-disabled scenarios pass. PENDING (#187, #189).
- **Tier 3 (adversarial security):** the discipline survives adversarial pressure (productivity-vs-quality tension, AI hype, "just use the model" temptation). ONGOING — the practice is the verification.
- **Tier 4 (UX adversarial):** AI-disabled tenants do not feel like second-class users. ONGOING.
- **Tier 5 (integration):** every AI-touching crate in the workspace uses the trait abstraction pattern; no direct LFI/LLM imports outside designated augmentation impls. PENDING (#188).
- **Tier 6 (meta-validation):** the doctrine itself is auditable. ✓ (this document is normative; deviations are tracked exceptions per `EXCEPTIONS.md`).

When all six tiers reach checkmark status, the doctrine moves from `experimental` to `stable` in the AVP-Doctrine versioning per `CHANGELOG.toml`.
