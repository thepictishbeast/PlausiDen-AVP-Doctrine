# avp — the AVP-2 supersociety toolchain

`avp` is the canonical Rust binary that every PlausiDen sibling repo uses to
enforce the [AVP-2 supersociety protocol](../AVP2_PROTOCOL.md). One static-musl
binary, one config surface, one set of gates — replacing the bash/python
sprawl that historically governed the portfolio.

## Status

`v0.1.0-dev` — workspace scaffold, ratchet types, CLI scaffolding. The Tier-1
Rust gates (`avp check rust`) implement next.

## Layout

```
avp/
├── Cargo.toml                       workspace root + canonical deps
├── rust-toolchain.toml              channel pin
├── deny.toml                        cargo-deny baseline (also synced to siblings)
├── clippy.toml                      clippy tuning (also synced)
├── rustfmt.toml                     formatter config (also synced)
├── .cargo/config.toml               build-level flags + cargo aliases
└── crates/
    ├── avp-core/                    library — types, traits, gate framework
    └── avp/                         binary — CLI, subcommand dispatch
```

## Subcommands (planned)

| Command | Purpose | Status |
|---|---|---|
| `avp check rust [--workspace] [--ratchet F] [--strictness ...]` | Run AVP Tier-1 gates against a Rust workspace. | dev |
| `avp check ts \| py` | Same for TypeScript / Python siblings. | planned |
| `avp ratchet validate \| list \| preflight \| add` | Manage `avp-ratchet.toml` overrides. | partial |
| `avp drift [--root DIR] [--open-issues]` | Detect drift across the PlausiDen portfolio. | planned |
| `avp install [--dry-run] [--force]` | Drop the canonical `.github/workflows/avp.yml` + configs into a sibling repo. | planned |
| `avp gate list` | Print every gate the binary enforces, with descriptions. | dev |
| `avp explain <gate>` | Print the doctrine rationale for a given gate. | planned |
| `avp intent claim \| overlap \| merge-order \| verify` | Multi-instance coordination over `.avp-intent.toml`. | planned |

## Local development

```sh
cd /home/user/Development/PlausiDen/PlausiDen-AVP-Doctrine/avp
cargo build --release
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# convenience: the cargo aliases in .cargo/config.toml
cargo avp-check     # ./target/release/avp check rust
cargo avp-deny      # cargo-deny against this workspace
```

## Self-hosting

`avp` is governed by `avp`. The doctrine repo's own CI runs `avp check rust`
against this workspace via `.github/workflows/avp.yml`. New gates land here
first; siblings opt-in by tag bump.

## Versioning

- `0.x.y` = pre-stable. Breaking changes can land in any minor.
- Tags are signed (`git tag -s vX.Y.Z`). Releases publish a static-musl binary +
  blake3 manifest to GitHub Releases.
- Composite GH Actions in sibling repos pin by tag (`@v0.1.0`), never by branch.
