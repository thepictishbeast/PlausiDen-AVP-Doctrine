# Harvest Integration Tier

**Added:** 2026-04-24 (v1.1).
**Applies to:** any artifact accepted upstream via the harvest protocol after a number of consumer adoptions.

## Question

Once accepted upstream, does the integration survive N consumer adoptions
without regression?

## Required after upstream acceptance

For the first 3 consumers that adopt the upstreamed artifact:

- Each consumer's CI passes against the doctrine's published version.
- No emergency amendment required within 30 days of consumer adoption.
- No consumer files an `EMERGENCY:` revert PR in the doctrine repo.

## Passing criterion

3 consumer adoptions × 30 days each, no regressions, no emergency amendments.

## Failing criterion

If 2+ consumers report the upstream version has worse behavior than their
local prior version, the artifact gets reverted to a candidate state and
re-evaluated. The original consumer's local copy is restored if removed.

This tier is what closes the harvest loop: it converts "accepted upstream"
into "proven in shared use."
