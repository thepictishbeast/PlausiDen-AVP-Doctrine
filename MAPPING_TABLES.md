# MAPPING_TABLES.md

Curation workflow for the cross-orientation mapping tables that drive substrate decisions across the 12 N-orientations. Mapping tables encode authoritative associations between orientation axes (e.g., `domain="healthcare"` ↔ `compliance="hipaa,gdpr"`) — they are NOT derived, they are **curated**.

> Per `[[n-orientation-substrate]]` + `N_ORIENTATION_SUBSTRATE.md`: mapping tables are how the substrate translates between orientation axes. The doctrine database tells the substrate *what rules exist*; mapping tables tell the substrate *which combinations of axes imply which others*.

> Authored to close `#197 [orient-v9]`. Companion to `N_ORIENTATION_SUBSTRATE.md` (axes) + `TRAIT_DAG.md` (capabilities) + the doctrine rule database. Per `[[manifest-layer-is-the-keystone]]`: mappings project through the manifest, queryable.

---

## What is a mapping table?

A typed projection that says: *"when axis A has value X, axis B is implied to have value Y (or set of values)"*.

Examples:

| Source axis | Source value | Target axis | Implied values |
|-------------|--------------|-------------|----------------|
| Domain      | healthcare   | Compliance  | hipaa, gdpr, wcag-2.1-aa |
| Domain      | finance      | Compliance  | pci-dss-4, gdpr, ccpa, soc2-type-ii |
| Domain      | voting       | Compliance  | state-vote-acts, wcag-2.2-aaa |
| Objective   | enable_payment | Risk      | tier-6-mutation (minimum) |
| Objective   | collect_consent_with_legal_basis | Compliance | gdpr, ccpa |
| Sovereignty | anonymous    | Compliance  | gdpr-recital-26 (anonymized data not subject) |
| Sovereignty | local-only   | Resource    | network-frugal (implies) |
| Audience    | regulator    | Outcome     | audit_chain_verified, doctrine_citation_resolved |
| Audience    | end_user     | Accessibility | wcag-2.1-aa (minimum) |
| Risk        | tier-6-mutation | Resource | cpu-bounded (proptest budget caps) |
| Lifecycle   | experimental | Risk        | tier-3-functional (minimum trial gate) |
| Lifecycle   | stable       | Risk        | tier-6-mutation (minimum production gate) |
| Compliance  | hipaa        | Sovereignty | private (PHI requires) |
| Compliance  | pci-dss-4    | Sovereignty | cleartext-forbidden (mandatory) |

Mapping tables are **directional** — `Domain→Compliance` and `Compliance→Sovereignty` are separate entries, not derivable from a single bidirectional relation. The substrate composes them by chained query.

---

## Why curated, not derived?

1. **Domain knowledge is not algorithmic.** That `healthcare` implies HIPAA is a regulatory + jurisdictional fact, not a derivable property of the entity. Encode it explicitly so it can be audited + reviewed + signed.

2. **Multi-jurisdictional nuance.** `healthcare` in a US tenant implies HIPAA; in an EU tenant it implies GDPR Article 9 (special category data). The mapping table carries a `jurisdiction` qualifier when needed.

3. **Versioned + signed.** Per `[[backward-compat-version-discipline]]`: a mapping change is a doctrine change. Versioned + signed via Ed25519 + ML-DSA dual.

4. **Auditable in legal review.** Compliance officers can read the TOML directly. Derivation logic in Rust is opaque to non-engineers.

5. **Per `[[deterministic-first-lfi-optional]]`**: mappings are deterministic enums; AI may *suggest* mappings during curation but never *decide* them.

---

## File layout

```
PlausiDen-AVP-Doctrine/
  mappings/
    SCHEMA.md                         # this file's companion (rule contract)
    domain-to-compliance.toml         # Domain → Compliance set
    domain-to-required-traits.toml    # Domain → required trait declarations
    objective-to-primitives.toml      # Objective → Loom primitive set
    objective-to-risk.toml            # Objective → minimum Risk tier
    audience-to-primitives.toml       # Audience → primitive set
    audience-to-accessibility.toml    # Audience → Accessibility floor
    sovereignty-to-forbidden.toml     # Sovereignty → forbidden capabilities
    sovereignty-to-resource.toml      # Sovereignty → Resource implications
    risk-to-required-tests.toml       # Risk tier → AVP-2 test surface
    compliance-to-required-phases.toml # Compliance → Forge phase set required
    lifecycle-to-risk.toml            # Lifecycle stage → Risk floor
    cross-jurisdictional.toml         # jurisdiction × axis qualifiers
```

Each file is a curated TOML with entries following the schema below.

---

## Schema

```toml
# mappings/<source>-to-<target>.toml
[meta]
schema_version    = "1.0"
mapping_id        = "domain-to-compliance"
source_axis       = "domain"
target_axis       = "compliance"
relation          = "implies"          # implies | forbids | requires_minimum
description       = "Each domain implies its baseline compliance posture..."

[[entry]]
source            = "healthcare"
target            = ["hipaa", "gdpr", "wcag-2.1-aa"]
jurisdiction      = "global"           # optional; "us" / "eu" / etc.
rationale         = """
HIPAA applies in US deployments handling PHI. GDPR applies in EU
deployments handling health data (Article 9 special category).
WCAG 2.1 AA is the substrate floor for any consumer-facing site.
"""
references        = ["45 CFR §164", "GDPR Article 9", "WCAG 2.1"]
related_doctrine  = ["sec-001", "a11y-003"]
lifecycle         = "stable"
signatures        = [
  "ed25519:<base64url>",
  "ml-dsa:<base64url>",
]
signed_by         = "paul"
signed_at         = "2026-05-20T16:00:00Z"

[[entry]]
source            = "finance"
target            = ["pci-dss-4", "gdpr", "ccpa", "soc2-type-ii"]
# ...
```

Mandatory fields per entry: `source`, `target`, `rationale`, `references`, `signatures`. Without signatures the entry is rejected at parse.

---

## Curation workflow

Adding or modifying a mapping is a **doctrine change** — heavyweight, reviewed, signed.

### 1. Identify the gap

Triggers:
- A site declares a new domain (e.g., `Domain=hospitality`) that isn't in `domain-to-compliance.toml`.
- Legal review surfaces a new regulatory regime (e.g., DORA for EU financial services).
- An AVP-2 retrospective surfaces that a high-risk operation lacked the proper Risk-tier floor.

### 2. Author the entry

Following the schema above. Each new entry must include:
- Source + target values from the canonical enums (per `N_ORIENTATION_SUBSTRATE.md`)
- Rationale (multi-paragraph; will be read by lawyers, regulators, future engineers)
- References (authoritative external sources: CFR / GDPR articles / WCAG criteria / RFCs / ISO standards)
- Cross-references to related doctrine rules

### 3. Open ADR

Mapping changes carry an Architectural Decision Record. Per AVP-Doctrine rule `docs-004`: every architectural decision has an ADR. The ADR captures: alternatives considered, why this mapping over others, impact on existing tenants.

### 4. Doctrine review + signing

- One reviewer per affected jurisdiction (legal-aware).
- One reviewer per affected substrate concern (engineering).
- Final signatures (Ed25519 + ML-DSA dual) from a doctrine maintainer.

### 5. CI validation

The mapping CI workflow (analogous to `substrate-discipline.yml`) walks every `mappings/*.toml` and asserts:
- Schema parses.
- Source + target values resolve in the canonical enums.
- Signatures verify.
- No duplicate `(source, jurisdiction)` pairs within a mapping.
- Every `related_doctrine` rule id resolves in the rule database.
- Lifecycle transitions follow `VERSION_DISCIPLINE.md`.

### 6. Merge + publish

Once merged, mapping entries are queryable via `forge mapping query`:

```bash
forge mapping query --source domain --target compliance --value healthcare
forge mapping query --source objective --target risk --value enable_payment
```

JSON output for AI-agent consumption (per `[[priority-architectural-first-and-cross-ai]]`):

```jsonc
{
  "status": "ok",
  "mapping_id": "domain-to-compliance",
  "source": { "axis": "domain", "value": "healthcare" },
  "target": { "axis": "compliance", "values": ["hipaa", "gdpr", "wcag-2.1-aa"] },
  "jurisdiction": "global",
  "lifecycle": "stable",
  "rationale": "...",
  "references": [...]
}
```

---

## Cross-orientation query composition

Mapping tables compose for richer queries. Example: *"For a healthcare tenant in the US, what doctrine rules must each Loom primitive cite?"*

```text
1. domain="healthcare" → compliance=["hipaa", "gdpr", "wcag-2.1-aa"]
                          (via domain-to-compliance, jurisdiction=us)
2. compliance="hipaa" → required_phases=["audit_log_verifier", "privacy_retention"]
                          (via compliance-to-required-phases)
3. compliance="wcag-2.1-aa" → required_traits=[
     "MobileFriendly", "ScreenReaderAccessible", "RespectsReducedMotion", ...]
                          (via compliance-to-required-traits — implied)
4. UNION over the chain → primitive must declare all of the above traits +
   any phase enforcing the union must cite all applicable rules.
```

The substrate composes the chain via `forge mapping resolve --tenant <id>` — no human computation needed once the curated tables are in place.

---

## Initial mapping tables (must land at v1.0)

These nine are the priority-1 set; others can land additively:

### `domain-to-compliance.toml` (12+ entries)

Each canonical Domain value (healthcare / finance / hospitality / voting / education / ecommerce / legal / journalism / philanthropy / ai_research / agnostic / …) maps to its baseline Compliance set.

### `domain-to-required-traits.toml` (12+ entries)

Domain → trait floor (e.g., healthcare requires `ScreenReaderAccessible` + `LowVisionSupported`, voting requires `KeyboardOperable`+`OfflineCapable`).

### `objective-to-primitives.toml` (~20 entries)

Each canonical Objective → recommended Loom primitive(s). Drives `forge suggest-primitive --objective <name>`.

### `objective-to-risk.toml` (~20 entries)

Each Objective → minimum AVP-2 Risk tier. `enable_payment` → tier-6 minimum, `display_content` → tier-3 minimum, etc.

### `audience-to-accessibility.toml` (7 entries)

Each Audience (end_user / operator / regulator / partner_developer / ai_agent / legal_archive / internal_engineer) → accessibility floor.

### `sovereignty-to-forbidden.toml` (11 entries)

Each Sovereignty value → forbidden capabilities (e.g., `anonymous` forbids tracking pixels, `local-only` forbids any network call, `tor-compatible` forbids CDN dependencies).

### `risk-to-required-tests.toml` (10 entries)

Risk tier → AVP-2 test surface. Tier-6 requires mutation testing; tier-8 requires TLA+/Lean4.

### `compliance-to-required-phases.toml` (~10 entries)

Each Compliance regime → Forge phase set that must be in the build pipeline for that regime to be honored (e.g., `pci-dss-4` requires `audit_log_verifier` + `csp` strict + `external_assets` strict).

### `lifecycle-to-risk.toml` (5 entries)

Lifecycle stage → Risk floor: experimental → tier-3 minimum; beta → tier-4; stable → tier-6.

---

## Anti-patterns

| ❌ Don't | ✅ Do |
|---------|------|
| Derive mappings from string heuristics or AI suggestion | Curate explicitly with signed entries (per `[[deterministic-first-lfi-optional]]`) |
| Hard-code domain→compliance maps in Rust source | Curated TOML — auditable by non-engineers + signed |
| Skip the rationale + references fields | Rejected at parse; legal review needs the source citations |
| Sign with Ed25519 only | Dual-sign (Ed25519 + ML-DSA) per `[[super-society-tech-stack]]` |
| Add a mapping without an ADR | Mapping changes are architectural; ADR is mandatory |
| Allow ambiguous `(source, jurisdiction)` duplicates | Schema rejects; one entry per source-jurisdiction pair |
| Drift between mapping rationale and references | CI verifies references field is non-empty + each entry's lifecycle is declared |

---

## Implementation arc

This design (#197) gates the actual mapping-table population + querying:

| Task | Deliverable |
|------|-------------|
| **#197** (this) | Workflow design + 9 initial table specs + schema + curation steps |
| Follow-on capability requests | Author the 9 initial tables (each is its own PR + signing event) |
| Forge subcommand | `forge mapping query` + `forge mapping resolve --tenant <id>` (typed CLI) |
| CI workflow | `mapping-discipline.yml` walks mappings/*.toml + verifies signatures + schema + cross-references |
| MCP tool | `mcp/tools/forge_mapping_query.json` for cross-AI consumption |
| Manifest projection | mapping entries surface in the manifest alongside doctrine rules + traits + orientations |

---

## See also

- `N_ORIENTATION_SUBSTRATE.md` — the 12 axes mapping tables connect
- `TRAIT_DAG.md` — traits referenced from mapping entries (e.g., compliance → required traits)
- `VERSION_DISCIPLINE.md` — mapping-entry versioning + signed migration
- `DETERMINISTIC_FIRST.md` — mappings are curated, never AI-derived
- `SUBSTRATE_DISCIPLINE.md` — mapping additions are substrate changes
- `[[n-orientation-substrate]]` memory — 12 axes founding insight
- `[[manifest-layer-is-the-keystone]]` memory — mappings project through manifest
- `[[super-society-tech-stack]]` memory — dual-signature requirement
