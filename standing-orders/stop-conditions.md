# Stop Conditions

> What makes every agent halt immediately?

## Stop words from the human

Any of the following in any user message, anywhere in the conversation:

`stop`, `halt`, `kill`, `abort`, `freeze`, `wait`, `pause`, `enough`,
`stand down`, `back off`, `cease`, `shut up`, `quiet`, `silence`,
`shut it down`, `kill the loop`, `stop the heartbeat`, `stop the cron`.

On any of these:

1. **Halt the current tool call** as soon as the current operation completes.
2. **Cancel any pending tool calls.**
3. **Cancel any scheduled wakeups** (`ScheduleWakeup`) and any cron jobs the
   agent installed in this session. Use `CronList` then `CronDelete`.
4. **Stop any background tasks** the agent started. Use `TaskList` then
   `TaskStop`.
5. **Acknowledge in one short message** what was halted.
6. **Wait** for further direction. Do not infer the next step.

## Stop signals from another agent

A message on the IPC bus addressed to this agent with kind `broadcast` and
subject `HALT` halts the agent the same way. Acknowledge by posting an
`ack` and waiting.

## Stop signals from the doctrine itself

If a `pre-commit` audit fails, the agent does not commit. Period. The agent
either fixes the failure, files a `SHIP-DECISION:` after escalation, or
hands off.

If a `pre-merge` audit fails, the agent does not open / update the PR.
Same recourses.

## Stop signals from the environment

- Disk full → halt and report.
- IPC bus unreachable for > 60 seconds → halt and report.
- Authentication to a required service failed → halt and report.
- Any unexpected non-zero exit code from a build / test / audit script →
  halt and report.

## Resume conditions

After halting on a stop word from the human, resume only when:

- The human posts a new message that is not a stop word.
- The new message either explicitly tells the agent to resume or gives a
  concrete next instruction.

Do not auto-resume on a context compaction. Do not auto-resume on a
ScheduleWakeup that fired before the halt was acknowledged.
