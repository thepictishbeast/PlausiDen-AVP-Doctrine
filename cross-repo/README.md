# Cross-Repo Contribution Protocol

> When you find a bug or improvement that applies to a sibling repo, switch
> contexts and fix it there. Then return.

PlausiDen is multi-repo. Bugs travel. Fixes must too.

## When this applies

- A finding in repo A is also true in repo B (shared base, shared dep,
  shared pattern).
- An improvement in repo A would benefit repo B.
- An audit run in repo A reveals a class of problem; the same class likely
  exists in repos C, D, E.

## The protocol

1. **Pull latest** from every sibling repo first. Align shared dependency
   versions before working in any of them.
2. **Switch to the affected sibling.** Apply the fix there. Run the
   short-loop AVP (Tiers 1-3, 6 passes minimum).
3. **Commit with a `CROSSFIX` prefix:**
   ```
   AVP-CROSSFIX from <source-repo>: <description>

   CROSSFIX: <source-repo> — <one-line explanation>
   ```
4. **Open the PR** on the sibling. Reference the source repo and the
   originating finding.
5. **Return** to the original task. Note in IPC that a crossfix is in flight.
6. **Run integration tests** spanning all affected repos before declaring
   the original task complete.

## Sibling treatment

Treat every sibling's code with the same suspicion as third-party code.
Your past self is an untrusted contributor.

- Don't assume the sibling's tests cover the surface you're touching.
- Don't assume the sibling's CI runs the audits you'd expect.
- Run the relevant audits manually if CI doesn't.

## Templates

- [`crossfix-commit.tmpl`](crossfix-commit.tmpl) — commit message template.
- [`crossfix-pr.tmpl`](crossfix-pr.tmpl) — PR description template.

## Annotations

Every crossfix carries a `CROSSFIX:` annotation in the affected file or
function:

```rust
// CROSSFIX: PlausiDen-Engine — same off-by-one in chunk boundary appeared
// in PlausiDen-Inject's similar parser; this fix mirrors the engine fix.
```
