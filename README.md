<!-- repo-label: meta-doctrine -->
<!-- repo-class: validation-protocol-and-agent-standing-orders -->
<!-- repo-consumes: nothing (axiom floor; see PlausiDen-Meta/AXIOM_FLOOR.md) -->
<!-- repo-consumed-by: every PlausiDen-* repo (advisory grading) + every PlausiDen agent (standing orders) -->

# PlausiDen-AVP-Doctrine

> The validation protocol every PlausiDen-namespace artifact is graded
> against before release, plus the standing orders for every AI agent and
> human contributing to the ecosystem.
>
> Sister repos: [`PlausiDen-Meta`](https://github.com/thepictishbeast/PlausiDen-Meta)
> (priority gate + governance), [`PlausiDen-Audits`](https://github.com/thepictishbeast/PlausiDen-Audits)
> (catalog enforcing this protocol).

## Why this exists

> "I want this AVP to manage you so I don't have to."

An agent operating under this doctrine should produce supersociety-grade work
without the human babysitting every commit. Agents read [`standing-orders/`](standing-orders/)
at session start, run [`crons/`](crons/) and [`loops/`](loops/) for routine
work, gate every commit on [`gates/`](gates/), annotate every line per
[`annotations/`](annotations/), absorb FOSS per
[`foss-absorption/`](foss-absorption/), contribute across siblings per
[`cross-repo/`](cross-repo/), and write `SHIP-DECISION:` annotations per
[`ship-decision/`](ship-decision/).

## Layout

| Folder | Purpose |
|--------|---------|
| [`AVP2_PROTOCOL.md`](AVP2_PROTOCOL.md) | The full Adversarial Validation Protocol v2 — the source the rest derives from. |
| [`standing-orders/`](standing-orders/) | What each agent reads at session start. Roles, allowed/forbidden actions, IPC, escalation, stop conditions. |
| [`gates/`](gates/) | Per-language coding gates that block commits and merges. |
| [`annotations/`](annotations/) | The inline annotation standard (`BUG ASSUMPTION`, `AVP-PASS-N`, `SAFETY`, `SHIP-DECISION`, …). |
| [`foss-absorption/`](foss-absorption/) | The 6-step protocol for vendoring and hardening FOSS instead of using as-is. |
| [`cross-repo/`](cross-repo/) | How fixes flow between sibling repos (`AVP-CROSSFIX`). |
| [`ship-decision/`](ship-decision/) | Templates for the ship verdict (always "STILL BROKEN"). |
| [`prompts/`](prompts/) | Reusable AI agent prompt templates (new-feature, bug-fix, code-review, audit-self). |
| [`crons/`](crons/) | Cron snippets for non-systemd hosts. |
| [`timers/`](timers/) | Systemd system timers (the default on systemd hosts — daily/weekly audits, heartbeat, IPC archive, FOSS drift). |
| [`loops/`](loops/) | Long-running loop scripts (audit-loop, ipc-poll, sprint-progress) and the loop **registry**. |
| [`examples/`](examples/) | Concrete examples of doctrine application in real PlausiDen work. |
| [`scripts/`](scripts/) | Helper scripts: install doctrine into a repo, check repo compliance. |

## How an agent uses this

1. **Session start:** read the standing-orders file matching this agent's role.
   Re-read on every context compaction.
2. **Before any code change:** identify which `gates/` apply.
3. **Before any commit:** run the appropriate audit routine from
   PlausiDen-Audits (see `routines/`).
4. **Before push:** verify+test+audit against the audits this surface declares.
5. **Long-running work:** install the relevant loop from `loops/` or schedule
   the relevant cron from `crons/`. Register in `loops/REGISTRY.md`.
6. **Cross-repo bug:** apply the cross-repo protocol — fix the sibling first,
   tag with `AVP-CROSSFIX`, return to original task.
7. **Shipping:** write the `SHIP-DECISION:` annotation listing accepted
   residual risks. The verdict is always "STILL BROKEN."

## What this governs

Any project that adopts this doctrine. The doctrine itself enumerates no
specific consumers — per [`PlausiDen-Meta/SCOPE.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/SCOPE.md)
it applies to **any project, in any language, on any platform** that
declares adoption.

A non-normative ecosystem index (which projects have adopted this doctrine,
their tier targets, status) lives in
[`PlausiDen-Meta/REPO_LABEL_REGISTRY.md`](https://github.com/thepictishbeast/PlausiDen-Meta/blob/main/REPO_LABEL_REGISTRY.md).
That index is informational; this doctrine's design is independent of which
consumers happen to exist at any given time.

## Installing doctrine into a new repo

```bash
# From within the target repo
curl -fsSL https://raw.githubusercontent.com/thepictishbeast/PlausiDen-AVP-Doctrine/main/scripts/install-doctrine.sh | bash
```

This drops a `CLAUDE.md` referencing the doctrine, a `.github/workflows/avp.yml`
that runs the appropriate audit routines, and a `docs/AVP-LINK.md` pointing
contributors at the doctrine.

## License

MIT. See [LICENSE](LICENSE).
