# Loop Registry

> Every long-running loop on every host running PlausiDen agents lives here.
> Unregistered loops are forbidden and are killed by the compliance scan.

## How to register

1. Add an entry to the table below in your PR.
2. Include start command, owner agent, cancel command, stop conditions.
3. Get human review before merging the registration.

## Active loops

| Loop | Owner agent | Host | Started | Cadence | Cancel | Stop conditions |
|------|-------------|------|---------|---------|--------|-----------------|
| _none registered_ | _-_ | _-_ | _-_ | _-_ | _-_ | _-_ |

## Historical

Loops that have been retired. Keep the row so we have a record.

| Loop | Owner | Retired | Reason |
|------|-------|---------|--------|
| _none_ | _-_ | _-_ | _-_ |

## Compliance scan

```bash
bash scripts/check-compliance.sh --loops
```

Reports any running PlausiDen loop process not represented in this file.
