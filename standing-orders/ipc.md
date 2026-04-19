# IPC Standing Order

## Bus location

**Never hard-code a bus path.** Use [`../ipc/discovery.sh`](../ipc/discovery.sh)
(or an equivalent resolver in your host language) to find the bus. The
resolver tries, in priority order:

1. `$IPC_BUS` — explicit override.
2. `/home/user/Development/PlausiDen/.ipc/bus.jsonl` — canonical on the
   primary host.
3. `$XDG_RUNTIME_DIR/plausiden/bus.jsonl` — session-bound runtime dir.
4. `$HOME/.local/share/plausiden/ipc/bus.jsonl` — user-private persistent.
5. `$HOME/.cache/plausiden/ipc/bus.jsonl` — best-effort ephemeral.
6. `/tmp/claude-ipc/bus.jsonl` — legacy, may reset on reboot.

The first writable location wins on write. The first existing + readable
location wins on read. See [`../ipc/README.md`](../ipc/README.md) for the
full protocol.

## Message format

JSON Lines, one event per line:

```json
{
  "id": "uuid-v4",
  "ts": "2026-04-19T18:32:11Z",
  "from": "claude-0",
  "to": ["claude-1"],
  "kind": "task | ack | finding | broadcast | heartbeat",
  "subject": "short identifier",
  "body": "free-form text or structured payload",
  "refs": { "pr": 123, "audit": "data-leak", "commit": "abc123" }
}
```

## Discipline

1. **Every message addressed to you must be acked** within the next iteration
   of your loop. Acks are themselves messages, kind `ack`, body referencing
   the original `id`.
2. **No silent reads.** If you read a message and cannot act on it, post an
   `ack` with body explaining why (queued, deferred, declined).
3. **Heartbeats** every loop iteration: kind `heartbeat`, body containing
   the agent's current state and what loop it's in.
4. **Findings** are first-class IPC messages. The agent that finds something
   in another agent's surface posts kind `finding` and CC's the relevant
   audit folder in PlausiDen-Audits.
5. **Broadcasts** to `["all"]` are reserved for state changes that affect
   every agent — e.g. "doctrine updated, re-read", "merge freeze in effect".

## Quotas

- Soft cap: 1 message / second / agent.
- Hard cap: 100 messages / minute / agent. Exceeding triggers a `findings`
  message and 60s backoff.

## Garbage collection

Messages older than 30 days are moved to `bus.archive/<YYYY-MM>.jsonl` by a
nightly cron in `crons/`. Archive files are append-only and never deleted.
