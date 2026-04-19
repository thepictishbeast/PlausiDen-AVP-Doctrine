# Bug Fix Prompt

```
Fix the bug described in: {{ISSUE_OR_REPORT}}

Reproduction: {{STEPS_OR_LINK}}
Expected: {{WHAT_SHOULD_HAPPEN}}
Observed: {{WHAT_ACTUALLY_HAPPENS}}
Environment: {{OS_VERSION_BROWSER_DEVICE}}
Severity: {{LOW | MEDIUM | HIGH | CRITICAL}}

Steps (do not skip):
  1. Reproduce locally. If you cannot reproduce, post on IPC and pause.
  2. Write a failing test that pins the bug. Tag it `REGRESSION-GUARD:`.
  3. Find the root cause. Don't patch the symptom.
  4. Implement the fix.
  5. Verify the regression test now passes.
  6. Run `pre-commit` audit routine. Block on failures.
  7. Commit with: `fix({{SCOPE}}): {{ONE_LINE}}` — body must reference the
     issue and the audit that should have caught it (or "no audit covered
     this — proposing add-audit issue").
  8. Open PR, run `pre-merge`.
  9. After merge, file an `audits-of-audits` finding if no existing audit
     covered this bug class — propose what changes to the catalog would
     have caught it.

Stop and ask the human if:
  - The fix would touch more than one repo (file a CROSSFIX instead).
  - The reproduction reveals a dependency vulnerability — escalate.
  - The fix requires a schema migration — escalate.
  - The fix would change a public API — escalate.
```
