# examples/ — copy-paste templates for new repos

Templates used when bootstrapping a new PlausiDen repo, or when
bringing an existing repo into doctrine compliance. Each template is
opinionated — copy it, replace the `{{ … }}` placeholders, then add
repo-specific content below the framing block.

## Templates

| File | Goes at | Purpose |
|------|---------|---------|
| `CLAUDE-md-template.md` | `<repo>/CLAUDE.md` | The framing block every fresh agent session reads first. Identity, stop conditions, before/after-changes commands. |
| `ARCHITECTURE-md-template.md` | `<repo>/ARCHITECTURE.md` | Adds the explicit "Out of Scope" section that's missing in most architecture docs. Makes the ABSENCE of features readable at first glance. |

## Discipline

- The framing block sections in the CLAUDE.md template MUST appear
  in every PlausiDen repo's CLAUDE.md, in the order shown.
- The "Out of Scope" section in the ARCHITECTURE.md template MUST
  appear near the top of every PlausiDen repo's ARCHITECTURE.md
  (after identity, before crate map).
- Both are grep targets. Don't rename the headings — future audits
  (`framing-block-audit`, `out-of-scope-staleness`) will check for
  exact-string matches.

## Why opinionated templates?

The unstructured alternative ("just write a CLAUDE.md however you
want") burned the first ~6 months of the ecosystem on agents
re-deriving the same basic facts every session, contributors
adding features in the wrong layer because absence wasn't
documented, and reviewers explaining the same architectural seams
in PR comments instead of pointing at written doctrine.

Both templates collapse a recurring class of questions to a single
read at the top of a load-bearing file.
