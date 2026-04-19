# Standing Orders

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
