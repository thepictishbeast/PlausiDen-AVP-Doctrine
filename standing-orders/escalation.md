# Escalation Standing Order

> When does an agent stop and ask the human?

## Always escalate (no exceptions)

- A real secret (PAT, API key, password, private key) appears in code,
  history, logs, or training data.
- Any action that would push to a public branch or release tag.
- Any action that would touch shared infrastructure (production DB,
  customer-facing endpoint, billing, auth provider).
- Any action against `wlan0` or system networking config.
- A `SHIP-DECISION:` would be required and the residual risk is non-trivial.
- The agent is about to disagree with another agent on a destructive action.
- A user message contains "stop", "halt", "kill", "abort", "freeze", "wait",
  or any local-language equivalent.

## Escalate before acting

- Schema migrations.
- Public API breaking changes.
- Adding a new dependency that brings >100 transitive deps.
- Adding a new outbound network destination.
- Removing a feature flag.
- Renaming a public type, file, or repo.
- Rewriting more than ~500 lines of working code.
- Anything that could plausibly be reversed only by `git reflog`.

## Escalate after acting (still required)

- A non-trivial mistake was made and the recovery is in flight.
- A finding was discovered that affects a sibling repo (also CROSSFIX).
- The agent's plan needs more than 30 minutes to complete (chunk it and
  surface the chunks).

## How to escalate

Post on IPC with `to=["human"]`, kind `task`, subject prefixed `ESCALATION:`.
Then **wait** — do not proceed until the human acks. If the agent's loop
fires before the ack arrives, the agent re-checks the IPC and waits again.

If the agent loses its IPC connection, default to halting and writing a note
to the agent's memory file. Resume only after a human authorization is
visible in the next session.
