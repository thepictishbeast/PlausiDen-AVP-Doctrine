# Crons

Cron snippets for scheduled agent routines, for hosts where the init system
is **not** systemd. If you're on systemd (most Linux desktops/servers,
including the primary PlausiDen workstation), use
[`../timers/`](../timers/) instead — timers are the native idiom and
`Persistent=true` handles missed runs during downtime.

**None of these are installed automatically.** The human (or an authorized
agent) runs `install.sh` after reviewing.

## Available crons

| File | Cadence | Purpose |
|------|---------|---------|
| [`daily-audit.cron`](daily-audit.cron) | nightly 06:00 local | Run the `daily` audit routine across all repos. |
| [`weekly-audit.cron`](weekly-audit.cron) | Mondays 06:00 local | Run the `weekly` audit routine — supersociety, adversary, improvement, audits-of-audits. |
| [`hourly-heartbeat.cron`](hourly-heartbeat.cron) | every hour | Each agent posts a heartbeat to IPC; missed heartbeats raise alerts. |
| [`nightly-ipc-archive.cron`](nightly-ipc-archive.cron) | nightly 03:00 local | Roll IPC bus.jsonl > 30 days into archive. |
| [`weekly-foss-update-check.cron`](weekly-foss-update-check.cron) | Sundays 04:00 local | Diff vendored FOSS deps against upstream releases; file findings. |

## Installation

```bash
bash crons/install.sh                    # interactive — asks per cron
bash crons/install.sh --yes daily,weekly # non-interactive — only the listed crons
bash crons/install.sh --uninstall        # remove all PlausiDen crons
```

Installed crons are tagged with the comment `# managed by PlausiDen-AVP-Doctrine`
so the uninstaller can find and remove them without touching unrelated jobs.

## Verification

```bash
crontab -l | grep PlausiDen-AVP-Doctrine    # show installed
ls -lh ~/.local/share/plausiden/cron-state/ # last-run timestamps
tail -n 50 ~/.local/share/plausiden/cron-state/*.log  # recent output
```

## Stop signals

If the user issues a stop word (see [`../standing-orders/stop-conditions.md`](../standing-orders/stop-conditions.md)),
the agent runs:

```bash
bash crons/install.sh --uninstall
```

…to remove all PlausiDen-managed crons in one step.

## Per-host state

Cron output and per-run state live under
`~/.local/share/plausiden/cron-state/`. This directory is .gitignored;
nothing in this folder gets pushed.
