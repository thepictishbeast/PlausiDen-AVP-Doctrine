# ADVERSARIAL VALIDATION PROTOCOL v2 (AVP-2)
## Recursive Supersociety-Grade Development Methodology

> **Axiom 0:** The code is broken. The tests are broken. The tools are broken.
> The dependencies are broken. The OS is compromised. The compiler is lying.
> The adversary has your source code, your threat model, and unlimited resources.
> Build anyway. Prove everything. Trust nothing. Loop forever.

---

## 1. THREAT MODEL: STATE-ACTOR ADVERSARY

Every line of code is written against this adversary profile:

```
ADVERSARY CAPABILITIES (assume all simultaneously):
├── Has full source code access (assume repo is public even if private)
├── Has supply chain compromise capability (any dependency is a trojan)
├── Has hardware-level implants (CPU, NIC, RNG may be backdoored)
├── Has network MITM on all traffic (TLS termination, cert forgery)
├── Has zero-day exploits for every known software component
├── Has AI-assisted vulnerability discovery (faster than you patch)
├── Has social engineering operators (phishing, insider recruitment)
├── Has legal compulsion tools (NSLs, secret court orders, gag orders)
├── Has physical access capability (stolen device, evil maid)
├── Has unlimited compute for brute force / side channel analysis
├── Has temporal advantage (patient — will wait years for a window)
└── Is already inside. Assume breach is active NOW.
```

Every defensive decision must hold against this profile. "Nobody would bother"
is not a valid assessment. The adversary always bothers.

---

## 2. THE SUPERSOCIETY STACK

"Supersociety" means: solutions that don't exist yet in a single tool, so we
BUILD them by combining, hardening, and extending the best FOSS primitives.
Never accept a tool as-is. Every tool is raw material.

### Stack Philosophy
```
LAYER 0: Formal verification where possible (Kani, MIRI, TLA+)
LAYER 1: Memory-safe language enforcement (Rust, no unsafe without proof)
LAYER 2: Property-based + mutation testing (proptest, cargo-mutants)
LAYER 3: Continuous fuzzing (cargo-fuzz, AFL++, honggfuzz)
LAYER 4: Static analysis beyond clippy (semgrep, cargo-geiger, rudra)
LAYER 5: Dynamic analysis (valgrind, sanitizers, runtime assertions)
LAYER 6: Supply chain verification (cargo-vet, cargo-deny, sigstore)
LAYER 7: Reproducible builds (bit-for-bit, deterministic compilation)
LAYER 8: Defense-in-depth runtime (seccomp, landlock, capability dropping)
LAYER 9: Cryptographic integrity (Merkle audit logs, signed commits)
```

### FOSS Absorption Protocol
When the current task needs a capability:
1. **Search** for existing FOSS tools that solve the problem.
2. **Evaluate** the tool: audit its code, check its dependencies, review its
   CVE history, assess maintainer trust signals.
3. **Never use it as-is.** Fork or vendor it. Strip unnecessary features
   (attack surface reduction). Harden it:
   - Add bounds checking where missing
   - Replace any `unsafe` / raw pointer use with safe alternatives
   - Add comprehensive error handling where it panics or unwraps
   - Add logging/audit trails
   - Add the tests it should have had
4. **Integrate** the hardened version into the project.
5. **Loop through AVP on the absorbed tool itself** — it is now YOUR code
   and inherits YOUR paranoia. Minimum 12 passes on absorbed code.
6. **Contribute upstream** if fixes are generalizable — but never depend on
   upstream accepting. Your fork is canonical.

---

## 3. THE LOOP (MINIMUM 36 PASSES)

The loop runs a **minimum of 36 passes** before the developer may even
consider interrupting. Each pass has a distinct focus that escalates in
adversarial sophistication. The passes are grouped into TIERS.

### TIER 1: EXISTENCE PROOF (Passes 1–6)
*"Does this code do literally anything correctly?"*

**Pass 1 — Skeleton Audit**
- Read every function signature. Does the type system express the intent?
- Write `// BUG ASSUMPTION:` above every block.
- Write 1 happy-path test per public function. Run them. Expect failures.
- Fix failures. Do NOT assume fixes are correct.

**Pass 2 — Null/Zero/Empty Sweep**
- Every function gets a test with: empty string, zero, None/null, empty
  vec, empty map, default-constructed struct.
- Every unwrap/expect is replaced with proper error handling.
- Add `#[must_use]` to every function returning Result/Option.

**Pass 3 — Boundary Sweep**
- Every numeric input: test MIN, MAX, MIN-1, MAX+1, i64::MIN, u64::MAX.
- Every string input: test len=0, len=1, len=MAX_REASONABLE, len=10MB.
- Every collection: test len=0, len=1, len=2 (off-by-one), len=enormous.
- Every index/offset: test negative, zero, last, last+1.

**Pass 4 — Error Path Completeness**
- For every Result-returning call in the codebase: write a test that forces
  the Err path. If you cannot force it, the error handling is untestable
  and must be redesigned.
- Every error message is reviewed: does it leak internal state? Fix it.
- Every error is logged with sufficient context to diagnose without
  reproducing.

**Pass 5 — Type Tightening**
- Replace every `String` parameter that has constraints with a newtype.
- Replace every `usize`/`i32` that has a valid range with a newtype.
- Replace every `bool` parameter with an enum (no "boolean blindness").
- Add `#[non_exhaustive]` to every public enum.
- Add `#[deny(missing_docs)]` to every public module.

**Pass 6 — Dependency Audit**
- `cargo tree --duplicates` — eliminate all duplicate dependency versions.
- `cargo deny check` — licenses, advisories, bans.
- `cargo audit` — known vulnerabilities.
- `cargo geiger` — count unsafe in dependency tree. Flag anything alarming.
- `cargo vet` — if not yet initialized, set it up now.
- For every dependency with >100 transitive deps: evaluate if it's worth it.
  Can you replace it with 50 lines of your own code? Do that.

### TIER 2: FAILURE RESILIENCE (Passes 7–12)
*"Does this code survive when everything goes wrong?"*

**Pass 7 — Fault Injection**
- Wrap every I/O operation in a trait. Write a mock that fails randomly.
- Test: disk full, permission denied, network timeout, DNS failure,
  connection reset, TLS handshake failure, certificate expired.
- Every external call must have: timeout, retry with backoff, circuit breaker.

**Pass 8 — Concurrency Chaos**
- Run all tests under `--test-threads=1` AND under max parallelism.
- If any Mutex/RwLock exists: write a test with 100 threads hammering it.
- If any async code exists: test cancellation at every await point.
- Run under Miri if feasible: `cargo +nightly miri test`.
- Use `loom` for lock-free structures if applicable.

**Pass 9 — Resource Exhaustion**
- Test with: 0 bytes of available memory (catch alloc failures gracefully).
- Test with: max open file descriptors reached.
- Test with: 0 available disk space for writes.
- Test with: CPU-bound tight loop in a dependency (timeout must fire).
- Test with: 100,000 concurrent requests/operations.

**Pass 10 — Graceful Degradation**
- If the database is down: what happens? Must not panic or corrupt state.
- If the config file is missing: what happens? Secure defaults must apply.
- If the network is unavailable: what happens? Offline mode or clean error.
- If a required service dependency is down: what happens?
- For each scenario: write a test. Add inline recovery logic.

**Pass 11 — Data Integrity Under Failure**
- Simulate crash at every write point (power loss mid-operation).
- Verify: no partial writes corrupt state. Use atomic rename, WAL, or fsync.
- Verify: all persistent state has checksums or Merkle verification.
- Verify: recovery from backup/snapshot works and produces correct state.

**Pass 12 — Chaos Engineering Dry Run**
- Combine faults: network partition + disk full + high CPU simultaneously.
- Run the test suite with random 100ms sleeps injected into I/O paths.
- Run the test suite with random panics injected (catch_unwind boundaries).
- If anything breaks: fix, add regression test, re-run entire Tier 2.

### TIER 3: ADVERSARIAL SECURITY (Passes 13–24)
*"Can a state-level adversary break this?"*

**Pass 13 — Input Validation Hardening**
- Every deserialization point: test with truncated data, extra fields,
  type confusion, integer overflow in length fields, nested depth bombs
  (e.g., 1000-deep JSON nesting), unicode edge cases (RTL overrides,
  zero-width joiners, homoglyph attacks).
- Enforce maximum sizes BEFORE parsing. Reject, don't truncate.

**Pass 14 — Injection Sweep**
- SQL injection on every query (even parameterized — test the parameters).
- Command injection on every shell/exec call (there should be ZERO of these;
  if any exist, replace with direct API calls).
- LDAP injection if applicable.
- Template injection on every rendered output.
- Log injection (newlines, ANSI escape codes in log inputs).
- Header injection on every HTTP response.

**Pass 15 — Authentication Adversarial**
- Timing attack on every comparison: replace `==` with constant-time eq.
- Brute force: verify rate limiting exists and works.
- Credential stuffing: verify account lockout + alerting.
- Session fixation: verify session ID rotates on privilege change.
- Token replay: verify nonces / one-time-use where applicable.
- Test: can an expired token be replayed? Can a revoked token be used?

**Pass 16 — Authorization Adversarial**
- For every endpoint/operation: test as unauthenticated, wrong user,
  expired session, admin vs regular, and with forged role claims.
- IDOR: for every resource access, test with a valid ID belonging to
  another user.
- Privilege escalation: test every admin operation as a regular user.
- Verify: default is deny-all. Permissions are additive, never subtractive.

**Pass 17 — Cryptographic Audit**
- No custom crypto. If custom crypto exists, delete it and use a
  reviewed library (ring, rustcrypto, libsodium).
- Verify: no use of MD5, SHA1 for security purposes.
- Verify: all random values use CSPRNG (OsRng, not thread_rng).
- Verify: all keys are derived with proper KDFs (Argon2id for passwords,
  HKDF for key derivation).
- Verify: no secrets in logs, error messages, URLs, or query strings.
- Verify: secrets are zeroized after use (use `zeroize` crate).
- Verify: constant-time comparison for all secret material.

**Pass 18 — Side Channel Analysis**
- Timing: benchmark critical operations with different inputs. Variance
  should be within noise (use `criterion` benchmarks).
- Cache: if processing secrets, assess cache-timing vulnerability.
  Use constant-time implementations from `subtle` crate.
- Power/EM: document if hardware-level side channels are in scope.
  If running on shared infrastructure: they are.
- Error oracle: do different error types reveal different information?
  Normalize all auth/crypto error responses.

**Pass 19 — Supply Chain Attack Simulation**
- Pick your most-used dependency. Assume it's compromised.
  What's the blast radius? Can you contain it?
- Add `cargo-crev` reviews for critical dependencies.
- Pin all dependency versions exactly (no `^` or `~`).
- Verify: `Cargo.lock` is committed and checked in CI.
- Audit: any build.rs scripts? What do they do? Can they be eliminated?
- Audit: any proc macros? What code do they generate? Review expanded output.

**Pass 20 — Network Security**
- Every outbound connection: verify certificate pinning or TOFU.
- Every TLS configuration: TLS 1.3 only, no fallback.
- Every DNS resolution: assess for DNS poisoning. Use DoH/DoT if feasible.
- Every listening socket: verify bind address is correct (not 0.0.0.0
  when localhost is intended).
- Firewall rules: document required ports. Everything else: deny.

**Pass 21 — Data at Rest**
- Every persistent secret: encrypted with a key not stored alongside it.
- Every database: encrypted at rest. Key management documented.
- Every log file: verify no secrets, PII, or sensitive data is logged.
- Every temporary file: verify it's created with restrictive permissions
  and deleted on scope exit (use `tempfile` crate).

**Pass 22 — Data in Transit**
- No plaintext protocols. Period. No HTTP, no plain SMTP, no FTP.
- Every API call: mutual TLS or authenticated encryption.
- Every internal service-to-service call: authenticated.
- Zero trust networking: assume the local network is hostile.

**Pass 23 — Operational Security**
- Secrets in environment variables: verify they don't leak via /proc,
  ps aux, or crash dumps. Use a secrets manager.
- Logging: verify log levels are appropriate. No DEBUG in production.
- Error pages: verify no stack traces, internal paths, or version numbers
  are exposed to users.
- HTTP headers: verify security headers (HSTS, CSP, X-Frame-Options,
  X-Content-Type-Options, Referrer-Policy).

**Pass 24 — Fuzzing Campaign**
- `cargo fuzz` on every parser, deserializer, and protocol handler.
- Run for minimum 1 hour per target (longer in CI nightly jobs).
- `honggfuzz` as a secondary fuzzer (different mutation strategies).
- Any crash = P0 bug. Fix immediately. Add regression test from the
  crashing input. Re-fuzz.

### TIER 4: UX/UI ADVERSARIAL (Passes 25–30)
*"Would a real human be able to use this without cursing?"*

**Pass 25 — First-Contact Test**
- Delete all local state. Install from scratch. Follow only the README.
- Time it. If setup takes >5 minutes for a developer: simplify.
- If any step requires "just know" tribal knowledge: document it or
  automate it away.

**Pass 26 — Error Experience**
- Trigger every error the user can encounter.
- Each error message must answer: What happened? Why? What do I do now?
- No error codes without human-readable explanations.
- No stack traces in user-facing output.
- No generic "something went wrong" without actionable next steps.

**Pass 27 — Accessibility**
- WCAG 2.1 AA minimum. Test with screen reader (even if just checking
  aria labels are present and correct).
- Keyboard navigation: every interactive element reachable without mouse.
- Color contrast: minimum 4.5:1 for normal text, 3:1 for large text.
- No information conveyed by color alone.
- Responsive: test at 320px, 768px, 1024px, 1440px, 4K.
- Motion: respect `prefers-reduced-motion`.

**Pass 28 — Performance UX**
- Every user action: response within 100ms or show a loading indicator.
- Every network request: show progress, handle timeout gracefully.
- Skeleton screens for async content. Never a blank white page.
- Measure Largest Contentful Paint, First Input Delay, CLS.
- Test on throttled connection (Chrome DevTools 3G preset).

**Pass 29 — Adversarial User**
- Paste 10MB into every text field. What happens?
- Click submit 100 times rapidly. What happens?
- Open 50 tabs. What happens?
- Navigate away mid-operation. What happens?
- Use browser back/forward through form flows. What happens?
- Disable JavaScript. What happens? (At minimum: a useful error.)
- For every scenario: write an automated test or document as UX-DEBT.

**Pass 30 — Design Consistency**
- Every component uses the design system. No one-off styles.
- Typography scale is consistent. No magic numbers in font sizes.
- Spacing uses a consistent scale (4px/8px grid).
- Icons are consistent in style and weight.
- Empty states are designed, not forgotten.
- Loading states are designed, not forgotten.
- Error states are designed, not forgotten.

### TIER 5: INTEGRATION & ECOSYSTEM (Passes 31–33)
*"Does this code play well with its siblings?"*

**Pass 31 — Cross-Repo Integration**
- Identify every sibling repo this code interacts with:
  `plausiden-engine`, `plausiden-inject`, `plausiden-swarm`,
  `plausiden-browser-ext`, `plausiden-desktop`, `plausiden-android`,
  `plausiden-usb`, `plausiden-vault`, `plausiden-auth`, Sacred.Vote.
- For each integration point: write an integration test that runs BOTH
  sides. Test with version skew (current + previous version).
- Any shared types/protocols: verify they are defined in a shared crate
  and both sides use the same version.

**Pass 32 — Contribute Back**
- Any bug found in a sibling repo during integration: fix it IN that repo.
- Any hardening applied here that applies to a sibling: port it.
- Any test pattern developed here that applies to a sibling: port it.
- Any FOSS tool absorbed here that a sibling needs: share it via shared crate.
- Apply AVP Tier 1–3 (minimum 6 passes) to every contributed change.

**Pass 33 — Ecosystem Integrity**
- Run the full test suite of every sibling repo that depends on this one.
- If any sibling test breaks: this is YOUR regression. Fix it here.
- Verify all shared dependencies are at consistent versions across repos.
- Verify no circular dependencies have been introduced.

### TIER 6: META-VALIDATION (Passes 34–36)
*"Are we even testing the right things?"*

**Pass 34 — Mutation Testing**
```bash
cargo mutants --jobs $(nproc) --timeout 300
```
- Every surviving mutant = a test gap. Write a test that kills it.
- Target: <5% mutant survival rate. If higher: loop Tier 1–2 again.

**Pass 35 — Property-Based Testing**
- For every pure function: write a `proptest` or `quickcheck` property.
- Properties to test:
  - Idempotency: `f(f(x)) == f(x)` where applicable
  - Round-trip: `decode(encode(x)) == x`
  - Monotonicity: if `a < b` then `f(a) <= f(b)` where applicable
  - Commutativity/associativity where applicable
  - "No panic" property: `∀ x: f(x) does not panic`
- Run with at least 10,000 cases per property.

**Pass 36 — Formal Verification (where feasible)**
- For critical algorithms (crypto, consensus, state machines):
  use `kani` (Rust model checker) or write TLA+ specs.
- For unsafe blocks: use Miri (`cargo +nightly miri test`).
- For numeric code: verify absence of overflow with `cargo careful`.
- Document what IS and IS NOT formally verified. The boundary matters.

---

## 4. FOSS ABSORPTION DETAILED WORKFLOW

When you need a capability and a FOSS tool exists:

```
DISCOVER
├── Search crates.io, GitHub, GitLab, Codeberg
├── Prefer: Rust-native, minimal dependencies, active maintenance
├── Check: license compatibility (MIT/Apache-2.0/BSL-1.1 compatible)
└── Check: CVE history, open security issues, unsafe usage

EVALUATE (before writing a single line)
├── `cargo geiger` on the crate — how much unsafe?
├── `cargo tree -p <crate>` — how deep is the dependency tree?
├── Read the source. All of it if <5000 lines. Key modules if larger.
├── Run THEIR test suite. How comprehensive is it?
├── Check: does it handle errors or panic/unwrap?
├── Check: does it validate inputs or trust the caller?
└── Verdict: absorb, adapt, or write from scratch?

ABSORB
├── Vendor the crate (copy source into your repo, not just Cargo dep)
│   OR fork on your Git host with a clear "hardened fork" label
├── Strip features you don't use (attack surface reduction)
├── Replace all unwrap/expect with proper error handling
├── Replace all unsafe blocks with safe alternatives where possible
│   Where not possible: add // SAFETY: comments with proof
├── Add bounds checking on all array/slice access
├── Add input validation on all public API surfaces
├── Add comprehensive logging/tracing
├── Add the tests the original should have had
└── Run AVP Tier 1–3 on the absorbed code (minimum 12 passes)

INTEGRATE
├── Wrap in a thin adapter that matches YOUR project's error types,
│   logging, and configuration patterns
├── The adapter is the only code that touches the absorbed library
│   (dependency inversion — you can swap the library later)
├── Write integration tests between the adapter and your code
└── Run full project test suite

MAINTAIN
├── Track upstream releases. Diff each release against your fork.
├── Cherry-pick security fixes. Ignore feature churn.
├── Re-run AVP Tier 3 (security passes) on each upstream merge.
└── If upstream dies: you own it now. You already hardened it. Continue.
```

---

## 5. SUPERSOCIETY TOOLCHAIN

These are the FOSS tools to absorb, harden, and integrate into CI:

### Build & Verify
| Tool | Purpose | Integration |
|------|---------|-------------|
| `cargo clippy` | Lint (deny all warnings) | CI gate |
| `cargo fmt --check` | Format enforcement | CI gate |
| `cargo audit` | Known CVE check | CI gate |
| `cargo deny` | License + advisory + ban check | CI gate |
| `cargo vet` | Supply chain review | CI gate |
| `cargo geiger` | Unsafe usage report | CI report |
| `cargo careful` | Extra UB checks | CI nightly |
| `cargo +nightly miri test` | Undefined behavior detection | CI nightly |
| `kani` | Model checking / formal verification | CI nightly |
| `cargo mutants` | Mutation testing | CI nightly |
| `cargo fuzz` / `honggfuzz` | Continuous fuzzing | CI nightly + dedicated |
| `cargo tarpaulin` | Code coverage | CI report (target >90%) |
| `semgrep` | SAST with custom rules | CI gate |

### Runtime Defense
| Tool | Purpose | Integration |
|------|---------|-------------|
| `seccomp-bpf` | Syscall filtering | Process init |
| `landlock` | Filesystem sandboxing | Process init |
| `capsicum` / capability mode | Capability-based security | Where supported |
| `rlimit` | Resource limits | Process init |
| `tracing` + `tracing-subscriber` | Structured audit logging | All code |
| `sentry` / equivalent | Error tracking with PII scrubbing | Production |

### Crypto & Secrets
| Tool | Purpose | Integration |
|------|---------|-------------|
| `ring` / `rustcrypto` | Reviewed crypto primitives | All crypto operations |
| `subtle` | Constant-time operations | All secret comparisons |
| `zeroize` | Secret memory clearing | All secret-holding types |
| `argon2` | Password hashing | plausiden-auth |
| `sequoia-pgp` | PGP operations | plausiden-vault |
| `age` | File encryption | plausiden-vault |
| `webauthn-rs` | WebAuthn/FIDO2 | plausiden-auth |

---

## 6. CROSS-REPO CONTRIBUTION PROTOCOL

```
WHEN WORKING IN ANY REPO:

1. Before starting: pull latest from ALL sibling repos.
2. Check shared dependency versions. If mismatched: align first.
3. While working: if you find a bug/improvement applicable to a sibling:
   a. Switch to that repo
   b. Apply the fix
   c. Run AVP Tier 1–3 (6 passes minimum) ON THAT FIX
   d. Commit with message: "AVP-CROSSFIX from <source-repo>: <description>"
   e. Switch back and continue
4. After completing work: run integration tests spanning all affected repos.
5. Treat every sibling repo's code with the same suspicion as third-party
   code. Your past self is an untrusted contributor.

SIBLING REPOS (current ecosystem):
├── plausiden-engine    — LFI core (HDC/PSL/VSA)
├── plausiden-inject    — Data injection + Hurd/seL4 translators
├── plausiden-swarm     — Distributed coordination
├── plausiden-browser-ext — Browser extension
├── plausiden-desktop   — Desktop application
├── plausiden-android   — Android app (Kotlin shim → Rust daemon)
├── plausiden-usb       — USB security module
├── plausiden-vault     — Secret management (Vaultwarden, KDBX4, PGP, age)
├── plausiden-auth      — Authentication (Argon2id, TOTP, WebAuthn)
└── sacred-vote         — Cryptographic polling platform (TS/React + Rust)
```

---

## 7. INLINE ANNOTATION STANDARD

Every annotation is machine-grepable for CI and audit:

```rust
// BUG ASSUMPTION: <what could go wrong here>
// AVP-PASS-N: <date> <finding and resolution>
// SAFETY: <proof that this unsafe block is sound>
// SECURITY: <threat mitigated and how>
// UX-DEBT: <manual verification required, risk if skipped>
// REGRESSION-GUARD: <why this fix exists, what broke before>
// FOSS-ABSORBED: <crate name> <version> <reason for vendoring>
// SUPERSOCIETY: <defense-in-depth measure beyond standard practice>
// DEBUG-REMOVE: <line to be stripped before release>
// SHIP-DECISION: <date> <accepted residual risks> <developer>
// CROSSFIX: <source-repo> <description of ported fix>
```

---

## 8. CI PIPELINE (FULL)

```yaml
name: AVP-2 Enforcement
on: [push, pull_request]

jobs:
  # ── GATE (must pass to merge) ──────────────────────────
  gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: No DEBUG-REMOVE on protected branches
        run: |
          if grep -rn "DEBUG-REMOVE" src/ --include="*.rs"; then
            echo "::error::DEBUG-REMOVE markers found"
            exit 1
          fi

      - name: Format check
        run: cargo fmt --check

      - name: Clippy (deny everything)
        run: cargo clippy --all-targets --all-features -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery

      - name: Dependency audit
        run: cargo audit

      - name: Dependency deny
        run: cargo deny check

      - name: Full test suite
        run: cargo test --all-features --all-targets

      - name: Test density check
        run: |
          TESTS=$(grep -rc "#\[test\]\|#\[tokio::test\]" src/ tests/ --include="*.rs" | awk -F: '{s+=$2}END{print s+0}')
          FUNCS=$(grep -rc "pub fn\|pub async fn" src/ --include="*.rs" | awk -F: '{s+=$2}END{print s+0}')
          echo "Tests: $TESTS | Public functions: $FUNCS"
          RATIO=$(echo "scale=1; $TESTS / ($FUNCS + 1)" | bc)
          echo "Ratio: $RATIO (minimum: 4.0)"
          if [ $(echo "$RATIO < 4" | bc) -eq 1 ]; then
            echo "::error::Test-to-function ratio below 4.0"
            exit 1
          fi

      - name: BUG ASSUMPTION coverage
        run: |
          FUNCS=$(grep -c "pub fn\|pub async fn" src/**/*.rs 2>/dev/null || echo 0)
          BUGS=$(grep -c "BUG ASSUMPTION" src/**/*.rs 2>/dev/null || echo 0)
          echo "Public functions: $FUNCS | BUG ASSUMPTION: $BUGS"
          if [ "$BUGS" -lt "$FUNCS" ]; then
            echo "::error::Not all public functions have BUG ASSUMPTION annotations"
            exit 1
          fi

      - name: No unwrap in src (only tests)
        run: |
          UNWRAPS=$(grep -rn "\.unwrap()" src/ --include="*.rs" | grep -v "// SAFETY:" | grep -v "#\[cfg(test)\]" || true)
          if [ -n "$UNWRAPS" ]; then
            echo "$UNWRAPS"
            echo "::error::unwrap() in src/ without SAFETY justification"
            exit 1
          fi

      - name: Coverage (minimum 85%)
        run: |
          cargo tarpaulin --all-features --out xml
          COVERAGE=$(cargo tarpaulin --all-features 2>&1 | grep -oP '\d+\.\d+%' | tail -1 | tr -d '%')
          echo "Coverage: $COVERAGE%"
          if [ $(echo "$COVERAGE < 85" | bc) -eq 1 ]; then
            echo "::error::Coverage below 85%"
            exit 1
          fi

  # ── NIGHTLY (deep analysis) ────────────────────────────
  nightly:
    runs-on: ubuntu-latest
    if: github.event_name == 'schedule'  # cron: '0 3 * * *'
    steps:
      - uses: actions/checkout@v4

      - name: Miri (undefined behavior)
        run: cargo +nightly miri test

      - name: Cargo careful
        run: cargo +nightly careful test

      - name: Mutation testing
        run: |
          cargo mutants --jobs $(nproc) --timeout 300 2>&1 | tee mutants.log
          SURVIVED=$(grep -c "SURVIVED" mutants.log || echo 0)
          TOTAL=$(grep -c "mutant" mutants.log || echo 1)
          RATE=$(echo "scale=1; $SURVIVED * 100 / $TOTAL" | bc)
          echo "Mutant survival rate: $RATE% (target: <5%)"
          if [ $(echo "$RATE > 5" | bc) -eq 1 ]; then
            echo "::error::Mutant survival rate above 5%"
            exit 1
          fi

      - name: Fuzz corpus run (30 min per target)
        run: |
          for target in $(cargo fuzz list 2>/dev/null); do
            cargo fuzz run "$target" -- -max_total_time=1800
          done

      - name: Unsafe report
        run: cargo geiger --output-format=json > geiger-report.json

      - name: SAST (semgrep)
        run: semgrep --config=auto --error src/

  # ── CROSS-REPO (integration) ───────────────────────────
  cross-repo:
    runs-on: ubuntu-latest
    if: github.event_name == 'push' && github.ref == 'refs/heads/main'
    steps:
      - uses: actions/checkout@v4
        with:
          path: current

      # Clone and test each sibling that depends on this repo
      - name: Integration sweep
        run: |
          for repo in plausiden-engine plausiden-vault plausiden-auth; do
            if git clone "https://github.com/redcaptian1917/$repo" 2>/dev/null; then
              cd "$repo"
              cargo test --all-features || echo "::warning::$repo integration tests failed"
              cd ..
            fi
          done
```

---

## 9. LOOP INTERRUPTION (SHIPPING)

After a MINIMUM of 36 passes, the developer MAY interrupt with:

```rust
// SHIP-DECISION: 2026-04-06 after 42 AVP passes
// Accepted residual risks:
//   - Fuzzing coverage at 78% of targets (3 parsers remain)
//   - Kani verification pending on consensus module
//   - UX-DEBT: mobile breakpoint testing manual-only
//   - Dependency libfoo v2.3.1 has advisory RUSTSEC-2026-XXXX,
//     mitigated by input validation layer (Pass 13)
// Mutant survival rate: 3.2%
// Coverage: 91.4%
// Developer: Paul
// VERDICT: STILL BROKEN. Shipping with documented residual risk.
```

**The verdict is always STILL BROKEN.** Shipping is risk acceptance, not a declaration of correctness. The loop resumes on the next commit.

---

## 10. PHILOSOPHY

This is not paranoia. This is engineering for a world where:
- The adversary has more resources than you
- The adversary has more time than you
- The adversary is already inside
- Your tools are compromised
- Your assumptions are wrong
- Your tests are lying
- Your code is guilty until proven innocent, and innocence is provisional

The supersociety stack exists because no single tool is sufficient.
The recursive loop exists because no single pass is sufficient.
The FOSS absorption protocol exists because no dependency is trustworthy.
The cross-repo contribution protocol exists because your own code isn't either.

**You are not building software. You are building a fortress out of materials
you found in enemy territory. Inspect every brick. Test every beam.
And when you're done: inspect again.**
