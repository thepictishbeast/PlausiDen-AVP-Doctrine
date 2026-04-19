# PlausiDen-AVP-Doctrine

> Standing orders, gates, and operational doctrine for every AI agent and
> human contributing to the PlausiDen ecosystem. This repo is the source of
> truth for how work is done; [PlausiDen-Audits](https://github.com/redcaptian1917/PlausiDen-Audits)
> is the catalog of named gates that enforce it.

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
| [`crons/`](crons/) | Cron snippets for scheduled routines — daily/weekly audits, IPC checks, drift sweeps. |
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

- [LFI / lfi_vsa_core](https://github.com/redcaptian1917/LFI)
- [PlausiDen-Engine](https://github.com/redcaptian1917/PlausiDen-Engine)
- [PlausiDen-Browser-Ext](https://github.com/redcaptian1917/PlausiDen-Browser-Ext)
- [PlausiDen-Sentinel](https://github.com/redcaptian1917/PlausiDen-Sentinel)
- [PlausiDen-Inject](https://github.com/redcaptian1917/PlausiDen-Inject)
- [PlausiDen-Firewall](https://github.com/redcaptian1917/PlausiDen-Firewall)
- [PlausiDen-Purge](https://github.com/redcaptian1917/PlausiDen-Purge)
- [PlausiDen-Sentinel](https://github.com/redcaptian1917/PlausiDen-Sentinel)
- [PlausiDen-Shard](https://github.com/redcaptian1917/PlausiDen-Shard)
- [PlausiDen-Swarm](https://github.com/redcaptian1917/PlausiDen-Swarm)
- [PlausiDen-Tidy](https://github.com/redcaptian1917/PlausiDen-Tidy)
- [PlausiDen-AppGuard](https://github.com/redcaptian1917/PlausiDen-AppGuard)
- [PlausiDen-Atrium](https://github.com/redcaptian1917/PlausiDen-Atrium)
- [PlausiDen-USB](https://github.com/redcaptian1917/PlausiDen-USB)
- [PlausiDen-Desktop](https://github.com/redcaptian1917/PlausiDen-Desktop)
- [PlausiDen-Android](https://github.com/redcaptian1917/PlausiDen-Android)
- [PlausiDen-OS-for-Mobile](https://github.com/redcaptian1917/PlausiDen-OS-for-Mobile)
- [PlausiDen-MCP](https://github.com/redcaptian1917/PlausiDen-MCP)
- [Testing-Framework](https://github.com/redcaptian1917/Testing-Framework)
- [Vulnerability-Scanner](https://github.com/redcaptian1917/Vulnerability-Scanner)
- And anything else carrying the PlausiDen prefix.

## Installing doctrine into a new repo

```bash
# From within the target repo
curl -fsSL https://raw.githubusercontent.com/redcaptian1917/PlausiDen-AVP-Doctrine/main/scripts/install-doctrine.sh | bash
```

This drops a `CLAUDE.md` referencing the doctrine, a `.github/workflows/avp.yml`
that runs the appropriate audit routines, and a `docs/AVP-LINK.md` pointing
contributors at the doctrine.

## License

MIT. See [LICENSE](LICENSE).
