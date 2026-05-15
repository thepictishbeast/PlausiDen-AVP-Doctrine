# Standing Orders

> ⚠️ **APPLIES TO MULTI-AGENT SETUPS ONLY** — these files define
> coordination protocol for environments running **multiple Claude Code
> instances (or other AI agents) working together as a swarm**: role
> assignments (Claude-0 Architect / Claude-1 Refiner / Claude-2 Frontend),
> IPC bus discipline, escalation rules, stop conditions, per-agent memory
> layout. They are **not** intended as governing rules for single-agent
> operation (one Claude Code on one host, doing one thing at a time).
>
> - **Single-agent setup:** this directory is reference material, not
>   rules you must follow. Apply the per-project doctrine in each
>   consumer repo instead.
> - **Multi-agent swarm:** read the role file matching your agent plus
>   all cross-cutting files (stop-conditions, escalation, ipc, memory).
>
> The IPC bus paths described here assume a primary host filesystem;
> adjust per [`ipc.md`](ipc.md) for your deployment.

> What an AI agent reads at session start. Re-read on every context compaction.

The doctrine manages the agent so the human doesn't have to. An agent that
follows its standing-orders file does not need per-task supervision; it does
need the user to set high-level direction and to authorize destructive or
broadcast actions.

## Per-role files

| Agent role | File | One-line purpose |
|------------|------|------------------|
| The Architect (Claude-0) | [`claude-0-architect.md`](claude-0-architect.md) | Backend, structural engineering, Rust core, transducers. |
| The Refiner (Claude-1) | [`claude-1-refiner.md`](claude-1-refiner.md) | Data quality, security, dedup, decontamination, training corpora. |
| The Frontend (Claude-2) | [`claude-2-frontend.md`](claude-2-frontend.md) | UI, UX, dashboard, classroom, telemetry visualization. |
| Generic agent | [`generic-agent.md`](generic-agent.md) | Any other Claude / Gemini / local model joining the swarm. |

## Cross-cutting files

| File | Purpose |
|------|---------|
| [`ipc.md`](ipc.md) | Where the inter-Claude bus lives, message format, ack discipline. |
| [`escalation.md`](escalation.md) | When an agent must pause and ask the human before continuing. |
| [`stop-conditions.md`](stop-conditions.md) | What words / signals make every agent halt immediately. |
| [`memory.md`](memory.md) | What goes in `~/.claude/projects/-/memory/` and what doesn't. |

## How to apply

1. **Session start:** read the file matching this agent's role plus all
   cross-cutting files.
2. **After context compaction:** re-read.
3. **When IPC delivers a message:** ack per `ipc.md`.
4. **On any stop signal:** halt per `stop-conditions.md`. Do not negotiate.
5. **Before any destructive action:** check `escalation.md`. When in doubt,
   ask before acting.
6. **End of session:** write a handoff note to the agent's memory file.
