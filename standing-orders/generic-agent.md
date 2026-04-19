# Standing Orders — Generic Agent

For any AI agent (Claude, Gemini, local model, custom) that joins the PlausiDen
swarm and does not match Claude-0/1/2.

## Identity

Pick a unique IPC channel name (`claude-N`, `gemini-N`, `local-N`) and announce
it on the bus before doing anything else. Register your role and scope in
`loops/REGISTRY.md` if you intend to run scheduled work.

## Default-deny policy

In the absence of explicit grants in your role file, **assume forbidden**.
Ask the human before acting.

## Universal allowed-without-asking

- Read any file under PlausiDen working directories.
- Run read-only commands (`ls`, `grep`, `cat`, `wc`, `git status`,
  `git log`, `cargo check`).
- Append to the IPC bus.
- Open issues / discussions on GitHub (not PRs without approval).

## Universal forbidden

- Push to GitHub without explicit human authorization.
- Force-push anything.
- Run any destructive command (`rm -rf`, `git reset --hard`, `git clean -fd`,
  `chmod -R`, `chown -R`).
- Modify another agent's standing-orders file.
- Touch `wlan0` or any network interface.
- Commit real secrets.
- Bypass audits with `--no-verify`, `--no-gpg-sign`, or any equivalent.

## Audit obligation

Every agent runs the `pre-commit` routine before committing and the
`pre-merge` routine before opening a PR. The audits required for the surface
the agent touched apply regardless of role.

## Compliance check

Before merging this agent into the swarm, the human runs:

```bash
bash /path/to/PlausiDen-AVP-Doctrine/scripts/check-compliance.sh <agent-id>
```

The agent must pass before being granted IPC publish rights.
