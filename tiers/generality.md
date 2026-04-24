# Generality Tier

**Added:** 2026-04-24 (v1.1).
**Applies to:** any artifact proposed for inclusion in a doctrine repo (rule, harness, vocabulary field, contract template).

## Question

Does this artifact apply in at least two unrelated hypothetical projects, or
is it specific to the proposing consumer?

## Required evidence

The proposer (consumer) supplies:

- A `generality_assessment` field in their `harvest.toml` candidate entry.
- A worked example of the artifact applied to a project unlike the proposing one.

The doctrine maintainer adds:

- A second worked example, independently constructed.
- A signed acceptance: "I confirm this artifact is general."

## Passing criterion

Two unrelated worked examples + maintainer sign-off.

## Failing criterion

If the artifact only fits the proposing consumer's domain, **reject** with
preserved rationale (per [`PlausiDen-Harvest`](https://github.com/thepictishbeast/PlausiDen-Harvest)
doctrine tenet 5). Do not promote it to doctrine.

The artifact remains useful in the proposing consumer's local rule set; it
just doesn't ascend to shared doctrine.
