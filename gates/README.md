# Coding Gates

Gates that block commits and merges. Each language has its own file with
language-specific rules. The cross-language rules live here.

| Language | File |
|----------|------|
| Rust | [`rust.md`](rust.md) |
| TypeScript / JavaScript | [`typescript.md`](typescript.md) |
| Python | [`python.md`](python.md) |
| Frontend (HTML/CSS) | [`frontend.md`](frontend.md) |

## Cross-language rules

These apply to every language and every surface.

- **No commits with failing tests.** A red test does not get a "fix later"
  commit; either fix it now or revert the change that broke it.
- **No commits with secrets.** `gitleaks` / `trufflehog` is the second
  defense; the first is the agent's own check.
- **No commits with TODO/FIXME without an issue or assignment.** A TODO
  without an owner is dead weight.
- **No commits that bypass hooks (`--no-verify`).** If a hook fails,
  investigate; don't skip.
- **No commits that disable tests** (`#[ignore]`, `it.skip`, `xtest`,
  `pytest.mark.skip`) without a `REGRESSION-GUARD:` and an issue link.
- **No commits that introduce panics, unwraps, or `as_unchecked`** in
  library code without a `SAFETY:` annotation.
- **No commits with mass auto-formatter changes mixed in.** Format-only
  diffs go in their own commit so the substantive diff stays reviewable.
- **No commits that reduce code coverage in core crates** without an
  explicit `SHIP-DECISION:`.

## Enforcement

The gates above are enforced by:

1. **Local pre-commit hook** (installed by `scripts/install-doctrine.sh`).
2. **CI workflow** (`pre-commit` and `pre-merge` routines from
   PlausiDen-Audits).
3. **Branch protection** on `main` (server-side enforcement, can't be
   bypassed by an agent).

The hook + CI + protection form three layers — supersociety means no single
defense.
