# avp-rust-gate

Composite GitHub Action that runs the pinned `avp` binary against a
PlausiDen sibling repo. One `uses:` line replaces the boilerplate that
every sibling currently copy-pastes (and drifts away from).

## Usage

```yaml
# .github/workflows/avp.yml in any sibling repo
name: avp
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

concurrency:
  group: avp-${{ github.ref }}
  cancel-in-progress: true

jobs:
  rust-gate:
    runs-on: [self-hosted, linux, x64, plausiden]
    steps:
      - uses: actions/checkout@v4
      - uses: thepictishbeast/PlausiDen-AVP-Doctrine/.github/actions/avp-rust-gate@v0.1.0
        with:
          version: v0.1.0
          strictness: doctrine        # or "minimal" pre-cleanup
          test-density-min: "4.0"
        env:
          GITHUB_TOKEN: ${{ secrets.AVP_DOCTRINE_TOKEN }}   # required while the doctrine repo is private
```

## Why a binary shim?

- **Zero `cargo install` at workflow time.** The Kali self-hosted runner's
  `cargo install --locked cargo-audit` flakes on rust-src corruption (see
  Engine's parked `security.yml` AVP-PASS comment); we sidestep that
  whole class of failure by shipping a pinned binary.
- **Reproducibility.** The release pipeline cross-compiles
  `x86_64-unknown-linux-musl` static-link with `lto = fat`, publishes a
  blake3 manifest. Every sibling runs the *same* binary; drift is only
  possible by tag bump.
- **No language churn.** Sibling repos that aren't Rust still get the
  doctrine — `avp check ts` (next) and `avp check py` (after) ship in
  the same binary.

## Private-repo access

`PlausiDen-AVP-Doctrine` is currently private. To let other repos `uses:`
this action:

1. **Action sharing**: in the doctrine repo, *Settings → Actions →
   General → Access* set to **"Accessible from repositories owned by
   the user"**.
2. **Release-asset download**: pass a PAT with `read:packages` (and
   `repo` if needed) as `GITHUB_TOKEN` in the calling workflow's `env`.
   Without it, `install-avp.sh` can't reach the release API.

When the doctrine repo flips public, the token requirement disappears.

## Inputs

| Name | Default | What |
|---|---|---|
| `version` | *(required)* | Tag to download. Must match an existing GH Release (e.g. `v0.1.0`). |
| `strictness` | `doctrine` | `doctrine` = `-D warnings -D clippy::pedantic -D clippy::nursery`. `minimal` = `-D warnings` only. |
| `ratchet` | `avp-ratchet.toml` | Path to per-repo override file (relative to repo root). |
| `test-density-min` | `"4.0"` | Aggregate test-density floor. |
| `format` | `auto` | `auto` (gh-actions in CI), `github-actions`, `human`, `json`. |

## Pinning

Pin the `uses:` ref to a tag, not a branch:

```yaml
uses: thepictishbeast/PlausiDen-AVP-Doctrine/.github/actions/avp-rust-gate@v0.1.0
```

Pinning to `@main` is forbidden by doctrine — a future commit could
re-shape the action and break consumers silently.

## Local dry-run

The composite action is just a wrapper around the same binary you can
run locally:

```sh
TMPDIR=/home/user/avp-build-tmp \
  cargo run --bin avp --manifest-path /path/to/avp/Cargo.toml \
  -- check rust --root $(pwd)
```

For local CI parity, prefer using the published binary:

```sh
AVP_VERSION=v0.1.0 AVP_BIN=$(mktemp) \
  bash .github/actions/avp-rust-gate/install-avp.sh
"$AVP_BIN" check rust
```
