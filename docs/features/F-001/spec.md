---
id: F-001
title: Project Foundation
type: chore          # feature | refactor | bugfix | chore
system: cipherglass
status: implemented
area: foundation
owners:
  - three_seat
created_at: 2026-10-02
contract: docs/features/F-001/contract.yaml
requirements:        # system-overview REQ IDs this feature implements (see Traceability)
  - REQ-002
  - REQ-041
  - REQ-043
  - REQ-111
  - REQ-132
adrs:
  - ADR-0001
---

# Summary

Set up the Rust workspace that every later v0 feature builds on: a set of
crates whose dependency graph enforces the parser adapter boundary, a
`cipherglass` binary that starts and reports its version, workspace-wide lint
and safety settings, a test harness with a fixture directory, and a local check script. F-001
adds no reconnaissance behavior. Its job is to make the architectural rules in
`docs/system-overview.md` hard to break by accident before any parser or
analyzer code exists.

# Context

- **Problem.** The system overview's central constraint is parser
  independence (REQ-041, REQ-043, REQ-111): analyzers must never see
  third-party AST types. If all code lives in one crate, that rule depends on
  review discipline alone. Splitting the code into crates lets the compiler
  and an automated test enforce it.
- **V0 feature.** #1, Project Foundation. Features #2 to #5 (Artifact
  Discovery, JavaScript Parsing and Evidence Extraction, GraphQL Analysis,
  Findings and Export) fill in the crates created here.
- **Pipeline stages.** None are implemented. F-001 creates the crates each
  stage will live in.
- **Constraints.**
  - Rust, edition 2024, built with stable `cargo`.
  - Apache-2.0, matching `LICENSE`.
  - Linux is the development platform. Nothing in F-001 is platform-specific.
- **Governing documents.** `docs/system-overview.md` v0.3.0 (baselined).
  F-001 produces ADR-0001 to record the crate layout and why.

## Traceability

| REQ | Role in F-001 |
|-----|---------------|
| REQ-002 | **Enforces.** Parsing (`cipherglass-js`) and analysis (`cipherglass-graphql`) are separate crates with no dependency between them. |
| REQ-041, REQ-111 | **Enforces.** Approved dependency lists confine third-party parser crates to the adapter crate. |
| REQ-043 | **Enforces.** `cipherglass-graphql` cannot name a type from a crate it cannot depend on. |
| REQ-132 | **Enforces** from the first commit, through the architecture test. |
| REQ-044 | **Partial.** `cipherglass-core` (where findings live) cannot depend on a parser. Re-exports from the facade are enforced by review only (see Known limitations). |
| REQ-055, REQ-061, REQ-134 | **Prepares for.** The analyzer crate exists with its boundary in place. The behavior arrives in F-004. |
| REQ-022, REQ-023 | **Prepares for.** `unsafe_code = "forbid"`; JS engines and runtimes are banned from approved lists; fixtures are inert. |
| REQ-136, REQ-137 | **Prepares for.** Test harness and fixture directory only. Per-stage and end-to-end tests arrive with F-002 to F-005. |
| REQ-140 to REQ-142 | **Prepares for.** Any networking crate needs an approved-list change, which needs contract review. |

# Behavior

## User flow

- **Command / library entrypoint:** `cipherglass` binary; `cipherglass` library crate.
- **Inputs:** command-line arguments only.
- **Outputs:** version string, help text, usage errors.
- **Error cases:** no arguments; unknown flag or subcommand; stdout write failure.

## Detailed behavior

### Workspace layout

```
Cargo.toml                 virtual workspace manifest
Cargo.lock                 committed (the workspace ships a binary)
deny.toml                  cargo-deny policy
rustfmt.toml
crates/
  cipherglass-core/        Cipherglass-owned types: artifact, location, evidence, finding
  cipherglass-js/          JavaScript parser adapter (the only home for a JS parser crate)
  cipherglass-graphql/     GraphQL analyzer
  cipherglass/             library facade: discovery, pipeline wiring, exporters
    tests/architecture.rs  dependency boundary test
  cipherglass-cli/         `cipherglass` binary
    tests/cli.rs           CLI skeleton test
tests/fixtures/README.md   fixture policy and layout
docs/adrs/ADR-0001-crate-layout.md
scripts/check.sh           runs every check locally
```

- `[workspace.package]` sets `version = "0.1.0"`, `edition = "2024"`,
  `license = "Apache-2.0"`, `repository`, and `publish = false`. Every crate
  inherits these.
- In F-001 every library crate is an empty skeleton: a crate-level doc
  comment stating its responsibility and its boundary rule, and nothing else.

### Allowed dependency directions

The rules apply to every dependency kind: normal, dev and build.

| Crate                 | May depend on (workspace)                          | Must not depend on                   |
|-----------------------|----------------------------------------------------|--------------------------------------|
| `cipherglass-core`    | nothing                                            | any other workspace crate            |
| `cipherglass-js`      | `cipherglass-core`                                 | `cipherglass-graphql`, `cipherglass`, `cipherglass-cli` |
| `cipherglass-graphql` | `cipherglass-core`                                 | `cipherglass-js`, `cipherglass`, `cipherglass-cli` |
| `cipherglass`         | `cipherglass-core`, `cipherglass-js`, `cipherglass-graphql` | `cipherglass-cli`           |
| `cipherglass-cli`     | `cipherglass`                                      | `cipherglass-core`, `cipherglass-js`, `cipherglass-graphql` |

`cipherglass-cli` goes through the facade only, so the facade remains the
single public surface.

### Approved third-party dependencies (F-001)

| Crate | Kind | Dependency | Purpose |
|-------|------|------------|---------|
| `cipherglass-cli` | normal | `clap` `=4.6.7` (feature `derive`; exact pin) | Argument parsing |
| `cipherglass` | dev | `serde_json` `=1.0.151` (exact pin) | Parsing `cargo metadata` output in the architecture test |

Later features add entries through their own contracts. A JavaScript engine
or runtime (anything that can execute JavaScript) may never be approved
(REQ-023).

### Boundary enforcement

- `crates/cipherglass/tests/architecture.rs` runs
  `$CARGO metadata --no-deps --format-version 1 --offline` (where `$CARGO` is
  the environment variable cargo sets for tests), then reads each workspace
  package's declared dependencies, of every kind.
- The checking logic is a pure function. It takes the declared dependencies
  per crate, the workspace dependency table, and the approved lists, and
  returns a list of violations. The test asserts the real workspace has none.
- The same function is unit-tested against synthetic inputs, so the checker
  is shown to detect each kind of violation:
  - a forbidden workspace edge (e.g. `cipherglass-graphql` → `cipherglass-js`);
  - an unapproved third-party dependency;
  - a forbidden edge that appears only as a dev-dependency.
- The workspace table and approved lists live in the test file. A crate
  missing from the table is itself a violation, so new crates must be
  classified.

### Known limitations

- **Re-exports (REQ-044).** Neither cargo nor the architecture test can stop
  `cipherglass` from writing `pub use cipherglass_js::SomeParserType`. Stable
  Rust has no public/private dependency distinction. Until a mechanical
  check exists, this is a review checklist item. F-003 must revisit it when
  `cipherglass-js` first gains public items.

### CLI skeleton

- `cipherglass --version` prints `cipherglass 0.1.0` (from
  `CARGO_PKG_VERSION`) to stdout and exits 0.
- `cipherglass --help` prints usage to stdout and exits 0. The first line
  names the tool and its one-line purpose from `README.md`.
- `cipherglass` with no arguments prints usage to stderr and exits 1.
- An unknown flag or subcommand prints an error and usage to stderr and exits 1.
- If writing to stdout fails (e.g. closed pipe), the binary exits 2 without
  panicking. Every write goes through a handled `io::Result`; `print!` and
  `println!` (which panic on write failure) are denied by clippy.
- F-001 defines no subcommands. They arrive with F-002 to F-005.
- The binary target sets `doc = false` to avoid a `cargo doc` output
  collision with the `cipherglass` library.

### Workspace settings

- `[workspace.lints.rust] unsafe_code = "forbid"`, inherited by every crate
  through `[lints] workspace = true`. Cipherglass parses untrusted input
  (REQ-022), so memory safety is non-negotiable in our own code.
- Clippy runs with `-D warnings` in `scripts/check.sh`.
- `[workspace.lints.clippy]` sets `print_stdout = "deny"` and
  `print_stderr = "deny"`. `print!`/`println!` panic when stdout is closed, so
  all output goes through a handled `io::Result` (exit 2 on failure).
- `rustfmt.toml` sets `edition = "2024"` and nothing else.

### Test harness and fixture policy

- `tests/fixtures/README.md` states:
  - Fixtures are synthetic, or public and license-compatible with
    Apache-2.0. **No client-derived material, ever.** No real credentials,
    even revoked ones.
  - Fixtures are inert data. No test executes fixture content (REQ-023).
  - Layout: `tests/fixtures/<area>/<case>/`, where `<area>` matches a spec
    `area` value. Each case may have an `expected/` subdirectory.
- `crates/cipherglass-cli/tests/cli.rs` runs the built binary through
  `env!("CARGO_BIN_EXE_cipherglass")` and checks every CLI skeleton behavior
  above. It needs no extra test dependency.

### Local checks (no CI yet)

Nothing runs automatically on GitHub for now. There is no `.github/`
directory. `scripts/check.sh` runs every check locally and stops at the first
failure:

1. `cargo fmt --all --check`
2. `cargo build --workspace --locked`
3. `cargo clippy --workspace --all-targets --locked -- -D warnings`
4. `cargo test --workspace --locked`
5. `cargo deny --locked check`, against `deny.toml`: RustSec advisories and
   yanked crates are denied; licenses are limited to Apache-2.0, MIT and
   Unicode-3.0; crates.io is the only source; known JavaScript engines
   (`boa_engine`, `v8`, `deno_core`, `rquickjs`, `quickjs-rs`, `mozjs`) are
   banned anywhere in the dependency tree (REQ-023).
6. `cargo audit`. This overlaps cargo-deny's advisory check, and the overlap
   is accepted as a second opinion.

- The script names the exact tool versions to install: `cargo-deny` 0.19.8
  and `cargo-audit` 0.22.2 (`cargo install --locked`).
- Steps 5 and 6 fetch the RustSec advisory database over the network. That
  is developer tooling, not Cipherglass runtime behavior, so REQ-140 to
  REQ-142 do not apply.
- New advisories appear without any code change, so run the script before
  every commit, not only when dependencies change.
- Direct third-party dependencies use exact (`=`) version pins. Upgrades are
  deliberate, reviewed changes.
- Adding GitHub Actions CI later is a separate, explicit decision.

### ADR-0001

`docs/adrs/ADR-0001-crate-layout.md` records:

- the crate layout and the dependency table above;
- why the split exists (REQ-041, REQ-043, REQ-111);
- how it is enforced, and the re-export limitation;
- the determinism convention for later features: output-affecting
  collections are ordered (`BTreeMap`/`BTreeSet`, or a sorted `Vec`), never
  `HashMap` iteration order (REQ-004, REQ-005).

# Non-Functional Requirements

- **Determinism (REQ-004, REQ-005):** F-001 produces no findings. The
  convention is recorded in ADR-0001.
- **Untrusted input (REQ-022, REQ-023):** `unsafe_code = "forbid"`
  workspace-wide; no JS engine or runtime is ever approved; fixtures are
  inert.
- **Fault tolerance (REQ-021):** not applicable. There is no pipeline yet.
- **Performance:** not applicable.
- **Portability:** builds on stable Rust. No platform-specific code.
- **UX:** help text names the tool and its purpose in one line.

# Acceptance Criteria

- [x] AC-1 `cargo build --workspace --locked` succeeds from a clean checkout.
- [x] AC-2 `cargo test --workspace --locked` passes.
- [x] AC-3 `cargo fmt --all --check` and `cargo clippy --workspace --all-targets --locked -- -D warnings` pass.
- [x] AC-4 `cipherglass --version` prints `cipherglass 0.1.0` to stdout and exits 0.
- [x] AC-5 `cipherglass --help` prints usage to stdout and exits 0.
- [x] AC-6 `cipherglass` with no arguments, and with an unknown flag, exits 1 and writes usage to stderr.
- [x] AC-7 The architecture test passes on the real workspace, and its unit tests show it detects a forbidden workspace edge, an unapproved third-party dependency, a forbidden dev-dependency edge, and an unclassified crate.
- [x] AC-8 Every crate has `[lints] workspace = true`, and the workspace sets `unsafe_code = "forbid"` and denies clippy's `print_stdout` and `print_stderr`.
- [x] AC-9 `docs/adrs/ADR-0001-crate-layout.md` exists and covers the four points in the ADR-0001 section.
- [x] AC-10 `scripts/check.sh` runs the checks in AC-1 to AC-3 and AC-14, and exits non-zero on the first failure. No `.github/` directory exists.
- [x] AC-11 `tests/fixtures/README.md` states the fixture policy.
- [x] AC-12 Every crate inherits `publish = false` (checked by the architecture test through the `publish` field of `cargo metadata`).
- [x] AC-13 `cipherglass --help` with stdout closed exits 2 without panicking.
- [x] AC-14 `cargo deny --locked check` and `cargo audit` pass.

# Implementation Notes

- **Files to create:** everything listed in the workspace layout above.
  Update `README.md` with build and test commands.
- **Refactors:** none. The repository has no code yet.
- **Dependencies:** exactly the approved list above. `clap` returns exit code
  2 for usage errors by default, so the CLI must map those to 1.
- **Owned types:** none added yet. Evidence and finding types come with F-003
  and F-004/F-005 (REQ-114).
- **Fixtures:** `tests/fixtures/README.md` only.

# Open Questions

- Q1 (closed): Five crates, or fewer? — Five. Merging `cipherglass-graphql`
  into the facade would lose the compiler-enforced boundary.
- Q2 (closed): Declare an MSRV (`rust-version`) now? — No. Revisit when an
  MSRV check exists; an untested MSRV is a false promise.
- Q3 (closed): Where does the architecture test live? — In
  `crates/cipherglass/tests/`. No `xtask` until there is a second task.
