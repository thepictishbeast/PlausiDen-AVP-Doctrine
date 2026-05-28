# N_ORIENTATION_SUBSTRATE.md

The substrate organizes around **N orthogonal orientations** simultaneously. Every substrate entity (Loom primitive, CMS section, Forge phase, Crawler detector, *-core type, audit chain entry, manifest projection) carries metadata along **all** orientations — none of which is privileged as "the" type system. Object identity is one orientation among many.

> Per `[[n-orientation-substrate]]` doctrine: beyond object-orientation, substrate organizes around 12 orthogonal axes (object / objective / outcome / audience / domain / lifecycle / compliance / risk / resource / accessibility / sovereignty / temporal). All simultaneous, all queryable, manifest carries them all.

> Authored to close `#183 [orient-v1]`. Tasks `#190-#197` each implement one orientation following this design.

> Companion to `TRAIT_DAG.md` (which classifies entity *capabilities*) and the doctrine rule database (which classifies *enforced standards*). N-orientations classify entities along *axes of meaning* — what they are, what they're for, who they serve, what they bind to, etc.

---

## The 12 orientations

| # | Orientation | Question it answers | Example values |
|---|-------------|---------------------|----------------|
| 1 | **Object** | What is this entity? | LoomPrimitive::Hero, ForgePhase::Contrast, CmsSection::PullQuote |
| 2 | **Objective** | What is its purpose? | reduce_signup_friction, communicate_trust, enable_payment, surface_alarm |
| 3 | **Outcome** | What does it cause / aim to cause? | user_completes_signup, page_lcp_under_2.5s, audit_chain_verified |
| 4 | **Audience** | Who is the consumer? | end_user, operator, partner_developer, regulator, ai_agent |
| 5 | **Domain** | What topic / vertical? | healthcare, finance, hospitality, voting, education, ecommerce |
| 6 | **Lifecycle** | What evolution stage? | experimental, beta, stable, deprecated, retired |
| 7 | **Compliance** | What regulatory regimes apply? | gdpr, ccpa, hipaa, pci-dss-4, soc2, wcag-2.1-aa, dora |
| 8 | **Risk** | What AVP-2 tier? (per [[avp2-tiers]]) | tier-1-trivial, tier-3-functional, tier-6-mutation, tier-9-adversarial |
| 9 | **Resource** | What budget tier consumes / produces? | cpu-cheap, memory-bounded, network-frugal, carbon-budgeted |
| 10 | **Accessibility** | What a11y posture? | wcag-2.1-aa, wcag-2.2-aaa, screen-reader-first, keyboard-first |
| 11 | **Sovereignty** | What PSA posture? | anonymous, private, local-only, tor-compatible, offline-capable |
| 12 | **Temporal** | What time-bound behaviors? | session-scoped, request-scoped, daily, monthly, archival, ephemeral |

These 12 are **orthogonal** — a single entity carries a value (or set of values) for each. None subsumes the others. None can be derived from another. Compare with single-orientation systems (pure OOP = Object only; pure DDD = Domain only) — they describe *aspects*, not totalities.

---

## Design principles

1. **All orientations are simultaneously projected.** The manifest carries values for every orientation per entity. Querying by any combination is a first-class operation.

2. **No privileged axis.** Object identity is a useful index, not a hierarchical root. A query like *"every Sovereignty=anonymous + Compliance=gdpr + Audience=end_user entity"* must work as well as *"every LoomPrimitive::Hero"*.

3. **Multi-value where applicable.** Compliance and Audience are typically multi-valued (a payment form is gdpr+ccpa+pci-dss+wcag); Object and Lifecycle are typically single-valued.

4. **Closed enumeration per axis.** Each orientation has a canonical closed enum (extensible via doctrine-rule + capability-request, never ad-hoc).

5. **Default = strict-required.** Every entity declares values for every orientation. Omission produces strict findings unless the orientation explicitly allows "not applicable" (declared, not implicit).

6. **Deterministic enumeration.** Per `[[deterministic-first-lfi-optional]]`: every orientation value is a discrete enum; no AI-inferred categories. LFI may *suggest* values during authoring; humans + audit confirm.

7. **Cross-orientation queries are typed.** `manifest.query(Sovereignty::Anonymous + Audience::EndUser)` is a typed projection — no string concatenation, no ad-hoc filters.

8. **Versioned per axis.** Each orientation's enum is versioned per `[[backward-compat-version-discipline]]`. Adding a value is additive; removing is operator-action; renaming is auto-migration.

---

## Orientation 1 — Object

The Rust type / module identity. **Single-valued.**

```text
Object enum:
  Loom.Primitive.<Variant>          e.g. Loom.Primitive.Hero, Loom.Primitive.SplitHero
  Loom.Token.<Category>             e.g. Loom.Token.Color, Loom.Token.Space
  Loom.Theme.<Slug>                 e.g. Loom.Theme.AmoledDark, Loom.Theme.LightDefault
  CMS.Section.<Variant>             e.g. CMS.Section.PullQuote
  CMS.Page                          (page-level)
  Forge.Phase.<Name>                e.g. Forge.Phase.Contrast, Forge.Phase.SubstratePurity
  Forge.Subcommand.<Name>           e.g. Forge.Subcommand.Orient, Forge.Subcommand.DoctrineQuery
  Core.<Crate>.<Type>               e.g. Core.PrivacyCore.RetentionPolicy
  Crawler.Detector.<Axis>           e.g. Crawler.Detector.ViewportOverflow
  Crawler.Journey                   (journey-level)
  AuditChainEntry                   (observability)
  ManifestProjection                (manifest itself)
  DoctrineRule                      (doctrine entry)
  Trait                             (trait entry)
  MCPTool                           (mcp/tools/*.json)
```

Source: the rust crate / module + the manifest declares its Object slug. One-to-one mapping.

---

## Orientation 2 — Objective

What the entity is *for*. **Single-valued** in canonical form; aliases allowed.

```text
Objective enum (representative — extensible via doctrine + capability-request):
  reduce_signup_friction
  enable_payment
  collect_consent_with_legal_basis
  communicate_trust
  surface_alarm
  prove_provenance
  enforce_access_control
  enable_navigation
  display_content
  capture_metric
  audit_artifact
  publish_doctrine
  schedule_recurring_action
  detect_regression
  validate_typed_input
  render_typed_output
  configure_substrate
  declare_capability
  encode_belief
  request_human_review
```

Each Loom primitive declares its primary objective. A `Hero` has objective `display_content` plus secondary `communicate_trust`. A `Form` has objective `validate_typed_input` plus `enable_payment` if it's a payment form.

---

## Orientation 3 — Outcome

What the entity *causes* / aims to cause. **Multi-valued.** Distinct from Objective: the objective is the *goal*, the outcome is the *measurable effect*.

```text
Outcome enum (representative):
  user_completes_signup
  page_lcp_under_2.5s            (perf)
  page_cls_under_0.1             (perf)
  no_contrast_violations         (a11y)
  no_keyboard_traps              (a11y)
  audit_chain_verified           (observability)
  no_phantom_buttons             (substrate)
  no_orphan_bypasses             (substrate)
  doctrine_citation_resolved     (doctrine)
  capability_request_filed       (substrate)
  pii_minimization_applied       (privacy)
  user_session_ends_within_30m   (sovereignty)
```

Outcomes are the *test surface* — every entity contributes to (or detracts from) a measurable platform outcome. Audit phases that enforce an outcome cite it; primitives that contribute cite it.

---

## Orientation 4 — Audience

Who consumes the entity. **Multi-valued.**

```text
Audience enum:
  end_user                     (the human site visitor)
  operator                     (the operator of the substrate / a tenant admin)
  partner_developer            (third-party integrating against APIs)
  regulator                    (compliance officer / auditor)
  ai_agent                     (Claude / Gemini / other agents)
  legal_archive                (long-term compliance retention)
  internal_engineer            (PlausiDen contributors)
```

A `Hero` primitive's audience is typically `end_user`. A `forge orient` command's audience is `ai_agent + internal_engineer`. A doctrine rule's audience is `internal_engineer + regulator + ai_agent`.

---

## Orientation 5 — Domain

The topic / vertical the entity binds to. **Multi-valued.** Most substrate entities are domain-agnostic; site-specific configurations bind to a specific domain.

```text
Domain enum (representative):
  healthcare
  finance
  hospitality
  voting
  education
  ecommerce
  legal
  journalism
  philanthropy
  ai_research
  agnostic                    (substrate-general; default for Loom primitives)
```

Per `prim-012`: substrate primitives MUST declare `Domain=agnostic`. Site-specific composition can bind a domain. Cross-domain queries like "every Domain=healthcare entity" surface the entity set that touches PHI.

---

## Orientation 6 — Lifecycle

Where the entity sits in its evolution. **Single-valued.**

```text
Lifecycle enum (parallel to doctrine rule + trait lifecycle):
  experimental                (trial; advisory enforcement)
  beta                        (functional but unstable; gated)
  stable                      (binding; strict enforcement)
  deprecated                  (sunset scheduled; replaced_by required)
  retired                     (removed; archived for citation)
```

Per `[[backward-compat-version-discipline]]`: every entity carries its lifecycle. Promotion / demotion is a versioned migration.

---

## Orientation 7 — Compliance

The regulatory regimes the entity must conform to. **Multi-valued.**

```text
Compliance enum (representative; extensible per jurisdiction):
  gdpr           EU General Data Protection Regulation
  ccpa           California Consumer Privacy Act
  hipaa          US Health Insurance Portability and Accountability Act
  pci-dss-4      Payment Card Industry Data Security Standard v4
  soc2-type-ii   Service Organization Control 2 Type II
  iso-27001      ISO/IEC 27001 Information Security
  iso-25010      ISO/IEC 25010 Software Quality
  iso-40500      ISO/IEC 40500 (WCAG 2.0 reference)
  wcag-2.1-aa    Web Content Accessibility Guidelines 2.1 AA
  wcag-2.2-aaa   WCAG 2.2 AAA (strongest a11y target)
  dora           EU Digital Operational Resilience Act
  cra            EU Cyber Resilience Act
  state-vote-acts
  none-applicable (declared, never implicit)
```

Per `prim-001`: every primitive declares its compliance posture. A payment form is `gdpr + ccpa + pci-dss-4 + wcag-2.1-aa`. A doctrine rule is `iso-25010 + iso-27001`.

---

## Orientation 8 — Risk

AVP-2 tier per `[[avp2-tiers]]`. **Single-valued.**

```text
Risk enum (AVP-2 tier ladder):
  tier-1-trivial       smoke test
  tier-2-unit          unit tests
  tier-3-functional    integration tests
  tier-4-property      proptest at boundaries
  tier-5-fuzz          cargo-fuzz / afl
  tier-6-mutation      cargo-mutants
  tier-7-concurrent    loom/shuttle-style
  tier-8-formal        TLA+/Lean4-verified
  tier-9-adversarial   red-team gated
  tier-10-economic     incentive-compatibility proven
```

Per AVP-2 protocol: every entity declares its risk tier. The substrate refuses to ship a `Forge.Subcommand` below `tier-3`. Critical paths (auth / payment / consent) must reach `tier-6` minimum.

---

## Orientation 9 — Resource

Cost / budget envelope. **Multi-valued.**

```text
Resource enum:
  cpu-cheap          O(1) or O(log n) per invocation
  cpu-bounded        O(n) bounded by an enum or small list
  cpu-expensive      requires explicit budget declaration
  memory-bounded     allocates ≤ declared bytes
  memory-streaming   constant memory for arbitrary input
  network-frugal     ≤ 1 round-trip per operation
  network-bursty     batched but explicit budget
  carbon-budgeted    declares CO2e per invocation (perf-006)
  disk-frugal        ≤ declared bytes written
  disk-archival      append-only, no reads
```

Resource declarations are enforced by Forge phase `carbon_budget` + `bundle_size` + similar. Per `perf-001`..`perf-008`.

---

## Orientation 10 — Accessibility

A11y posture. **Single-valued** at the target level, multi-valued for capability list.

```text
Accessibility enum (target levels):
  wcag-2.1-a         minimum legal floor in most jurisdictions
  wcag-2.1-aa        substrate default (rule a11y-003)
  wcag-2.1-aaa       aspirational; opt-in per primitive
  wcag-2.2-aa        substrate target for net-new primitives
  wcag-2.2-aaa       strongest

Accessibility capabilities (multi-valued; cross-cuts trait DAG):
  screen-reader-first    designed for non-visual primary
  keyboard-first         designed for non-pointer primary
  cognitive-load-low     reading-level + decision-density bounded
  voice-control-aware    targets sized for voice control hit rates
  motor-impairment-aware no hover-only affordances
```

Per rule `a11y-001` + `a11y-003`. Bridges to trait `MobileFriendly` + `Focusable` + `KeyboardOperable` + `ScreenReaderAccessible`.

---

## Orientation 11 — Sovereignty

The PSA differentiator (privacy / security / anonymity). **Multi-valued.**

```text
Sovereignty enum:
  anonymous            no identifier links to a person (linkability bounded)
  pseudonymous         identifier per session, no cross-session link
  identified           identifier declared; consent required
  private              data never leaves the substrate without explicit consent
  local-only           data never persists to disk
  ephemeral            data expires per declared TTL
  tor-compatible       reachable over .onion; no clearnet linkage required
  offline-capable      functions without network (Service Worker / local data)
  pq-secure            post-quantum (ML-DSA + ML-KEM)
  cleartext-forbidden  never transmits unencrypted
  zero-knowledge       proves a claim without revealing the underlying data
```

The PlausiDen differentiator per `[[super-society-tech-stack]]`: every substrate entity scores on the sovereignty axis simultaneously with fast / reliable / robust / secure. New doctrine rules (sovereignty domain) emerge from this axis.

---

## Orientation 12 — Temporal

Time-binding behaviors. **Multi-valued.**

```text
Temporal enum:
  session-scoped       lives for one user session
  request-scoped       lives for one HTTP request
  build-scoped         materialized at forge build time
  daily-recurring      regenerated daily (e.g. token rotation)
  monthly-recurring    monthly cycle
  archival             long-term retention (compliance)
  ephemeral            destroyed immediately after use
  versioned-immutable  every change produces a new version; old retained
  monotonic            increases monotonically (audit chain, sequence)
  bounded-history      keeps last N (declared)
```

Per `iso-25010` reliability + `[[backward-compat-version-discipline]]`.

---

## The 12-axis manifest projection

Every substrate entity emits a canonical 12-axis projection into the manifest:

```jsonc
{
  "$schema": "https://plausiden.com/schemas/orientation-manifest-v1.json",
  "version": "1.0.0",
  "entity": "Loom.Primitive.Hero",
  "orientations": {
    "object":        "Loom.Primitive.Hero",
    "objective":     "display_content",
    "outcome":       ["page_lcp_under_2.5s", "no_contrast_violations"],
    "audience":      ["end_user"],
    "domain":        ["agnostic"],
    "lifecycle":     "stable",
    "compliance":    ["wcag-2.1-aa", "iso-25010"],
    "risk":          "tier-4-property",
    "resource":      ["cpu-cheap", "network-frugal", "carbon-budgeted"],
    "accessibility": "wcag-2.1-aa",
    "sovereignty":   ["private"],
    "temporal":      ["build-scoped"]
  }
}
```

The substrate refuses to render any entity that omits an axis. CI consistency check (task #196): every Rust-declared entity must have a manifest projection; every manifest projection must reference a Rust-declared entity.

---

## Cross-orientation query language

The manifest supports typed cross-orientation queries (task #196):

```text
manifest query: object="Loom.Primitive.*"
manifest query: sovereignty="anonymous" + audience="end_user"
manifest query: domain="healthcare" + risk>=tier-6
manifest query: compliance="gdpr" + lifecycle="stable"
manifest query: outcome="audit_chain_verified" + audience="regulator"
```

Returns the matching entity set. Used by:
- Forge phases: "every entity that claims compliance=hipaa — does it satisfy our HIPAA enforcement?"
- AGENTS.md: "scope rules surfacing per directory" via `forge doctrine for` (cross-orient generalization).
- Operator UI: "show me every primitive my tenant uses that touches PHI."
- Regulator export: "every entity bound to compliance=gdpr — render the inventory."

---

## Mapping tables (task #197)

The orientation system relies on curated mapping tables that translate between axes:

- `objective → primitives` — which primitives satisfy which objectives
- `audience → primitives` — which primitives serve which audiences
- `domain → compliance` — which compliance regimes follow from a domain (healthcare → hipaa+gdpr)
- `domain → required_traits` — domain implies trait-set requirements
- `risk → required_tests` — risk tier dictates AVP-2 test surface
- `sovereignty → forbidden_capabilities` — anonymous forbids tracking pixels, etc.
- `compliance → required_audit_phases` — gdpr requires consent + retention + erasure phases

These mappings are **curated**, not derived. Curation workflow:
1. PR proposes a mapping entry with rationale.
2. Doctrine + trait DAG verify consistency (e.g., a domain→compliance mapping doesn't override an entity's explicit declaration).
3. Stable mappings get a lifecycle stamp; trial mappings are experimental.

---

## Implementation arc

Authoring complete (this doc) gates these tasks:

| Task | Deliverable |
|------|-------------|
| **#183** (this) | 12-orientation design doc + JSON manifest schema + query-language sketch |
| **#190** [orient-v2] | Implement **Objective** orientation (Rust crate + manifest projection + first 20 objective values + 1 forge phase that queries by objective) |
| **#191** [orient-v3] | Implement **Compliance** orientation (GDPR / CCPA / HIPAA / PCI / SOC2 / WCAG / DORA / CRA enum + per-rule mapping) |
| **#192** [orient-v4] | Implement **Sovereignty** orientation (the PlausiDen differentiator — 11 sovereignty values + new doctrine rules domain) |
| **#193** [orient-v5] | Implement **Risk** orientation (AVP-2 tier ladder; ties to existing AVP-2 protocol) |
| **#194** [orient-v6] | Implement **Resource** orientation (per-tier budgets + cost attribution) |
| **#195** [orient-v7] | Implement **Audience + Domain + Lifecycle + Accessibility + Temporal** (batched smaller axes) |
| **#196** [orient-v8] | Cross-orientation query language + manifest projection CI consistency check |
| **#197** [orient-v9] | Mapping table curation workflow + initial mapping tables |

---

## Anti-patterns

| ❌ Don't | ✅ Do |
|---------|------|
| Treat Object as the primary index | Every orientation is queryable at first-class level |
| Use a `String` field for an orientation value | Closed enum per orientation, extensible via doctrine + capability-request |
| Skip an orientation "for now" — entity declares 9 of 12 | The substrate refuses; entity must declare all 12 (or explicit "not applicable") |
| Use AI to *infer* an orientation value | Discrete enums; AI may *suggest*, humans + audit confirm (per `[[deterministic-first-lfi-optional]]`) |
| Add a domain-specific orientation (`healthcare_phi_class`) | Express as a mapping table entry; orientations stay platform-general |
| Hand-author cross-orientation joins | Use the typed query language; ad-hoc joins drift |
| Make orientations hierarchical (Object → Domain → Audience) | All 12 are orthogonal; no axis subsumes another |

---

## See also

- `TRAIT_DAG.md` — entity *capabilities* (parallel system; orthogonal to orientations)
- `SUBSTRATE_DISCIPLINE.md` — Rule 0 (hand-coding forbidden)
- `DETERMINISTIC_FIRST.md` — LFI opt-in posture
- `doctrine/rules/SCHEMA.md` — doctrine rule format
- `[[n-orientation-substrate]]` memory — the founding insight
- `[[super-society-tech-stack]]` memory — fast + reliable + robust + secure + anonymous + private SIMULTANEOUSLY
- `[[manifest-layer-is-the-keystone]]` memory — manifest projection across the platform
- AVP-2 protocol — risk tier ladder source
