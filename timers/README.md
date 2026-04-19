# Timers

Systemd system timers for scheduled agent routines. This is the native idiom
on systemd hosts — use these instead of [`../crons/`](../crons/) when
systemd is available (check with `systemctl --version`).

Timers ship as `/etc/systemd/system/plausiden-<name>.{service,timer}`
pairs, generated and installed by [`install.sh`](install.sh). Each carries
`# managed by PlausiDen-AVP-Doctrine` in the unit file so the uninstaller
can find them without touching anything unrelated.

## Available jobs

| Job | OnCalendar | Purpose |
|-----|------------|---------|
| `daily-audit` | `*-*-* 06:00:00` | Run the `daily` audit routine per repo. |
| `weekly-audit` | `Mon *-*-* 06:00:00` | Run the `weekly` audit routine per repo. |
| `hourly-heartbeat` | `hourly` | Post agent heartbeat to IPC bus. |
| `nightly-ipc-archive` | `*-*-* 03:00:00` | Roll IPC bus.jsonl entries older than 30 days. |
| `weekly-foss-check` | `Sun *-*-* 04:00:00` | Diff vendored FOSS deps vs upstream; file findings. |

All timers use `Persistent=true`, so missed runs (machine off, sleep, etc.)
fire on next boot.

## Installation

```bash
sudo bash timers/install.sh                 # interactive — asks per job
sudo bash timers/install.sh --all           # install every job
sudo bash timers/install.sh --yes a,b,c     # only the listed jobs
sudo bash timers/install.sh --uninstall     # remove all PlausiDen timers
     bash timers/install.sh --list          # show installed (no root needed)
```

## Verification

```bash
systemctl list-timers 'plausiden-*'                   # schedule overview
journalctl -u plausiden-hourly-heartbeat.service -n 50   # recent runs
ls -lh ~/.local/share/plausiden/cron-state/           # log files
```

## Stop signals

If the user issues a stop word (see
[`../standing-orders/stop-conditions.md`](../standing-orders/stop-conditions.md))
the agent runs:

```bash
sudo bash timers/install.sh --uninstall
```

…to remove every PlausiDen timer in one step.

## Per-host state

Log files live under `~/.local/share/plausiden/cron-state/` — gitignored,
per-host, never pushed.

## Why systemd timers instead of cron on this host

- `Persistent=true` handles missed runs during downtime (cron silently
  misses them).
- Journaled output via `journalctl -u plausiden-*` — no log rotation
  bespoke.
- Unit files are the same format as the rest of the service catalog —
  one mental model.
- `systemctl list-timers` gives one-glance schedule visibility.
