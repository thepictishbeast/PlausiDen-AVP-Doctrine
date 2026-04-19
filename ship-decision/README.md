# Ship Decision

> The verdict is always **STILL BROKEN**. Shipping is explicit risk
> acceptance, not a declaration of correctness.

After a minimum of 36 AVP-2 passes the agent may interrupt the loop with a
`SHIP-DECISION:` annotation listing accepted residual risks, the mutant
survival rate, the coverage number, and the human signer.

Then the loop resumes on the next commit.

## When a SHIP-DECISION is required

- A required audit failed but the residual risk is judged acceptable for
  the milestone.
- A known CVE in a dep cannot be patched yet; the dep is not on the
  exploit path *for this surface*.
- A regression test cannot be written for a closed bug because the
  reproduction is environmental; the manual repro is documented and the
  alarm is wired to catch a recurrence.

## When a SHIP-DECISION is NOT acceptable

- A real secret reached production. Rotate, don't waive.
- An accessibility regression on a previously-passing flow. Fix, don't waive.
- An adversary-class finding without a documented response. Fix, don't waive.
- "I don't have time" without an estimate of the residual risk and an owner
  for the follow-up.

## Format

Use the [template](template.md). The annotation must include:

- Date (absolute, ISO-8601).
- Audits waived (slug list).
- Accepted residual risks (one bullet each).
- Owner (the human who signed; agents may draft, humans must sign).
- Follow-up issue link (where the residual risk is being tracked).

## Lifetime

A `SHIP-DECISION:` is valid for one milestone unless explicitly extended.
The next milestone re-runs the audit; if the same waiver is needed,
re-sign deliberately.

## Storage

- Inline in the file the decision affects (so `rg 'SHIP-DECISION:'` finds it).
- Mirrored in the release notes / CHANGELOG entry.
- Mirrored in `audits/<slug>/findings/<date>-ship.md` in PlausiDen-Audits.
