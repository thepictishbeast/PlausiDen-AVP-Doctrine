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

## Repos this governs

- [LFI / lfi_vsa_core](https://github.com/thepictishbeast/LFI)
- [PlausiDen-Engine](https://github.com/thepictishbeast/PlausiDen-Engine)
- [PlausiDen-Browser-Ext](https://github.com/thepictishbeast/PlausiDen-Browser-Ext)
- [PlausiDen-Sentinel](https://github.com/thepictishbeast/PlausiDen-Sentinel)
- [PlausiDen-Inject](https://github.com/thepictishbeast/PlausiDen-Inject)
- [PlausiDen-Firewall](https://github.com/thepictishbeast/PlausiDen-Firewall)
- [PlausiDen-Purge](https://github.com/thepictishbeast/PlausiDen-Purge)
- [PlausiDen-Sentinel](https://github.com/thepictishbeast/PlausiDen-Sentinel)
- [PlausiDen-Shard](https://github.com/thepictishbeast/PlausiDen-Shard)
- [PlausiDen-Swarm](https://github.com/thepictishbeast/PlausiDen-Swarm)
- [PlausiDen-Tidy](https://github.com/thepictishbeast/PlausiDen-Tidy)
- [PlausiDen-AppGuard](https://github.com/thepictishbeast/PlausiDen-AppGuard)
- [PlausiDen-Atrium](https://github.com/thepictishbeast/PlausiDen-Atrium)
- [PlausiDen-USB](https://github.com/thepictishbeast/PlausiDen-USB)
- [PlausiDen-Desktop](https://github.com/thepictishbeast/PlausiDen-Desktop)
- [PlausiDen-Android](https://github.com/thepictishbeast/PlausiDen-Android)
- [PlausiDen-OS-for-Mobile](https://github.com/thepictishbeast/PlausiDen-OS-for-Mobile)
- [PlausiDen-MCP](https://github.com/thepictishbeast/PlausiDen-MCP)
- [Testing-Framework](https://github.com/thepictishbeast/Testing-Framework)
- [Vulnerability-Scanner](https://github.com/thepictishbeast/Vulnerability-Scanner)
- And anything else carrying the PlausiDen prefix.

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
