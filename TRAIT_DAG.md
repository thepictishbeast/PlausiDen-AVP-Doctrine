# TRAIT_DAG.md

Typed trait DAG for the PlausiDen substrate. Traits are **declarative contracts** carried by Loom primitives, CMS sections, Forge phases, Crawler detectors, and *-core types — orthogonal to Rust type identity, enforceable at compile time + audit time + runtime.

> Per `[[substrate-traits-and-doctrine]]` doctrine: traits are typed inheritance + composition for substrate-managed entities (DAG, audit-enforced). Pairs with the doctrine rule database to express *what* an entity satisfies vs *what* the substrate enforces.

> Authored to close `#166 [trait-v1]`. Tasks `#167-#172` (Rust impl + Loom migration + Forge phases + Crawler runtime + manifest projection + doc generation) derive from this design.

---

## Design principles

1. **Traits are orthogonal to Rust types.** A `Hero` primitive declares which traits it satisfies; the `Hero` struct does not have to inherit from those traits as Rust types. The trait declaration lives in metadata and the manifest, audited at build time.

2. **Default-required traits exist.** Every primitive in the `Visible` lineage MUST declare `MobileFriendly` + `RTLAware` + `RespectsReducedMotion` + `ThemeAware`. The substrate refuses to render a primitive that omits any of the four. (Per rule `prim-001`.)

3. **Trait cascades describe capability accretion.** `Interactive → Focusable → KeyboardOperable → ScreenReaderAccessible` is a strict chain — you cannot declare a later trait without all earlier ones.

4. **Cross-cutting traits compose orthogonally.** Sovereignty traits (`Anonymous`, `Private`, `Local`) compose with every other trait independently — `Hero` can be `MobileFriendly` AND `Anonymous` AND `ThemeAware` without conflict.

5. **Every trait names its enforcement.** Without an enforcement mechanism (Forge phase / loom-lint / Crawler axis / property test), the trait is a wish. Per rule `docs-003`.

6. **Traits are versioned.** Per `[[backward-compat-version-discipline]]`: each trait carries a version tuple; a primitive may declare which trait version it satisfies. Trait deprecations follow the four-category change taxonomy.

7. **Deterministic baseline.** Per `[[deterministic-first-lfi-optional]]`: traits and their enforcement run without LFI. LFI-augmented critics may add advisory grading on top, never gate.

---

## Trait categories (11 axes)

The DAG groups ~50 initial traits into 11 categories. The categories mirror the N-orientation substrate (`[[n-orientation-substrate]]`).

```
1. Visibility & Lifecycle    →  Renderable, Visible, ClientOnly, ServerOnly, Cacheable, Streamable
2. Interaction               →  Interactive, Focusable, KeyboardOperable, MouseOperable, TouchOperable
3. Accessibility (a11y)      →  ScreenReaderAccessible, ReducedMotionAware, HighContrastSupported, ColorBlindSafe, LowVisionSupported
4. Responsive                →  MobileFriendly, TabletFriendly, DesktopFriendly, ContainerQueryAware, OrientationAware
5. Internationalization      →  RTLAware, LocaleAware, NumberFormatAware, DateFormatAware
6. Theming                   →  ThemeAware, ColorSchemePicked, DarkModeFirst, AMOLEDOptimized
7. Security                  →  CSPCompatible, SRIVerified, NonceAware, OriginIsolated, NoEval
8. Sovereignty (PSA)         →  Anonymous, Private, Local, EphemeralByDefault, TorCompatible, OfflineCapable
9. Performance               →  CarbonBudgeted, LCPSafe, CLSStable, BundleSizeBounded, LazyLoadable
10. Reliability              →  PropertyTested, FuzzTested, RegressionFixtured, FailsClosed
11. Discipline               →  DoctrineCited, SubstrateNative, NoSiteSpecific, Manifested, Versioned
```

Total: **49 traits.** A 50th is reserved for the manifest registry itself (`SelfDescribing`), declared by the manifest crate.

---

## DAG (textual representation)

Arrows denote `requires` — declaring B requires A is already declared.

```
                          Renderable
                              │
                       ┌──────┴──────┐
                       ▼             ▼
                   Visible       ClientOnly / ServerOnly
                       │              (mutually exclusive with each other)
              ┌────────┴────────┐
              ▼                 ▼
        Interactive       Cacheable / Streamable
              │
              ▼
          Focusable
              │
              ▼
      KeyboardOperable
              │
              ▼
   ScreenReaderAccessible

  (orthogonal cross-cutting cluster, attaches to Visible-line)
  MouseOperable      ─┐
  TouchOperable      ─┤
                      ├─── compose with Interactive
                      │
  ReducedMotionAware ─┤
  HighContrastSupported ──── compose with Visible
  ColorBlindSafe     ─┘

  (responsive cluster, attaches to Visible-line)
  MobileFriendly    ─┐
  TabletFriendly    ─┤
  DesktopFriendly   ─┤
  ContainerQueryAware ── compose with Visible
  OrientationAware  ─┘

  (i18n cluster, attaches to Visible-line)
  RTLAware           ─┐
  LocaleAware        ─┤── compose with Visible
  NumberFormatAware  ─┤      (each is independent of others)
  DateFormatAware    ─┘

  (theme cluster, attaches to Visible-line)
  ThemeAware           ─┐
  ColorSchemePicked    ─┤── compose with Visible
  DarkModeFirst        ─┤      (DarkModeFirst implies ThemeAware)
  AMOLEDOptimized      ─┘      (AMOLEDOptimized implies DarkModeFirst)

  (security cluster, attaches to Renderable-line)
  CSPCompatible      ─┐
  SRIVerified        ─┤── compose with Renderable
  NonceAware         ─┤
  OriginIsolated     ─┤
  NoEval             ─┘

  (sovereignty cluster, attaches to any entity that touches user data)
  Anonymous          ─┐
  Private            ─┤── compose freely
  Local              ─┤      (Local implies Private)
  EphemeralByDefault ─┤      (EphemeralByDefault implies Private)
  TorCompatible      ─┤
  OfflineCapable     ─┘

  (performance cluster, attaches to Renderable-line)
  CarbonBudgeted     ─┐
  LCPSafe            ─┤── compose with Renderable
  CLSStable          ─┤
  BundleSizeBounded  ─┤
  LazyLoadable       ─┘

  (reliability cluster, applies to any entity)
  PropertyTested        ─┐── compose freely
  FuzzTested            ─┤      (FuzzTested implies PropertyTested)
  RegressionFixtured    ─┤
  FailsClosed           ─┘

  (discipline cluster, applies to substrate entities)
  DoctrineCited      ─┐
  SubstrateNative    ─┤── compose freely
  NoSiteSpecific     ─┤      (NoSiteSpecific implies SubstrateNative)
  Manifested         ─┤
  Versioned          ─┘
```

---

## Default-required traits per entity class

The substrate's render pipeline + audit pipeline refuses any entity that omits these. Defaults are **enforced at compile time** via the manifest declaration + at build time via Forge phases.

| Entity class | Default-required traits |
|--------------|-------------------------|
| Loom primitive (Visible) | MobileFriendly, RTLAware, RespectsReducedMotion, ThemeAware, NoSiteSpecific, Manifested, Versioned, DoctrineCited |
| Loom primitive (Interactive) | + Focusable, KeyboardOperable, ScreenReaderAccessible |
| CMS section | NoSiteSpecific, Manifested, Versioned |
| Forge phase | DoctrineCited, PropertyTested, FailsClosed |
| Crawler detector | Manifested, PropertyTested, RegressionFixtured |
| *-core typed-surface | Manifested, Versioned, PropertyTested, FailsClosed |
| Asset (image/font/etc.) | LazyLoadable (if visual), BundleSizeBounded, ThemeAware (if color-sensitive) |
| Backend declaration | OriginIsolated (cross-origin), SubstrateNative |

Reading: a `Hero` primitive (Visible + Interactive lineage) must declare **all 11** default-required traits before the substrate will render it. Omission → strict finding.

---

## Trait contract format

Each trait carries a triple parallel to doctrine rules:

```toml
[[trait]]
id              = "MobileFriendly"
category        = "responsive"
statement       = "Primitive renders correctly at 390px viewport without horizontal overflow."
requires        = ["Visible"]
enforcement     = [
  "loom-lint: skin.css uses container queries, not media queries, for primitive internals (rule prim-004)",
  "forge phase: tokens — no raw px in primitive CSS (rule prim-007)",
  "crawler axis: viewport_390_overflow_check — flag horizontal scrollbar appearance",
  "property test: primitive deserialization with default content + 390px container produces no clipping",
]
verification    = "test fixture: viewport-390 baseline snapshot"
default_for     = ["Loom.Visible.*"]
version         = "1.0.0"
lifecycle       = "stable"
```

Three mandatory fields: `statement` + `requires` + `enforcement`. Without enforcement, the trait is unenforceable and rejected at parse time (parallel to `docs-003` for doctrine rules).

---

## Trait → doctrine rule mapping (selected)

| Trait | Cited rules |
|-------|-------------|
| MobileFriendly | prim-001, prim-003, prim-004, prim-007 |
| RTLAware | prim-003 (logical properties) |
| ReducedMotionAware | prim-004, a11y-004 |
| ThemeAware | prim-001, prim-007 |
| Focusable | a11y-001, prim-008 |
| KeyboardOperable | a11y-001 |
| ScreenReaderAccessible | a11y-001, prim-008 |
| CSPCompatible | sec-001 (deny_unknown_fields at boundaries — extends to CSP-clean output) |
| SRIVerified | sec-005 |
| NonceAware | sec-005 |
| Anonymous | (new sovereignty rules; capability request before promotion) |
| Private | (same) |
| Local | (same) |
| CarbonBudgeted | perf-006 |
| LCPSafe | perf-002 |
| CLSStable | perf-003 |
| BundleSizeBounded | perf-001 |
| PropertyTested | test-002 |
| FuzzTested | test-003 |
| RegressionFixtured | test-004 |
| FailsClosed | sec-001, test-008 |
| DoctrineCited | docs-005 |
| SubstrateNative | build-007 |
| NoSiteSpecific | prim-012 |
| Manifested | build-006 |
| Versioned | (backcompat-v1 rules pending — see `[[backward-compat-version-discipline]]`) |

Gaps surface as `[trait-v???]` follow-on tasks: new doctrine rules where traits need enforcement that doesn't yet exist (e.g., sovereignty cluster).

---

## Cross-language manifest schema

Traits are projected into the manifest (per `[[manifest-layer-is-the-keystone]]`) as a single canonical JSON document consumable from Rust + WASM + Crawler:

```jsonc
{
  "$schema": "https://plausiden.com/schemas/trait-manifest-v1.json",
  "version": "1.0.0",
  "categories": ["visibility", "interaction", "a11y", "responsive", "i18n",
                 "theming", "security", "sovereignty", "performance",
                 "reliability", "discipline"],
  "traits": {
    "MobileFriendly": {
      "category": "responsive",
      "statement": "...",
      "requires": ["Visible"],
      "default_for": ["Loom.Visible.*"],
      "enforcement": [...],
      "version": "1.0.0",
      "lifecycle": "stable"
    },
    // ... 48 more ...
  },
  "default_required_by_class": {
    "Loom.Primitive.Visible":     ["MobileFriendly", "RTLAware", ...],
    "Loom.Primitive.Interactive": ["Focusable", "KeyboardOperable", ...],
    "CMS.Section":                ["NoSiteSpecific", "Manifested", "Versioned"],
    // ...
  }
}
```

The Rust trait-DAG crate (task `#167`) parses this canonical manifest at build time; no duplication of the trait definitions in Rust source.

---

## Implementation arc

The trait DAG (#166, this doc) gates these tasks:

| Task | Deliverable |
|------|-------------|
| **#166** (this) | Design doc + 49 trait definitions + DAG diagram + contract format |
| **#167** [trait-v2] | `loom-traits` + `cms-traits` + `security-traits` + `forge-traits` Rust crates (typed projection of this manifest) |
| **#168** [trait-v3] | Migrate every existing Loom primitive to declare its traits in the manifest |
| **#169** [trait-v4] | Forge audit phases for each trait whose enforcement isn't already covered |
| **#170** [trait-v5] | Crawler runtime axes for traits with runtime-DOM enforcement (e.g., MobileFriendly viewport overflow) |
| **#171** [trait-v6] | Manifest projection + CI consistency check (every trait satisfied at compile time is observed at runtime) |
| **#172** [trait-v7] | Auto-generated docs (project this manifest into a published doctrine page) |

---

## Lifecycle of a trait

```
   proposed                                    deprecated
   ────────                                    ──────────
   capability      experimental    stable      replaced_by
   request    ──→  (trial)     ──→ (binding) ──→ (or removed)
   issue           lifecycle = "experimental"   lifecycle = "deprecated"
                       │
                       │   (trial period: enforced advisory, not gate)
                       │
                       ▼
                   promoted via PR review when enforcement is proven
```

Same lifecycle states as doctrine rules per `[[backward-compat-version-discipline]]`.

---

## Reading this DAG

- Traits with **strict-default-required** status appear in every primitive's manifest entry. Their omission fails build.
- Traits with **opt-in** status are declared per-primitive when the primitive class demands them (e.g., a `Form` primitive declares `KeyboardOperable` even though `Form` isn't a default of `Visible`).
- **Implication arrows** (`Local → Private`) mean: declaring Local automatically declares Private. The manifest expands implications at parse time.
- **Mutual-exclusion** (`ClientOnly` ⊕ `ServerOnly`): only one may be declared; the parser rejects both.

---

## Anti-patterns

| ❌ Don't | ✅ Do |
|---------|------|
| Add a Rust marker-trait per substrate trait | Declare in the manifest, audit at build time — Rust types stay flat |
| Use `String` field for the trait name | Closed enum derived from the canonical manifest at build time |
| Make traits implicit ("Hero is obviously focusable") | Manifest is the source of truth; implicit declarations fail audit |
| Skip the `enforcement` field "for now" | Trait without enforcement is a wish; parser rejects (rule docs-003 parallel) |
| Add a sovereignty trait without a corresponding doctrine rule | File the rule first; lifecycle = experimental until enforcement is reliable |
| Declare `Visible` without the 8 default-required Loom traits | The render pipeline refuses; strict finding at build |
| Add a site-specific trait | Generalize first; per `NoSiteSpecific` trait itself, traits are substrate-general |

---

## See also

- `SUBSTRATE_DISCIPLINE.md` — Rule 0 (hand-coding forbidden)
- `DETERMINISTIC_FIRST.md` — LFI opt-in posture
- `doctrine/rules/SCHEMA.md` — doctrine rule format (trait triple parallels rule triple)
- `[[manifest-layer-is-the-keystone]]` memory — manifest projection across the platform
- `[[n-orientation-substrate]]` memory — the 11 categories mirror the orientation axes
- `[[substrate-traits-and-doctrine]]` memory — twin meta-substrate systems
- Implementation pointers: PlausiDen-Loom/loom-cms-render/src/lib.rs (primitives that will declare traits)
- Future doctrine: PlausiDen-AVP-Doctrine/doctrine/rules/sovereignty.toml (new domain emerging from this design)
