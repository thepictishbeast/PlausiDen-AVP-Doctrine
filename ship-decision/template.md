# SHIP-DECISION template

```
// SHIP-DECISION: {{YYYY-MM-DD}}
//
// Milestone: {{MILESTONE_ID}}
// Surface: {{REPO}} / {{MODULE}} / {{FUNCTION_OR_FILE}}
//
// Audits waived (or partially passing):
//   - {{AUDIT_SLUG_1}} — {{REASON}}
//   - {{AUDIT_SLUG_2}} — {{REASON}}
//
// Accepted residual risks:
//   - {{RISK_1_DESCRIPTION}}
//     Severity: {{LOW | MEDIUM | HIGH | CRITICAL}}
//     Mitigation in flight: {{LINK_OR_DESCRIPTION}}
//   - {{RISK_2_DESCRIPTION}}
//     Severity: {{LOW | MEDIUM | HIGH | CRITICAL}}
//     Mitigation in flight: {{LINK_OR_DESCRIPTION}}
//
// Numbers (where available):
//   - Mutation survival rate: {{X.X}}%
//   - Line coverage: {{X.X}}%
//   - Open audit findings: {{N}}
//
// Follow-up: {{ISSUE_URL_OR_TODO_LINK}}
// Signed by: {{HUMAN_NAME}} (agent draft: {{AGENT_NAME}})
// Expires: end of milestone {{MILESTONE_ID}}, re-evaluate on milestone close.
```

## Inline placement rules

- Place above the function, struct, or module it affects.
- One `SHIP-DECISION:` per accepted risk; don't bundle risks under one
  annotation.
- Mirror the annotation in `CHANGELOG.md` under the milestone heading.
- File a copy in `audits/<slug>/findings/<date>-ship.md` in PlausiDen-Audits.

## Anti-patterns

- "We'll fix this later." Without a date, an owner, and a link, this is
  not a SHIP-DECISION; it's a wish.
- Bundling many risks under one waiver. Split them.
- Agent-only signature. A human must sign every SHIP-DECISION before merge.
- Indefinite expiry. Every SHIP-DECISION expires at the milestone boundary
  unless explicitly extended with a fresh signature.
