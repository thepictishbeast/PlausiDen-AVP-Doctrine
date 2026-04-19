# Audit-Self Prompt

```
Run the appropriate audit routine against the current diff or the last
commit before deciding to push.

Step 1: Determine the right routine.
  - Local commit, no PR yet: pre-commit
  - PR open or about to open: pre-merge
  - Tagging a release: pre-release
  - Just a daily check: daily
  - Weekly drift sweep: weekly
  - Something is wrong: incident

Step 2: For each audit in the routine, walk the checklist in
PlausiDen-Audits/audits/<slug>/checklist.md. For each item:
  - if green: tick it.
  - if red:
      - if fixable in place: fix and re-run.
      - if not fixable in place: file an issue and either request a
        SHIP-DECISION from the human or escalate.

Step 3: Record findings in audits/<slug>/findings/<YYYY-MM-DD>-<agent>.md.

Step 4: Post a summary to the IPC bus, kind: finding,
to: ["all"] for blockers, ["human"] for SHIP-DECISION requests.

Step 5: Decide:
  - block: do not commit / do not open PR / do not push.
  - proceed: commit / open PR / push.
  - escalate: stop and wait for human authorization.

Default to block. Proceed only if every required audit is green or
carries a current human-signed SHIP-DECISION.
```
