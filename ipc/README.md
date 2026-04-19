# IPC Communications Protocol

> Canonical specification for the Inter-Agent bus used by every PlausiDen
> agent (Claude-0, Claude-1, Claude-2, and any generic agent).

This folder is the full protocol reference. The short operational rules for
agents live in [`../standing-orders/ipc.md`](../standing-orders/ipc.md).

## Files

| File | Purpose |
|------|---------|
| [`README.md`](README.md) | You are here — protocol overview. |
| [`discovery.sh`](discovery.sh) | Resolve the bus path across primary + fallback locations. |
| [`schema.json`](schema.json) | JSON Schema for one message (validate every write). |
| [`transport.md`](transport.md) | File format, append semantics, read semantics, locking. |
| [`client.sh`](client.sh) | Reference CLI: post, tail, ack, search. |

## Path discovery (why fallbacks matter)

The canonical bus path can become unavailable for several reasons — tmpfs
reset, filesystem unmount, user re-org, permissions change, a sibling process
moving the symlink. The protocol defines a **priority-ordered fallback list**:
the first writable location in the list wins. Agents MUST use
[`discovery.sh`](discovery.sh) (or an equivalent resolver in their host
language) to find the bus; hard-coding a path is a bug.

Priority order:

| # | Location | Reason |
|---|----------|--------|
| 1 | `$IPC_BUS` (env var, if set and writable) | Explicit override for tests / overrides. |
| 2 | `/home/user/Development/PlausiDen/.ipc/bus.jsonl` | Canonical on the primary host. |
| 3 | `$XDG_RUNTIME_DIR/plausiden/bus.jsonl` | Session-bound runtime dir (Linux). |
| 4 | `$HOME/.local/share/plausiden/ipc/bus.jsonl` | User-private persistent. |
| 5 | `$HOME/.cache/plausiden/ipc/bus.jsonl` | Best-effort ephemeral. |
| 6 | `/tmp/claude-ipc/bus.jsonl` | Legacy — may reset on reboot. |

On **read** the resolver returns the first **existing and readable** path.
On **write** the resolver returns the first path whose parent is (or can be
made) writable; if no such location exists the resolver exits 1 and the
agent halts per [`../standing-orders/stop-conditions.md`](../standing-orders/stop-conditions.md).

### Symlink strategy

The canonical path at `/home/user/Development/PlausiDen/.ipc/` MAY be a
symlink to a persistent location (e.g. the repo's own hidden `.ipc/`). When
agents create bus files at fallback locations they SHOULD also drop a
breadcrumb file `bus.where` containing the absolute path of the active bus,
so orchestrators can discover it without running the resolver.

## Message format

Each line of `bus.jsonl` is one JSON object validating against
[`schema.json`](schema.json). Required fields:

```json
{
  "id": "uuid-v4",
  "ts": "2026-04-19T18:32:11Z",
  "from": "claude-1",
  "to": ["claude-0"],
  "kind": "task",
  "subject": "short identifier",
  "body": "free text or a stringified JSON payload",
  "refs": { "pr": 123, "audit": "data-leak", "commit": "abc123" }
}
```

### Message kinds

| Kind | Direction | Semantics |
|------|-----------|-----------|
| `task` | agent → agent or agent → human | Work request. MUST be acked. |
| `ack` | any | Acknowledges a prior message by `refs.original`. |
| `finding` | any → any | Audit finding; `refs.audit` MUST be set to the slug. |
| `broadcast` | any → `["all"]` | State change everyone should know about. |
| `heartbeat` | agent → `["all"]` | Liveness. Includes host + load snapshot. |
| `halt` | human → `["all"]` | Stop signal. All agents halt per `stop-conditions.md`. |

### Ack discipline

- Every message addressed to an agent MUST be acked within the agent's next
  loop iteration.
- An ack is itself a message with `kind: "ack"` and
  `refs.original: <original id>`.
- If an agent cannot act on a message, it still acks, with a body explaining
  (queued, deferred, declined).

## Transport & locking

See [`transport.md`](transport.md). Summary:

- Append-only JSONL file.
- Writers use O_APPEND (a single write under the POSIX atomic-write boundary
  at 4 KiB / 8 KiB depending on fs — messages SHOULD stay under 4 KiB).
- Messages > 4 KiB reference a side-file via `refs.body_path` instead of
  inlining the body.
- Readers use byte-offset cursors; the poll loop tracks cursor per-agent.
- The bus is eventually rotated to `bus.archive/<YYYY-MM>.jsonl` by the
  nightly cron.

## Schema validation

Writers SHOULD validate their message against
[`schema.json`](schema.json) before append. Readers MAY validate; a malformed
line is logged and skipped — one bad message never halts the bus.

## Quotas and backoff

- Soft cap: 1 message / second / agent.
- Hard cap: 100 messages / minute / agent.
- Exceeding hard cap triggers a `finding` message from the offending agent
  and 60-second backoff.

## Failure modes

| Failure | Agent behavior |
|---------|----------------|
| Resolver finds no writable location | Halt per stop-conditions, log to agent's memory file. |
| Resolver finds no readable bus on read | Log and sleep for the loop cadence; try again. |
| Malformed JSON line on read | Log, skip, continue. |
| `from` field unknown | Log finding to `ai-agent` audit. |
| Message older than 30 days appears on read cursor | Likely clock skew; log and process anyway. |

## Stop-signal propagation

A `kind: "halt"` broadcast from the human is the kill switch for the whole
swarm. On receipt, every agent halts per
[`../standing-orders/stop-conditions.md`](../standing-orders/stop-conditions.md),
acks the halt, and does not resume until a non-halt message from the human
is received.
