# The Sovereign Polyglot Stack

**A best-language-per-domain reference. Snapshot — not prophecy.**

Version: 2026.05 · Maintainer: William Armstrong / PlausiDen
Review cadence: **quarterly** (next review due 2026.08; see `crons/sovereign-stack-review.toml`)

Language dominance churns on a ~10–20yr cycle; treat every "best" below as a current estimate, not a permanent fact. **Re-run the calculation each time you update this doc** — the criterion is *best language for a specific use case under the sovereignty filter*, not nostalgia, not default, not ecosystem gravity alone.

---

## Selection axes

1. **Fitness** — is it actually the best tool for the domain, ignoring everything else?
2. **Governance / capture risk** — who controls it? Single vendor (high risk) → foundation (low risk) → captive platform (forced, mitigate). This is a first-class axis here, not a footnote.
3. **Maturity** — production-ready, or watchlist?

Legend: 🟢 FOSS + foundation/community-governed · 🟡 FOSS but single-vendor-steered · 🔴 captive/closed or vendor-gatekept platform · ⏳ pre-1.0 / not load-bearing yet

The through-line: **logic lives in Rust; captive platform languages are thin, swappable UI shells (UniFFI). Interfaces (WASM, protobuf, SQL, wire protocols) are the durable unit — not languages.**

---

## Layer 0 — Silicon & Hardware

| Domain | Primary | Fallback / Legacy | Watch | Notes |
|---|---|---|---|---|
| RTL / chip design | SystemVerilog 🔴 (IEEE std) | VHDL 🔴 | **Chisel** 🟢 (Scala-based, runs RISC-V/SiFive), Amaranth 🟢 (Python), Veryl 🟢 ⏳ | FOSS toolchain exists: Yosys + Verilator + nextpnr. The open-silicon path. |
| FPGA | SystemVerilog / VHDL | — | Chisel, SpinalHDL 🟢 | Same toolchain story. |
| HW verification | SystemVerilog + UVM 🔴 | — | SymbiYosys 🟢 (formal) | Formal verification of RTL is FOSS-viable now. |
| GPU kernels | CUDA C++ 🔴 (NVIDIA-captive) | OpenCL | **wgpu/WGSL** 🟢 (Rust), ROCm/HIP 🟡, Mojo 🔴 | CUDA is the deepest moat in computing. wgpu is the sovereign escape, slower today. |

## Layer 1 — Firmware, Bare-Metal, Embedded

| Domain | Primary | Fallback / Legacy | Watch | Notes |
|---|---|---|---|---|
| MCU / bare-metal | **Rust** 🟢 (embassy async) | C 🟢 (widest vendor support) | Zig 🟢 ⏳ | Rust is the future; C still wins raw breadth of vendor toolchains. |
| RTOS | C 🟢 (Zephyr, FreeRTOS) | — | Rust-on-Zephyr | — |
| Safety-critical (avionics/auto/med) | **Ada/SPARK** 🟢 (provable, DO-178C) | MISRA C | **Rust via Ferrocene** 🟢 (now ISO 26262 / IEC 61508 qualified) | Qualified Rust toolchain is the genuinely new thing. SPARK if you need machine-checked proofs. |
| DSP | C/C++ + intrinsics | — | Rust | — |

## Layer 2 — OS / Kernel

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Kernel | C 🟢 (Linux) + **Rust** 🟢 (mainline) | — | Redox OS 🟢 (Rust microkernel) | Rust-for-Linux is merged; the safe-systems debate is over. |
| Microkernel / formal | C + **Isabelle/HOL** proofs (seL4) | — | Rust + capability IPC | The proof corpus is the asset. Don't rewrite seL4; build on its guarantees. (PlausiDenOS target.) |

## Layer 3 — Systems & Performance-Critical

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Systems / perf services | **Rust** 🟢 | C++ 🟢 (where ecosystem forces it: games, HPC, legacy) | Zig ⏳ | Default. |
| Crypto / security tooling | **Rust** 🟢 | C (libsodium, audited) | F*/hax 🟢 (Rust→F* extraction) | "Proven" > "tested" for an adversarial product. |

## Layer 4 — Backend, Servers, Concurrency

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Concurrent / fault-tolerant / realtime | **Elixir/Erlang (BEAM)** 🟢 | — | **Gleam** 🟢 (typed BEAM, 1.0) | Nothing else has the supervision/distribution model. Gleam = OTP + static types; the typed-BEAM future. |
| General API / web backend | **Rust** (Axum) 🟢 | Elixir/Phoenix, Gleam, Go 🟡 | — | CPU/latency-bound → Rust; connection-bound → BEAM. |
| Ops/infra services | **Go** 🟡 | Rust | — | The k8s/cloud-native ecosystem is Go; fight it only with reason. |
| Wire protocols | protobuf/gRPC, Cap'n Proto 🟢 | — | — | The durable interface layer. Language-agnostic by design. |

## Layer 5 — Data & Databases

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Querying data | **SQL** 🟢 | — | — | The single most future-proof language in existence. 50yr, outlived every paradigm. Bet hard on it. |
| Stored procedures | PL/pgSQL 🟢 (Postgres) | — | — | Your CRM/Salesman backend. |
| Building a storage engine | **Rust** 🟢 | C++ 🟢 (Postgres/SQLite legacy) | — | New DBs are overwhelmingly Rust. |
| Embedded DB | SQLite (C) 🟢 | — | — | Most-deployed DB on Earth. |
| Advanced query / reasoning | Datalog 🟢 | — | — | For graph/recursive queries; relevant to neurosymbolic work. |

## Layer 6 — Data Engineering, ML, Scientific

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| ML/AI research & training | **Python** 🟢 (PyTorch/JAX) | — | — | Lingua franca. Unavoidable. Don't fight it; wrap it. |
| Production ML inference | **Rust** 🟢 (candle, burn) | Python | Mojo 🔴 (disqualified) | Fits LFI's Rust HDC core directly. |
| Scientific / numerical | **Julia** 🟢 | Python | — | Modern FOSS challenger; fast, MIT. |
| Dense numerics / HPC | **Fortran** 🟢 | C++ | Julia | Not a joke — still optimal for dense linear algebra/HPC. The incumbent that refuses to die because it's correct. |
| Dataframes / analytics | SQL + **Polars** 🟢 (Rust) | pandas | — | — |
| Mojo status | — | — | — | 🔴 1.0 Beta (May 2026) but **compiler is closed** (Modular Community License), single-vendor. Technically exciting, fails the FOSS/PSA filter. Revisit only if the compiler is OSI-licensed + foundation-governed. |

## Layer 7 — Mobile (gatekept — mitigate)

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Shared logic core | **Rust + UniFFI** 🟢 | — | — | **The sovereign move.** Real logic in Rust, exposed to both platforms. Native languages become thin shells. |
| Android UI | Kotlin 🟡 (JetBrains/Google) | Java | — | Apache-licensed but Google-steered platform. |
| iOS UI | Swift 🔴 (Apple-gatekept) | Obj-C | — | Swift-the-language is open; the platform/tooling is captive. Highest capture risk in the stack. |
| Cross-platform (one codebase) | **Flutter/Dart** 🟡 or KMP 🟡 | React Native/TS | Dioxus 🟢 (Rust) | Flutter is BSD-FOSS but Google-governed — license isn't the risk, governance capture is. |

## Layer 8 — Desktop

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Cross-platform (sovereign) | **Tauri** 🟢 (Rust core + web UI) | Qt 🟢/🟡 (C++) | Dioxus 🟢, Slint 🟢 | Tiny binaries, Rust core. Avoid Electron (bloat). |
| Linux native | C/C++ (GTK/Qt) 🟢 | Rust (gtk-rs) | Slint | — |
| Windows native | C# / .NET 🟡 (MIT, MS-steered) | C++ Win32 | — | .NET is FOSS now; still MS-directed. |
| macOS native | Swift/SwiftUI 🔴 | — | — | Captive. |

## Layer 9 — Web Frontend (captive platform: the browser)

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Production web UI | **TypeScript** 🟢 + Svelte/Solid | React | — | The browser is a captured runtime; TS is the floor of sanity. Treat as legacy-you-can't-escape-yet. |
| Sovereign / WASM future | **Rust→WASM** 🟢 (Leptos, Dioxus) | Gleam→JS | WASM Component Model 🟢 | Migration target off React. Push logic into WASM behind a stable component boundary. (Sacred.Vote's TS/React stays until this matures.) |

## Layer 10 — CLI, Scripting, Shell

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| CLI tools | **Rust** (clap) 🟢 | **Go** (cobra) 🟡 | — | Genuine tie. Most cloud CLIs are Go; Rust wins on perf/correctness. |
| Glue / automation | **Python** 🟢 | — | — | Ecosystem reach. Unavoidable. |
| Shell scripting | Bash 🟢 (portability floor) | — | **Nushell** 🟢 (Rust, structured data), fish | Nushell is the modern structured-data shell. |

## Layer 11 — Infra-as-Code & Config

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Reproducible systems | **Nix / NixOS** 🟢 | — | — | The supersociety answer. Declarative, reproducible, FOSS. Perfect fit for self-hosted sovereign infra. |
| Cloud provisioning | **OpenTofu** 🟢 (HCL) | — | — | **Use OpenTofu, not Terraform.** HashiCorp's BSL relicense is the exact vendor-capture event your filter exists to prevent; OpenTofu is the Linux Foundation FOSS fork. Case study in why governance is a selection axis. |
| Typed config | **Nickel** 🟢 (Rust/Nix ecosystem) | CUE 🟢, Dhall 🟢 | — | Escape YAML. |

## Layer 12 — Blockchain / ZK

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Chain logic / zkVM | **Rust** 🟢 (SP1, RISC Zero, Substrate, Solana) | — | — | Already your stack. Correct. |
| EVM contracts | Solidity 🟢 | Vyper 🟢 | — | Unavoidable for Ethereum. |
| ZK circuits | **Noir** 🟢, **Cairo** 🟢 (Starknet/STARK) | Circom, Halo2 (Rust) | — | Cairo aligns with your STARK-only/SP1 posture. |

## Layer 13 — Formal Methods & Correctness

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Theorem proving / verified programs | **Lean 4** 🟢 | Rocq (Coq) 🟢, Agda 🟢, Idris 2 🟢 | — | Lean is now a real programming language, not just a prover. |
| Verified systems code | **Verus / Creusot / Aeneas** 🟢 (Rust) | SPARK/Ada 🟢 | — | Prove your actual production code, not a model of it. |
| Protocol / consensus design | **TLA+** 🟢 | Quint 🟢 (modern TLA+), Alloy | — | **Spec the dual-chain consensus and Sacred.Vote tally protocol in TLA+/Quint before writing Rust.** Catches the bugs tests never will. |

## Layer 14 — Security / RE / Offensive

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| Tooling | **Rust** 🟢 / C | — | — | — |
| Exploit PoC / scripting | Python 🟢 | — | — | — |
| Reverse engineering | Assembly (x86/ARM) + Ghidra 🟢 (Java platform) | — | — | — |

## Layer 15 — Game Dev (if ever relevant)

| Domain | Primary | Fallback | Watch | Notes |
|---|---|---|---|---|
| FOSS engine | **Godot/GDScript** 🟢 or **Bevy/Rust** 🟢 | — | — | Avoid Unity (proprietary, licensing-volatile) and Unreal/C++ unless AAA. |

---

## The irreducible core

If you stripped this to the minimum set that covers ~90% of all development, sovereignty-weighted:

- **Rust** — systems, embedded, crypto, backend, CLI, ML-inference, ZK, WASM, desktop core. The spine.
- **Python** — ML/AI and glue. Wrapped, not loved. Unavoidable.
- **SQL** — all data. The most durable language alive.
- **Elixir / Gleam (BEAM)** — concurrency & fault tolerance.
- **TypeScript** — browser floor, until WASM displaces it.
- **C** — firmware/kernel incumbent.
- **Lean 4 + TLA+** — correctness frontier.
- **Nix** — reproducible infra.
- **Kotlin + Swift** — mobile UI shells only, forced by gatekeepers, kept thin over a Rust core.

That's the honest floor: ~9 languages, three of them (TS, Swift, Kotlin) forced on you by captured platforms rather than chosen.

## Risk / steel-man

The strongest objection: a sovereignty-weighted stack systematically *under-weights ecosystem gravity*, and ecosystem is itself a material force. Choosing OpenTofu over Terraform, wgpu over CUDA, or Tauri over Electron is correct on governance grounds and pays a real, ongoing tax in tooling maturity, talent availability, and integration friction. Solo, that tax is affordable and the sovereignty is worth more. The moment there's a team or a delivery deadline that the FOSS option can't hit, the stack has to be re-derived against the new material conditions — the captured tool sometimes wins because the cost of avoiding it exceeds the cost of depending on it. The discipline isn't "always pick FOSS"; it's "price the capture risk honestly and pay it deliberately, not by default." Re-run that calculation each time you update this doc.

---

## Update protocol

This document is **load-bearing** but **time-sensitive**. The review cadence:

1. **Quarterly review** — first Monday of Feb / May / Aug / Nov. Walk each layer, ask three questions per row:
   - Has fitness shifted? (new language matured, old one regressed)
   - Has governance shifted? (relicense, vendor capture, foundation handoff)
   - Has a watch-list entry become production-ready?
2. **Trigger-based review** — any time one of these events happens, schedule a review even mid-cycle:
   - A language we depend on relicenses to a non-FOSS license (HashiCorp / Elastic / MongoDB precedent)
   - A watch-list language hits 1.0
   - A new vendor-captive platform enters our deployment surface
   - A formal verification capability lands in Rust we've been waiting on
3. **Update mechanics** — bump `Version:` at top (YYYY.MM). Note material changes in a `## Change log` section at bottom. Commit with `doctrine(sovereign-stack):` prefix. Push to GitHub so downstream agents and humans see the diff.
4. **Cadence enforcement** — `crons/sovereign-stack-review.toml` schedules a reminder; review or explicitly defer in writing.

## Change log

- **2026.05** — initial publication. Sets the baseline. Mojo gets a 🔴 disqualification (closed compiler); OpenTofu replaces Terraform; Ferrocene moves Rust into safety-critical-qualified territory; Lean 4 promoted from prover-only to general programming language.
