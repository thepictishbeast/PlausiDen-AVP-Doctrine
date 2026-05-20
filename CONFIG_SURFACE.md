# CONFIG_SURFACE.md

The three-tier configuration surface (platform / tenant / operation) that controls AI augmentation across the PlausiDen substrate. Defines schema, file locations, precedence rules, fail-closed defaults, and runtime resolution. Operators read this to know what knobs exist; the substrate refuses to start if config is malformed.

> Authored to close `#187 [determ-v4]`. Companion to `DETERMINISTIC_FIRST.md` (the doctrine; what the layers mean) and `CAPABILITY_AI_POSTURE.md` (which capabilities have AI augmentation). This doc defines *how to control them*.

> Per `[[deterministic-first-lfi-optional]]`: AI is opt-in augmentation. The configuration surface makes that opt-in explicit, layered, and fail-closed.

---

## The three layers

```
   ▲ override priority (highest wins)
   │
   │   3. Operation-level   per invocation / per request
   │      (HTTP header, CLI flag, MCP tool input)
   │
   │   2. Tenant-level      per tenant.toml
   │      (tenant operator's standing posture)
   │
   │   1. Platform-level    compile-time + deploy-time
   │      (cargo feature flag + platform.toml)
   ▼   (default: AI disabled; fail-closed)
```

A capability runs with AI augmentation **only if all three layers agree**:

1. The capability is compiled in at the platform level (cargo feature flag enabled).
2. The tenant's `tenant.toml` allows it.
3. The operation hasn't explicitly disabled it.

Disabling at any single layer short-circuits the entire stack to deterministic baseline. No layer can override another's "off."

---

## Layer 1 — Platform-level

**Where**: compile-time cargo feature flags + `platform.toml` at deployment root.

### Cargo feature flags

Each LFI/LLM integration crate has a feature flag. Default-OFF for every flag:

```toml
# crates/forge-cli/Cargo.toml (example)
[features]
default = []
lfi = ["dep:lfi-core"]
llm = ["dep:llm-core"]
```

Building without `--features lfi,llm` produces a binary with no AI dependencies whatsoever. Self-hosted sovereignty deployments build this way.

Managed deployments build with `--features lfi,llm`; tenants then toggle at layer 2.

### `platform.toml`

For builds that DO compile AI integration in, `platform.toml` carries platform-wide gates:

```toml
# /etc/plausiden/platform.toml (or <deployment>/platform.toml)
schema_version = "1.0"

[ai]
# Master switches. If false at platform level, no tenant can enable.
lfi_enabled = false
llm_enabled = false

# When AI is enabled platform-wide, set per-provider availability.
# Per [[super-society-tech-stack]]: sovereignty-conscious deployments
# may want to whitelist only providers that meet PSA constraints.
[ai.providers]
local_only        = true   # if true, only local-host LFI/LLM allowed
allow_external    = false  # whitelist external providers explicitly
allowed_externals = []     # e.g. ["anthropic", "openai"] — empty default

# Audit posture. Every AI invocation lands in the audit chain when
# this is true. Defaults to true — per [[manifest-layer-is-the-
# keystone]], invocations are platform-level events.
audit_invocations = true

[fail_closed]
# When an AI invocation cannot determine its disposition (config
# missing, parse error, signature mismatch), DEFAULT to deterministic
# baseline. This is the load-bearing safety.
on_config_error    = "deterministic"  # never "ai"; never "error"
on_provider_down   = "deterministic"
on_signature_fail  = "deterministic"
on_rate_limit      = "deterministic"
```

**Schema validation**: `forge platform validate` parses + enforces. Build refuses to start if `platform.toml` is malformed.

**Fail-closed when platform.toml is absent**: assume `lfi_enabled = false` + `llm_enabled = false`. The substrate starts; AI is off. Never an error.

---

## Layer 2 — Tenant-level

**Where**: `tenant.toml` per tenant (or `tenants/<slug>.toml` in multi-tenant deployments).

```toml
# tenant.toml (or tenants/acme-corp.toml)
schema_version = "1.0"
tenant_id = "acme-corp"
tenant_name = "Acme Corporation"

[ai]
# Tenant operator's standing posture. Even if platform allows AI,
# tenant can refuse — per [[super-society-tech-stack]] sovereignty.
lfi_enabled = true
llm_enabled = false

# Per-capability fine-grain control. Names match CAPABILITY_AI_POSTURE
# inventory categories. If unlisted, defaults to platform setting.
[ai.capabilities]
# A capabilities (augmentable):
"similarity_check"              = "lfi"           # use LFI
"content_drift_detection"       = "lfi"
"trust_safety_concern_suggest"  = "deterministic" # opt-out (no AI)
"reading_level_check"           = "lfi"
"originality_score"             = "deterministic" # tenant wants pure deterministic
"recommend_primitive"           = "lfi"
"crawler_anomaly_summary"       = "deterministic"
"annotation_grouping"           = "lfi"

# P capabilities (primarily augmented):
"natural_language_drafting"     = "llm"          # only enabled when llm_enabled=true
"conversational_authoring"      = "off"          # tenant prefers forms
"freeform_reasoning"            = "off"

# Audit + sovereignty.
[ai.sovereignty]
require_local_inference = true   # disallow any external AI calls
require_signed_models   = true   # only ML-DSA-signed models accepted
log_invocations         = true   # tenant-side audit log enabled
no_telemetry            = true   # never phone home about AI usage

# Trial / experimental opt-in.
[ai.experimental]
opt_in_experimental_capabilities = false
```

**Schema validation**: `forge tenant validate <path>` parses + enforces.

**Fail-closed when tenant.toml is absent**: defaults to deterministic. A tenant directory with no `tenant.toml` runs the platform with AI off, regardless of platform setting. The presence of `tenant.toml` is the *opt-in signal*.

---

## Layer 3 — Operation-level

**Where**: per invocation — HTTP header, CLI flag, MCP tool input.

### CLI invocation

Every Forge subcommand whose underlying capability has D/A/P posture supports flags:

```bash
forge build                              # default: tenant.toml decides
forge build --ai=deterministic           # force deterministic for this build
forge build --ai=lfi --skip-llm          # selectively enable
forge build --ai=off                     # alias for deterministic

forge audit annotation_review --ai=lfi   # per-phase override
```

The `--ai` flag accepts: `deterministic` / `lfi` / `llm` / `auto` / `off` (alias for deterministic).

`auto` resolves to the tenant.toml + platform.toml decision. `auto` is also the default when the flag is omitted.

### HTTP request

For HTTP services (loom edit serve, dynamic-mode Forge, MCP servers), invocation-level override travels in headers:

```http
X-PlausiDen-AI: deterministic
X-PlausiDen-AI-LFI: enabled
X-PlausiDen-AI-LLM: disabled
```

When operator UI exposes a "use AI for this draft" toggle, the toggle materializes as one of these headers.

### MCP tool input

MCP tools that wrap A/P capabilities expose an `ai_mode` input:

```jsonc
{
  "name": "loom_edit_draft_section",
  "inputSchema": {
    "properties": {
      "page_id": {"type": "string"},
      "section_kind": {"type": "string"},
      "ai_mode": {
        "type": "string",
        "enum": ["deterministic", "lfi", "llm", "auto", "off"],
        "default": "auto"
      }
    }
  }
}
```

---

## Precedence resolution

For any capability `C` at any invocation, the substrate resolves AI mode as:

```text
fn resolve_ai_mode(
    capability: Capability,
    platform: &PlatformConfig,
    tenant: Option<&TenantConfig>,
    operation: Option<AiMode>,
) -> ResolvedAiMode {

    // 1. Look up the capability's static posture (D / A / P).
    let posture = CAPABILITY_AI_POSTURE.get(capability);

    // 2. D capabilities are NEVER augmented, regardless of layers.
    if posture == Posture::Deterministic {
        return ResolvedAiMode::Deterministic;
    }

    // 3. Operation-level wins if it says "off" or "deterministic".
    if let Some(op) = operation {
        if matches!(op, AiMode::Off | AiMode::Deterministic) {
            return ResolvedAiMode::Deterministic;
        }
    }

    // 4. Platform-level master switch.
    let platform_allows = match posture {
        Posture::Augmentable => platform.ai.lfi_enabled,
        Posture::PrimarilyAugmented => platform.ai.llm_enabled,
        _ => unreachable!(),
    };
    if !platform_allows {
        return ResolvedAiMode::Deterministic;
    }

    // 5. Tenant-level decision.
    let tenant_decision = tenant
        .and_then(|t| t.ai.capabilities.get(&capability_name(capability)))
        .unwrap_or(&AiMode::Auto);
    if matches!(tenant_decision, AiMode::Off | AiMode::Deterministic) {
        return ResolvedAiMode::Deterministic;
    }

    // 6. Operation-level lfi/llm/auto.
    let final_mode = operation
        .filter(|m| !matches!(m, AiMode::Auto))
        .or_else(|| Some(*tenant_decision))
        .unwrap_or(AiMode::Auto);

    ResolvedAiMode::Augmented(final_mode)
}
```

Key properties (each is a load-bearing invariant):

1. **D capabilities are never augmented.** Layer 1+2+3 cannot promote a deterministic capability to AI.
2. **Any layer can demote to deterministic.** No layer can force AI on if another disagrees.
3. **Missing config means deterministic.** No config file → AI off.
4. **Operation-level `off` short-circuits.** A single HTTP header / CLI flag disables AI for that invocation only, regardless of other layers.
5. **Auto preserves the standing posture.** Default behavior unchanged unless an operator explicitly opts a layer differently.

---

## Fail-closed semantics

The substrate refuses to start (or refuses to invoke AI) when:

| Condition | Substrate behavior |
|-----------|-------------------|
| `platform.toml` malformed | Refuses to start with diagnostic. |
| `platform.toml` absent + AI features compiled in | Logs warning; treats AI as platform-disabled. |
| `tenant.toml` malformed | Refuses to serve that tenant; other tenants unaffected. |
| `tenant.toml` absent | Tenant runs deterministic-only. |
| AI provider unreachable | Fall back to deterministic; emit `tracing::warn` + audit-chain entry. |
| AI provider returns invalid response | Fall back to deterministic; same audit. |
| Provider signature mismatch (signed-model verification) | Refuse provider; fall back. |
| Rate-limit hit | Fall back; per-tenant counter. |
| Tenant requires `require_local_inference = true` + only external providers configured | Refuse; emit clear diagnostic. |

In every case, the substrate's deterministic baseline runs. The platform never errors due to AI unavailability — only due to malformed configuration of the AI layer.

---

## Audit chain integration

Every AI invocation appends a typed audit-chain entry (per `observability-core`):

```jsonc
{
  "event_kind": "ai_invocation",
  "capability": "originality_score",
  "resolved_mode": "lfi",
  "tenant_id": "acme-corp",
  "operation_origin": "cli_flag" | "http_header" | "mcp_input" | "auto",
  "provider": "local-lfi-v1.4.0",
  "duration_ms": 142,
  "fell_back": false,
  "fallback_reason": null,
  "signed_by": "ed25519:<base64url>"
}
```

When the resolution falls back to deterministic (provider down, signature failed, etc.), `fell_back: true` + `fallback_reason: "<diagnostic>"`. This is the audit trail that proves the platform's deterministic guarantee held during incidents.

Per `[[manifest-layer-is-the-keystone]]`: AI invocation logging is a manifest-projected concern, not a buried implementation detail.

---

## Operator UX

`loom edit serve` carries a tenant settings page (per task #141) that surfaces:

```
AI Augmentation (this tenant)

┌─ Platform availability ─────────────────────────┐
│ LFI:  enabled  (provider: local-lfi-v1.4.0)     │
│ LLM:  disabled (not compiled in)                │
└─────────────────────────────────────────────────┘

┌─ Tenant standing posture ───────────────────────┐
│ ☑ LFI enabled for this tenant                   │
│ ☐ LLM enabled for this tenant                   │
│ ☑ Require local inference (sovereignty)         │
│ ☑ Log invocations to tenant audit chain         │
│ ☐ Opt in to experimental capabilities           │
└─────────────────────────────────────────────────┘

┌─ Per-capability (A & P only) ───────────────────┐
│ similarity_check           [lfi ▼]              │
│ content_drift_detection    [lfi ▼]              │
│ trust_safety_concern_sug.. [deterministic ▼]    │
│ ...                                              │
└─────────────────────────────────────────────────┘

Recent fallbacks (last 24h):
  3 × originality_score    → provider rate-limited
  1 × content_drift        → signature mismatch
```

Operator can flip toggles; changes commit to `tenant.toml` via signed write (Ed25519 + ML-DSA dual per `[[backward-compat-version-discipline]]`).

---

## Tenant migration

When tenant.toml schema_version increments:
- Cat 1+2 changes (additive): tenant.toml continues to read; new fields default.
- Cat 3 (auto-migration): substrate runs migration on read; rewrite tenant.toml on next save with notice in operator UI.
- Cat 4 (operator-action): substrate refuses to serve tenant; operator follows playbook.

Per `VERSION_DISCIPLINE.md`.

---

## CI verification

Two scenarios (per task `#189`):

**Scenario A** — AI disabled at compile time. Build with `--no-default-features`. Run full Forge pipeline + Crawler journeys + audit phases. Assert zero errors, all D capabilities functional, all A capabilities run deterministic baselines, all P capabilities either degrade gracefully or surface clear "AI required" diagnostics.

**Scenario B** — AI compiled in, runtime-failed. Build with `--features lfi,llm`. Set `PLAUSIDEN_LFI_FORCE_FAIL=1` + `PLAUSIDEN_LLM_FORCE_FAIL=1`. Run full pipeline. Assert: zero panics; every AI invocation falls back to deterministic; audit-chain entries record `fell_back: true`; total findings within tolerance of Scenario A (proves deterministic baseline is functionally equivalent).

Both scenarios block PR merge.

---

## Anti-patterns

| ❌ Don't | ✅ Do |
|---------|------|
| Hardcode AI provider in capability call sites | Route through Critic trait abstraction (per `DETERMINISTIC_FIRST.md`) |
| Use AI to *decide* whether to use AI | Configuration resolution is pure data; never AI-mediated |
| Add a new "ai_mode" parameter only to MCP, not CLI / HTTP | Cross-AI parity (per `[[priority-architectural-first-and-cross-ai]]`) |
| Fall back silently without audit entry | Emit `tracing::warn` + append audit-chain entry |
| Treat absent config as "use AI" | Default deterministic; absent config means OFF |
| Skip schema validation on `platform.toml` / `tenant.toml` | `forge platform validate` + `forge tenant validate` mandatory |
| Let one layer override another's "off" | "Off" wins at any layer; never overridable upward |
| Hardcode capability postures in calling code | Reference the CAPABILITY_AI_POSTURE.md inventory; if posture changes, lookup updates automatically |

---

## Implementation arc

This design (#187, this doc) gates:

| Task | Deliverable |
|------|-------------|
| **#187** (this) | Design doc + 3-layer schema + precedence rules + fail-closed semantics |
| **#186** [determ-v3] | Critic trait pattern (composite impl that consults this resolution) |
| **#188** [determ-v5] | Audit existing AI-assuming code; refactor through `resolve_ai_mode` |
| **#189** [determ-v6] | CI Scenarios A + B (described above) |
| **#141** [backcompat-v5] | Operator version-management UI in `loom edit serve` (includes the AI tenant settings page sketched above) |

---

## See also

- `DETERMINISTIC_FIRST.md` — the architectural doctrine
- `CAPABILITY_AI_POSTURE.md` — per-capability D/A/P inventory; this doc is *how to control* what that doc *categorizes*
- `VERSION_DISCIPLINE.md` — `platform.toml` + `tenant.toml` versioning
- `SUBSTRATE_DISCIPLINE.md` — Rule 0; AI augmentation is opt-in to substrate, not a replacement
- `N_ORIENTATION_SUBSTRATE.md` — Sovereignty orientation captures PSA posture per tenant
- `[[deterministic-first-lfi-optional]]` memory — the founding directive
- `[[super-society-tech-stack]]` memory — sovereignty as first-class
- `[[manifest-layer-is-the-keystone]]` memory — config schemas project through manifest
