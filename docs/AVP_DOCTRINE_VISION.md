# PlausiDen-AVP-Doctrine — vision document

> "If PlausiDen-AVP-Doctrine was already built and did everything
> we wanted, what would this doc say?"

This is that doc. **[shipped]** works today. **[in-flight]** is
mid-build. **[queued]** has a task ID. **[concept]** has been
implied or requested and a developer should design it.

Companion to: [LOOM_VISION](https://github.com/thepictishbeast/PlausiDen-Loom/blob/main/docs/LOOM_VISION.md),
[CMS_VISION](https://github.com/thepictishbeast/PlausiDen-CMS/blob/main/docs/CMS_VISION.md),
[FORGE_VISION](https://github.com/thepictishbeast/PlausiDen-Forge/blob/main/docs/FORGE_VISION.md),
[CRAWLER_VISION](https://github.com/thepictishbeast/PlausiDen-Crawler/blob/master/docs/CRAWLER_VISION.md),
[ANNOTATOR_VISION](https://github.com/thepictishbeast/PlausiDen-Annotator/blob/master/docs/ANNOTATOR_VISION.md),
OXIDIZER_VISION (in PlausiDen-Oxidizer scaffold).

---

## 1. What PlausiDen-AVP-Doctrine IS

**The validation protocol every PlausiDen artifact is graded
against, plus the standing orders for every AI agent operating
in the ecosystem.** Not code that runs — *contracts that
mechanically constrain what gets built and how.*

Operationally, AVP-Doctrine is a directory tree of TOML +
Markdown contracts:

| Path | Role |
|---|---|
| `AVP2_PROTOCOL.md`     | Full specification of the AVP-2 supersociety protocol |
| `DOCTRINE.md`          | High-level doctrine the protocol enforces |
| `standing-orders/`     | What every agent reads at session start |
| `crons/` + `loops/`    | Routine-work specifications agents run on schedule |
| `gates/`               | Pre-commit / pre-merge / pre-release gates |
| `annotations/`         | Inline-annotation grammar (`AVP-PASS-N`, `SAFETY`, `SECURITY`, `REGRESSION-GUARD`, `SHIP-DECISION`, `UX-DEBT`, …) |
| `foss-absorption/`     | Protocol for absorbing external FOSS tools (discover → evaluate → absorb → integrate → loop → maintain) |
| `cross-repo/`          | Cross-repo contribution protocol (CROSSFIX commits, sibling test runs) |
| `ship-decision/`       | When + how to write `SHIP-DECISION:` annotations accepting residual risk |
| `avp-ratchet.toml`     | Per-repo AVP tier-progression state |
| `avp/`                 | Per-repo machine-readable AVP grade artifacts |

PlausiDen-AVP-Doctrine is **not**:

- Source code (no `src/`, no `Cargo.toml` at root — it's contracts)
- A CI runner (Forge / Oxidizer / per-repo CI implements the gates)
- A test framework (PlausiDen-Tests provides those harnesses)
- A linter (PlausiDen-Audits + Oxidizer enforce the catalog)
- A marketplace (the doctrine is one and only — not pluggable)

AVP-Doctrine's contract: every PlausiDen-* repo + every AI agent
operating in the namespace reads and obeys this doctrine.
Compliance is mechanically checked by Forge / Oxidizer / per-repo
CI; non-compliance refuses the merge or tag.

## The meta-mission: making AI-built UI reliable

Every PlausiDen tool exists to make agentic GUI / frontend / UX
work reliable. AVP-Doctrine is **the source-of-truth document
that defines what "reliable" means** mechanically — it's the
Constitution every other tool enforces. Without AVP-Doctrine,
agents would each invent their own quality bar; with it, the bar
is one, immutable, signed.

For an AI agent: AVP-Doctrine tells you EXACTLY what's required
to ship a PlausiDen artifact. Sequence: read standing-orders →
run crons / loops → gate every commit → annotate every line →
absorb FOSS through the protocol → contribute across siblings →
sign every SHIP-DECISION. Skipping any step refuses the merge.

## 2. Personas

### 2.1 Mom — non-technical client

Mom never reads AVP-Doctrine. What she gets: every artifact
shipped by any agent on her behalf has been graded against this
protocol. The "0 strict findings" badge on her admin portal
ultimately traces back to this doctrine — a stable contract she
can sue against if she's ever ill-served.

### 2.2 The technical client

What they get: every PlausiDen-* repo they fork starts with the
same doctrine inherited via the `avp-ratchet.toml` tier-state.
Their fork's quality gate is the upstream doctrine until they
explicitly waive (signed). Forks remain auditable against
upstream's expectations.

### 2.3 The developer / contributor

What they get today:

- **Standing orders at session start** — read, then operate.
- **Inline-annotation grammar** for `AVP-PASS-N` / `SAFETY` /
  `REGRESSION-GUARD` / `SHIP-DECISION` / `UX-DEBT` / `SCHEMA` —
  every annotation is machine-grepable for CI + audit.
- **FOSS absorption protocol** — discover → evaluate → absorb
  → integrate → loop AVP Tiers 1–3 minimum 12 passes → maintain.
- **Cross-repo contribution protocol** — CROSSFIX commits, run
  the full sibling test suite, regression mine.

What developers want next:

- **Doctrine-version-pinning per repo** — every repo declares
  which AVP-Doctrine commit it conforms to; bumps require an
  AVP review pass.
- **Doctrine diff-tool** — show what changed between two
  doctrine revisions in human-readable form.
- **Per-clause search** — find every repo that cites or waives
  a specific doctrine clause.
- **TLA+ specification of the AVP loop itself** — formal model
  of the 36-pass minimum + tier-progression rules.

### 2.4 Claude Code (and other autonomous agents)

What an agent gets today:

- **Standing-orders TOML the agent loads at session start** —
  defines how it operates.
- **Crons + loops** — routine work specs the agent runs on
  schedule (e.g., the 5-minute cron in `~/.claude/CLAUDE.md`
  is exactly this pattern).
- **Gates that refuse the commit** if an annotation is missing
  or a SHIP-DECISION lacks a signer.

What agents want next:

- **MCP server exposing doctrine queries** — `query_clause`,
  `list_gates`, `find_repos_citing_clause`, `propose_doctrine_amendment`.
- **Per-agent doctrine compliance score** in the audit log.
- **Doctrine-version-aware mode** — agent knows which doctrine
  version the repo it's editing conforms to, and gates accordingly.
- **Federated doctrine (peer-doctrine cross-signing)** —
  multiple AVP-Doctrine instances can cross-sign each other's
  doctrine revisions for decentralized governance.

## 3. Capability map

| Capability | Status |
|---|---|
| AVP2 protocol specification | shipped |
| Standing orders for agents | shipped |
| Inline-annotation grammar | shipped |
| FOSS absorption protocol | shipped |
| Cross-repo contribution protocol | shipped |
| SHIP-DECISION rules + signing requirements | shipped |
| Per-repo AVP-ratchet tier progression | shipped |
| Crons + loops for routine agent work | shipped |
| Per-repo machine-readable AVP grade artifacts (`avp/`) | shipped |
| `phase_doctrine_conformance` in Forge consumes doctrine TOMLs | concept |
| Oxidizer checks derived from doctrine clauses | concept |
| TLA+ specification of the AVP loop | concept |
| Doctrine diff-tool | concept |
| Per-clause search across the ecosystem | concept |
| MCP server exposing doctrine queries | concept |
| Federated doctrine (peer cross-signing) | concept |
| Hardware-key-signed doctrine amendments | concept |
| Doctrine-amendment proposal protocol (open RFC) | concept |
| Quarterly doctrine-revision ceremony with public log | concept |

## 4. Architecture (when fully built)

```
┌──────────────── PlausiDen-AVP-Doctrine ────────────────┐
│                                                          │
│  Contracts (TOML + Markdown):                           │
│  - AVP2_PROTOCOL.md      (the spec)                     │
│  - standing-orders/      (what agents do at session)    │
│  - crons/ + loops/       (routine-work specs)           │
│  - gates/                (pre-commit / merge / release) │
│  - annotations/          (inline grammar)               │
│  - foss-absorption/      (external FOSS protocol)       │
│  - cross-repo/           (CROSSFIX protocol)            │
│  - ship-decision/        (residual-risk acceptance)     │
│  - avp-ratchet.toml      (per-repo tier state)          │
│                                                          │
└──────────────────────────────────────────────────────────┘
            │
            ▼ enforced by
┌──────────────────────────────────────────────────────────┐
│  Every PlausiDen-* repo                                  │
│  - Forge phase_doctrine_conformance                      │
│  - Oxidizer checks derived from doctrine clauses         │
│  - per-repo CI gates                                     │
│  - AI agents reading standing-orders at session start    │
└──────────────────────────────────────────────────────────┘
```

## 5. Roadmap

### Sprint 1 — operationalise into Forge + Oxidizer

- Forge `phase_doctrine_conformance` — read doctrine TOMLs,
  generate audit phases procedurally
- Oxidizer `oxidizer-checks` derived from doctrine clauses
  (auto-generated from `gates/` + `annotations/`)
- Doctrine version pinning in every PlausiDen repo's `avp-ratchet.toml`
- Doctrine diff-tool (markdown + machine-readable)

### Sprint 2 — agent-facing surface

- MCP server (`avp-doctrine` namespace): `query_clause`,
  `list_gates`, `find_repos_citing_clause`,
  `propose_doctrine_amendment`
- Per-agent compliance score in audit log
- Doctrine-version-aware mode in standing-orders

### Sprint 3 — formal verification + federation

- TLA+ specification of the AVP loop
- Federated doctrine (peer cross-signing)
- Quarterly doctrine-revision ceremony with public transparency log

## 6. Acceptance criteria for "done"

1. Every PlausiDen-* repo cites the exact AVP-Doctrine commit it
   conforms to in its `avp-ratchet.toml`.
2. Every commit in every PlausiDen-* repo carries an `AVP-PASS-N`
   annotation per the doctrine grammar (Forge / Oxidizer enforces).
3. Every accepted residual risk has a SHIP-DECISION signed by
   the named human or hardware-key-attested agent.
4. The doctrine is mechanically queryable — an agent or human
   can ask "which clauses apply to filesystem-write paths?" and
   get a typed answer.
5. Doctrine amendments go through an open RFC process with
   signed approvals; no silent doctrine drift.
6. The TLA+ spec + per-clause invariants formally model the
   36-pass minimum + tier-progression rules.
7. Federated doctrine instances cross-sign for decentralized
   governance — no single AVP-Doctrine fork is canonical.
