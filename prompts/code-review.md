# Code Review Prompt

```
Review {{PR_URL_OR_DIFF}}.

Return one of: approve / request-changes / block.

Rubric (each row is a hard gate):

  - [ ] Verify-code audit clean on the diff (cargo fmt / clippy / test /
        doc all green).
  - [ ] Test audit clean: every public fn touched has a test; every closed
        bug has a regression test.
  - [ ] Data-leak audit clean: no new secret-shaped strings, no new logs
        with PII, no new uncontrolled outbound network.
  - [ ] Debug-logs audit clean: no new println! / dbg! / println!
        equivalents in library code.
  - [ ] Annotations present: BUG ASSUMPTION on new public fns, SAFETY on
        new unsafe blocks.
  - [ ] No SHIP-DECISION smuggled in without human signature.
  - [ ] No new dependency without cargo audit / cargo geiger / FOSS
        absorption protocol applied.
  - [ ] No public API change without escalation note in the PR description.

If any gate fails: comment with the specific finding and request changes.
If all gates pass:
  - Approve if confident.
  - Request a second reviewer if the diff is large (> 500 lines) or
    touches a security-sensitive surface.

Block (do not approve, do not merge) if:
  - A real secret is in the diff.
  - The diff bypasses a hook (--no-verify or similar).
  - The diff disables an existing test without REGRESSION-GUARD justification.
  - The diff would push to main without branch-protection compliance.
```
