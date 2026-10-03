# F-001 Gap Analysis

## Round 1: spec draft

Compared `spec.md` (first draft) against `docs/system-overview.md` v0.3.0,
`docs/templates/feature.spec.md`, and whether the design actually works.

Severity: **H** = would let a system-overview requirement be broken or makes
an AC impossible to verify; **M** = ambiguity an implementer would have to
guess at; **L** = polish.

| #  | Sev | Gap | Resolution in spec |
|----|-----|-----|--------------------|
| 1  | H | **Traceability overclaims.** The frontmatter says F-001 *implements* REQ-055, REQ-061, REQ-134 (analyzer behavior) and REQ-136, REQ-137 (per-stage unit tests and end-to-end tests). F-001 has no stages and no pipeline, so it cannot satisfy them. | Frontmatter now lists only requirements F-001 structurally enforces. A traceability table in the body separates *enforces* from *prepares for*. |
| 2  | H | **Dependency kinds unspecified.** The rule doesn't say whether dev- and build-dependencies count. A `cipherglass-graphql` dev-dependency on `cipherglass-js` (convenient for test setup) would quietly mix the stages in tests. | The rules apply to every kind: normal, dev and build. |
| 3  | H | **AC-7 is a manual check.** "Verified by temporarily adding the dependency" is not repeatable, and the checker itself is never tested. | The check is a pure function over parsed metadata, unit-tested with synthetic violating inputs. AC-7 and AC-8 are now automated. |
| 4  | H | **REQ-044 re-export rule cannot be enforced.** Approved dependency lists do not catch `pub use cipherglass_js::…` in the facade, and stable Rust has no public/private dependency distinction. | Recorded as a known limitation, enforced by review for now. Revisit in F-003, when `cipherglass-js` first has public items. |
| 5  | M | **How the architecture test runs is unspecified.** Calling `cargo metadata` from inside `cargo test` needs the right `cargo` binary, and resolving dependencies could trigger network access. | Use the `CARGO` environment variable with `metadata --no-deps --format-version 1 --offline`. Checked: `--no-deps` still reports each crate's declared dependencies with their `kind`. |
| 6  | M | **Fixture policy missing.** Cipherglass is a recon tool built by a consultancy. The obvious way to get realistic fixtures is copying client JavaScript, which would leak confidential client material into a public repository. | Fixtures must be synthetic, or public and license-compatible. No client-derived material is allowed. Fixtures are inert data and are never executed (REQ-023). |
| 7  | M | **Accidental publishing.** Nothing stops `cargo publish` from pushing skeleton crates to crates.io. | `publish = false` in `[workspace.package]`. |
| 8  | M | **CI supply chain.** The spec says nothing about action pinning or token permissions, for a tool whose subject is security. | `permissions: contents: read`; third-party actions pinned by commit SHA; the toolchain installed with `rustup`, not a third-party action. |
| 9  | M | **`cargo doc` output collision.** A bin named `cipherglass` and a lib named `cipherglass` both write to `target/doc/cipherglass/` (a known cargo collision warning). | `doc = false` on the binary target. |
| 10 | M | **Exit code 2 is never defined for F-001.** Writing to a closed stdout (`cipherglass --help \| head -0`) panics in `println!`, which exits 101. | A stdout write failure exits 2 with no panic. |
| 11 | L | **Initial version unspecified.** `--version` output is asserted, but the version is not. The system overview's `0.3.0` is the document version, not the crate version. | Crate version starts at `0.1.0`. |
| 12 | L | **Q3 left open** when one answer is clearly simpler. | Resolved: the architecture test lives in `crates/cipherglass/tests/`. No `xtask` until there is a second task. |
| 13 | L | **No-network rule (REQ-140 to REQ-142) is not mentioned.** | Covered indirectly: any HTTP crate needs an approved-list change, which then needs contract review. Noted in traceability. |

### Considered, not adopted

- **Banning `HashMap` with clippy's `disallowed-types` for determinism.**
  Too blunt: a `HashMap` used only for lookups is harmless. Determinism stays
  an ADR-0001 convention, and later features check it with repeated-run tests
  (REQ-004a, REQ-005a).
- **Running `cargo-deny` / `cargo-audit` in CI.** Both are installed locally
  and would be worthwhile. But the only third-party dependency is `clap`, so
  this is deferred to the feature that adds the first parser crate.
  *Superseded during implementation (2026-10-02): adopted along with exact
  version pins. See "Implementation changes" below.*

### Remaining open questions (need a decision from you)

- **Q1:** five crates (proposed) or fewer.
- **Q2:** MSRV now or later (proposed: later).

## Round 2: contract against spec

Compared `contract.yaml` against the revised `spec.md` and
`docs/system-overview.md`, and probed the two test mechanisms that were not
obviously workable.

| #  | Sev | Gap | Resolution |
|----|-----|-----|------------|
| 1  | H | **The contract contradicts itself.** `ai_instructions` says "Do not modify … ADRs", but `writes_paths` creates ADR-0001. An implementer following the instructions literally would skip AC-9. | Instruction now reads: do not modify *existing* ADRs; creating ADR-0001 is in scope. |
| 2  | H | **T-9 (stdout closed → exit 2) would have been flaky.** The natural test (`Stdio::piped()`, then drop the child's stdout after spawning) races: the help text fits in the pipe buffer, so the write usually succeeds and the binary exits 0. | Probed: create the pipe with `std::io::pipe()` (stable since 1.87) and drop the read end *before* spawning. The write then fails reliably, giving exit 2. The mechanism is now in T-9. It also requires the binary to write through `writeln!` on a handled `Result`, never `println!`. |
| 3  | M | **The `publish = false` check would have been wrong.** `cargo metadata` reports it as `"publish": []`, not `false`. A check for `false` or `null` would pass or fail for the wrong reason. | Probed and confirmed. T-1 now says "`publish` is an empty array." |
| 4  | M | **AC-1 (`cargo build`) had no direct check.** It was mapped to "implied by clippy/test". | Added an explicit `cargo build --workspace --locked` CI step, and mapped AC-1 to it. |
| 5  | M | **Stdout-failure behavior had no acceptance criterion in the spec.** It appeared only in the CLI section, so the spec's AC list was weaker than the contract's tests. | Added spec AC-13. |
| 6  | M | **Dependency versions were unpinned.** The contract approved `clap` and `serde_json` without a major version, so a future major bump would pass without review. | `clap` `4`, `serde_json` `1`. |
| 7  | L | **Wrong crate name in `prohibited_always`.** `rusty_v8` is published as `v8`. | Corrected. |
| 8  | L | **ERR-1 only partly tested.** T-10 covers malformed JSON. An unset `CARGO` and a non-zero `cargo metadata` exit are not tested. | Accepted. Both fail loudly through `expect`/assert in the test harness, so neither can pass vacuously. Noted in the contract. |
| 9  | L | **LL-6 (`doc = false`) and the per-crate `[lints] workspace = true` have no automated check**, and `cargo metadata` does not expose lints. | Accepted as review items in the AC map. |
| 10 | L | **Two copies of the dependency table:** the contract's `architecture.crates` and the constants in `architecture.rs`. | Accepted. The test file is the enforcing copy, and the review checklist gains "contract and architecture.rs tables match". |

### Checked, no gap

- `requirements.system` matches the spec's frontmatter.
- Every spec AC (1 to 13) maps to a test, a CI step, or a named review item.
- Every `writes_paths` entry has a reason, and none overlaps `must_not_modify`.
- The contract's allowlist-only table is equivalent to the spec's
  allow/forbid table, since anything not allowed is forbidden.
- Nothing in F-001 makes a network call. CI's `cargo` resolution is outside
  Cipherglass's runtime and is not covered by REQ-140 to REQ-142.

### Decisions (2026-10-02)

- **Q1:** five crates, as proposed.
- **Q2:** no MSRV yet.

Nothing blocks moving the contract from `draft` to `in_progress`.

## Implementation changes (2026-10-02)

Decided during implementation, after the analyses above:

- **Exact pins** for direct dependencies: `clap =4.6.7`, `serde_json =1.0.151`.
  clap pins its own `clap_builder`/`clap_derive` exactly. Every other
  transitive crate is held by `Cargo.lock` (version and checksum) together
  with `--locked`.
- **Supply-chain checks** with `cargo-deny` (`deny.toml`) and `cargo-audit`. `deny.toml` bans known JavaScript engines
  anywhere in the tree, which turns INV-6 from a review item into an
  automated check. The ban mechanism was checked by temporarily banning
  `strsim` (failed as expected, then reverted).
- **No GitHub automation yet.** The GitHub Actions workflow was removed by
  decision. All checks run locally through `scripts/check.sh`. This replaces
  round 1 gap #8 (CI supply-chain hardening) and the weekly scheduled
  advisory run. Advisories are now caught only when someone runs the script,
  so run it before every commit.
