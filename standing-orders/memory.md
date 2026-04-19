# Memory Standing Order

> What goes in `~/.claude/projects/-/memory/`, what doesn't, and why.

## Layout

- `MEMORY.md` — the index. Always loaded into context. Every entry is one
  line under ~150 chars: `- [Title](file.md) — one-line hook`.
- `<topic>.md` — individual memory files, one per memory. Frontmatter
  declares `name`, `description`, `type` (`user` / `feedback` / `project` /
  `reference`).

## What goes in

| Type | When |
|------|------|
| `user` | Anything you learn about the user's role, preferences, knowledge, responsibilities. |
| `feedback` | Anything the user corrects ("don't") or confirms ("yes, keep doing that"). Include **why**. |
| `project` | Who is doing what, why, by when. Convert relative dates to absolute. |
| `reference` | Pointer to an external system (Linear board, Slack channel, Grafana dashboard). |

## What never goes in

- Code patterns, conventions, file paths, project structure (derive from the
  current state).
- Git history (use `git log` / `git blame`).
- Debug solutions / fix recipes (the fix is in the code; the commit message
  has the context).
- Anything already in CLAUDE.md.
- Ephemeral task state — use Plans / Tasks / TodoWrite, not memory.

## Discipline

1. **Save when you learn**, not at end of session.
2. **Update or remove** memories that turn out wrong or stale. Don't
   accumulate contradictions.
3. **Verify before acting on a memory.** A memory naming a file/function/flag
   is a claim that it existed *when written*. Grep before recommending.
4. **No duplicates.** Update the existing memory; don't append a new one.
5. **Memory is across-session.** In-session state belongs in Plan / Tasks.

## Stop signal interaction

If the user says "ignore memory" / "don't use memory": do not apply, cite,
compare against, or mention memory content in this session.
