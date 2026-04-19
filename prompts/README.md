# Agent Prompt Templates

Reusable prompts the human (or another agent) can drop on an agent to start
a class of work. Each template has a `{{...}}` parameter list and a checklist
the receiving agent walks through.

| File | Purpose |
|------|---------|
| [`new-feature.md`](new-feature.md) | Open a new feature with scope, threat model, audit list, owner, deadline. |
| [`bug-fix.md`](bug-fix.md) | Fix a bug with reproduction, regression test, audit re-run, postmortem note. |
| [`code-review.md`](code-review.md) | Review a PR or diff with the standard rubric, return one of: approve / changes / block. |
| [`audit-self.md`](audit-self.md) | Run the relevant routines from PlausiDen-Audits against the current diff. |

## Usage

```
/new-feature task=#142 surface=Browser-Ext owner=claude-2
```

The receiving agent loads the template, fills in parameters from the task
metadata, and proceeds. The template tells the agent which audits to run
and which standing-orders apply.
