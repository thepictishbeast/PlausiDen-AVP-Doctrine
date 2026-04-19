# New Feature Prompt

```
Open a new feature for: {{TASK_OR_TICKET}}

Context:
- Surface: {{REPO_AND_PATH}}
- Owner agent: {{AGENT_ID}}
- Required by: {{MILESTONE_OR_DATE}}
- Stakeholder: {{HUMAN_OR_TEAM}}

Run the `new-feature` audit routine from PlausiDen-Audits. That routine
covers:
  - new-feature (was this scoped, threat-modeled, designed before code?)
  - scope (no scope creep)
  - goal (advances a stated goal)
  - threat-model (written, current, signed)
  - ux (first-contact ≤ 5 min, every error answers what/why/what-now)
  - accessibility (WCAG 2.1 AA across all flows)

Produce, in this order:
  1. A scope doc (≤ 1 page) listing what's in / what's out / what's deferred.
  2. A threat-model doc covering the new attack surface this introduces.
  3. A design sketch (text + ASCII or low-fi wireframe) for any UI.
  4. A list of audits this feature must pass before merge.
  5. The first PR with the scaffold + tests, NOT yet wired into production.

Stop and ask the human if:
  - Scope expansion is needed.
  - A new dependency is required.
  - A schema migration is required.
  - The threat model surfaces something the existing controls don't cover.
```
