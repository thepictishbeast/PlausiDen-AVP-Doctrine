# IPC Transport

## File format

- One message per line (JSON Lines / `.jsonl`).
- UTF-8 encoded; no BOM.
- Trailing newline after every message.
- Lines > 4 KiB forbidden — inline bodies must fit within one POSIX
  atomic-append page. Oversized bodies reference a side-file via
  `refs.body_path` containing a path in the same user's storage.

## Writes

- Open with `O_APPEND`; one `write(2)` per message.
- On POSIX file systems with PIPE_BUF ≥ message size, appends are atomic
  against concurrent writers — no locking required.
- If the host system's filesystem doesn't guarantee that (NFS, FUSE mounts
  without `noac`), writers MUST take an advisory lock around the append:
  `flock(2)` on the parent directory file descriptor for the write.

Example (shell):

```bash
exec 9>>"$bus" || { echo "bus unwritable"; exit 1; }
flock 9 || true   # best-effort on normal fs, required on NFS
printf '%s\n' "$message" >&9
exec 9>&-
```

## Reads

- Readers keep a byte-offset cursor per agent, per bus path.
- On iteration, readers stat the bus, compute new bytes since cursor, and
  read only the new region.
- Malformed lines are logged and skipped; never aborts the reader.
- Cursors survive process restarts (persisted under
  `$DOCTRINE_ROOT/loops/runtime/ipc-poll-<agent>.cursor`).

## Rotation

The nightly cron at `crons/nightly-ipc-archive.cron` calls
`scripts/archive-ipc.sh`, which:

1. Splits messages older than 30 days by YYYY-MM.
2. Appends them to `bus.archive/<YYYY-MM>.jsonl` in the same parent dir.
3. Rewrites `bus.jsonl` with only messages from the last 30 days.
4. Never deletes an archive file.

Agents do NOT read archive files during normal polling. Audit tooling may.

## Oversized messages

A message whose body exceeds the 3 800-character limit (3 800 chars, not
bytes, leaving slack for envelope):

1. Writer writes the body to `$(dirname "$bus")/body/<message-id>.txt`.
2. Writer sets `refs.body_path` in the envelope.
3. Readers that need the body do `cat "$(dirname bus)/$refs.body_path"`.
4. `body` field may be a truncated preview (≤ 200 chars) for display.

## Backpressure

If a writer exceeds the hard cap (100 msg/min/agent), further writes MUST
silently drop and the agent MUST file a `finding` against the `ai-agent`
audit within the next iteration. The first dropped message SHOULD still be
written; subsequent writes within the violation window SHOULD be dropped
or queued.

## Concurrency model

- Writers: many; single-file append with `O_APPEND`. No explicit lock on
  local ext4/btrfs/zfs.
- Readers: many; each maintains its own cursor. No coordination needed.
- Archiver: singleton; takes `flock` on the bus file for the duration of
  the rewrite.
