# Harvest Readiness Tier

**Added:** 2026-04-24 (v1.1).
**Applies to:** any artifact a consumer wants to propose for upstream adoption.

## Question

Is this artifact ready to be proposed upstream via the harvest protocol?

## Checklist

- [ ] Listed in the consumer's `harvest.toml` with `status >= in-use-internal`.
- [ ] `adoption_evidence` field populated with concrete usage data (line counts, time in production, user-facing impact).
- [ ] `generality_assessment` field populated with the two-unrelated-projects argument.
- [ ] `doctrine_alignment` field declares which doctrine tenets the artifact obeys.
- [ ] `artifact_path` is a stable repo-relative path; not a temporary scratch location.
- [ ] `rationale` field explains *why* this should be doctrine, not just *what* it does.

## Passing criterion

All checklist items present and verifiable from public interfaces only
(Harvest Doctrine tenet 2). The maintainer of the proposing consumer signs.

## Output

A passing artifact appears in the next harvest run's candidate report. From
there, it enters the **Generality Tier** + **Harvest Integration Tier** flow.
