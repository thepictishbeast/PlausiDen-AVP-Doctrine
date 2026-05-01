# Conductor host abstraction

> **Status:** v0.1 design. Types in `conductor-core::host` (committed). Real SSH driver lands with task #25 (conductor v0.2).
> **Owner:** William Armstrong
> **Last updated:** 2026-05-01

## Why

The conductor isn't a single-machine tool. The PlausiDen portfolio is wide
enough — and the build/audit fan-out aggressive enough — that the right
shape is a **fleet orchestrator**: one conductor process drives many
Claude Code subprocesses, some local, some on workstations across the
LAN, some on rented VPSs.

Reasons to care:

- **Parallelism above one box.** A single workstation caps at its core
  count + RAM. Distributing sessions across N machines linearly scales
  throughput on independent intents.
- **Failure isolation.** A workstation rebooting mid-run shouldn't
  abort the whole portfolio sweep — the conductor reroutes new sessions
  to other hosts and resumes the dead ones when the box comes back.
- **Latency to remote git remotes.** A VPS in the same AZ as
  `git.plausiden.com` clones faster than this Kali box; running cheap
  drift-detection / read-only audits there saves minutes per cycle.
- **Trust segmentation.** Long-tail experiments / FOSS-absorption
  sandboxing belong in a disposable VM, not on the workstation that
  has all the keys.

## `Host` types

`conductor-core::host`:

```rust
pub enum Host {
    Local,                       // conductor's own machine
    Ssh(SshTarget),              // SSH-reachable remote
    // future: Docker, Kubernetes, EC2-on-demand, …
}

pub struct SshTarget {
    pub host: String,                       // required
    pub user: Option<String>,               // ~/.ssh/config default if None
    pub port: Option<u16>,
    pub identity_file: Option<PathBuf>,
    pub remote_workdir: PathBuf,            // where the repo lives on the remote
    pub ssh_options: Vec<String>,           // pass-through `-o Foo=bar`
    pub control_socket: Option<PathBuf>,    // ControlMaster socket
}
```

Validation (`SshTarget::validate`) rejects:

- empty host
- whitespace in host or user
- relative `remote_workdir` (must be absolute on the remote)
- `..` in `remote_workdir`
- newlines/CR in any `ssh_options` entry (would let a malformed config
  inject SSH flags into the driver's command line)

`Session::new_on(intent, worktree, host)` binds the session to a host;
`Session::new(...)` defaults to `Host::Local`.

## SSH driver design (lands with #25)

The conductor will ship two `ClaudeDriver` implementations:

- `LocalClaudeDriver` — `tokio::process::Command::new("claude").current_dir(&worktree).args([...]).spawn()`
- `SshClaudeDriver` — wraps the same args inside `ssh user@host -o ControlMaster=auto -o ControlPersist=10m -o ControlPath=<socket> -p <port> -i <key> "cd <remote_workdir> && claude --print --output-format=json …"`

### Connection multiplexing

An SSH driver naively re-establishes the TCP/TLS handshake for every
poll, which would hammer the SSH agent + the network + the remote sshd.
The driver opens a `ControlMaster` socket on first contact:

```
ssh -o ControlMaster=auto \
    -o ControlPersist=10m \
    -o ControlPath=~/.cache/conductor/sock/<sha8> \
    user@host -- :
```

Subsequent commands reuse the socket; the keep-alive holds the
connection open between polls. The `<sha8>` is a hash of
`(user, host, port)` so distinct targets get distinct sockets and
nothing collides.

### Streaming

`claude --print --output-format=json` writes one JSON event per line.
The driver:

1. Spawns the SSH subprocess (or local subprocess for `Host::Local`).
2. Reads stdout line-by-line via `tokio::io::BufReader::lines()`.
3. Parses each line into a `claude_cli::Event`, maps to `DriverEvent`.
4. Buffers events in a per-session VecDeque; `poll()` drains.

stdin remains open in case the supervisor wants to send a follow-up
prompt on `resume` (we'll see whether the `--continue` flag is
sufficient or whether we need an interactive bidi).

### Pause-reason mapping

The CLI's exit codes / stderr lines map to `PauseReason`:

| Observed signal | Mapped to |
|---|---|
| HTTP 429 from API | `RateLimit` |
| `connection refused` / TCP RST / DNS fail | `Network` |
| `compaction triggered` log line | `ContextCompaction` |
| permission prompt that's not in the per-repo allowlist | `Permission` |
| any other non-zero exit + non-allowlisted stderr | `Blocked` |

For SSH, an additional bucket: ssh exit codes 255 (connection failure)
and 130 (interrupted) map to `Network`.

### Resume

`resume(handle)` runs `claude --continue <session-id>` in the same
working directory. For SSH, the `<session-id>` is the Claude-side
session id, and the conductor just re-issues the SSH command with that
flag. The Claude API persists session state server-side, so resuming
across an SSH disconnect is straightforward.

### Kill

`kill(handle)` for local: `tokio::process::Child::kill()`.

For SSH: send SIGTERM to the remote PID via a separate
`ssh ... kill <remote_pid>`. The remote PID is captured at spawn time
by appending `& echo $!` to the launch command and reading the line.
We track `(local-pid, remote-pid)` in `SessionHandle::token`.

## Configuration

Conductor accepts a TOML config (path defaults to
`~/.config/conductor/hosts.toml`):

```toml
# Local is implicit; you only declare it here to override defaults.

[[host]]
name = "vps-eu-1"
host = "vps-eu-1.plausiden.com"
user = "william"
port = 22
identity_file = "~/.ssh/id_ed25519_plausiden"
remote_workdir = "/srv/plausiden/PlausiDen-Engine"
ssh_options = ["ServerAliveInterval=30"]

[[host]]
name = "macbook-mobile"
host = "192.168.1.42"
user = "william"
remote_workdir = "/Users/william/Development/PlausiDen/PlausiDen-Engine"

# Routing rules: which sessions go where.
[routing]
default = "local"
# "intents matching this glob run on this host"
rules = [
    { match = "tests-*",   host = "vps-eu-1" },
    { match = "browser-*", host = "macbook-mobile" },
]
```

Routing is intentionally simple in v0.1 — match-on-agent-id-or-tag.
Smarter scheduling (load-aware, latency-aware, cost-aware) lands when
we have data.

## Threat model addenda

Adding remote hosts opens new attack surface:

- **SSH compromise.** Per-target identity files; no plaintext
  passwords; ssh-agent forwarding off by default. Use jump hosts for
  internal-only targets.
- **Remote disk fill.** Conductor watches stderr for "no space left"
  and demotes the host (no new sessions scheduled there) until cleared.
- **Time skew.** Remote clock drift can confuse `expires_after` math
  on `.avp-intent.toml`. Conductor refuses to enroll a session on a
  host whose UTC differs from local by >5 minutes (sanity check at
  driver startup).
- **Code injection via host config.** All `ssh_options` validate
  against newlines/CR; remote_workdir validates absolute + no `..`.

## Out-of-scope (post-v1)

- Docker / k8s / cloud-run hosts.
- `conductor sync` to rsync a worktree to a remote.
- Cost/latency-aware routing.
- Per-host `.claude/settings.json` curation (v0.1 ships per-repo).
