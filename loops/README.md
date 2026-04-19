# Loops

Long-running scripts an agent runs to do repetitive work between human
interactions. Loops differ from crons:

- **Cron** = periodic, fire-and-forget, lifecycle managed by `cron`.
- **Loop** = persistent, can hold state, can react to events, lifecycle
  managed by the agent or systemd.

Every loop registers in [`REGISTRY.md`](REGISTRY.md). An unregistered loop
is forbidden and gets killed by the next compliance scan.

## Available loops

| Script | Purpose | Default cadence |
|--------|---------|-----------------|
| [`audit-loop.sh`](audit-loop.sh) | Continuous improvement: walk the daily routine, file findings, sleep. | 30 min |
| [`ipc-poll-loop.sh`](ipc-poll-loop.sh) | Watch IPC bus for messages addressed to this agent; ack and dispatch. | 60s |
| [`sprint-progress-loop.sh`](sprint-progress-loop.sh) | Walk the active sprint board, post a progress summary. | 4h |
| [`drift-detect-loop.sh`](drift-detect-loop.sh) | Watch for drift between vendored deps and upstream; file foss findings. | 12h |

## Running a loop

```bash
# Interactive (blocks, Ctrl-C to stop)
bash loops/ipc-poll-loop.sh --agent claude-1

# Background with PID file under runtime/
bash loops/ipc-poll-loop.sh --agent claude-1 --background

# Stop a backgrounded loop
bash loops/ipc-poll-loop.sh --stop
```

## Installing as systemd user units

```bash
bash loops/install-systemd.sh                      # interactive
bash loops/install-systemd.sh --yes ipc-poll       # non-interactive
bash loops/install-systemd.sh --uninstall          # remove all
```

User units live under `~/.config/systemd/user/plausiden-*.service`.

## Stop signals

A `stop` from the user kills every PlausiDen loop:

```bash
bash loops/install-systemd.sh --uninstall
for pid in runtime/*.pid; do kill "$(cat "$pid")" 2>/dev/null || true; done
```

This is also what an agent runs when it sees a stop word from the user.

## Per-host state

Loop PIDs and per-iteration state live under `loops/runtime/` (gitignored).
Logs live under `~/.local/share/plausiden/loop-state/`.
