---
id: F-002
title: Artifact Discovery
type: feature        # feature | refactor | bugfix | chore
system: cipherglass
status: implemented
area: discovery
owners:
  - three_seat
created_at: 2026-10-11
contract: docs/features/F-002/contract.yaml
requirements:        # system-overview REQ IDs this feature implements (see Traceability)
  - REQ-011
  - REQ-010
  - REQ-030
  - REQ-004
  - REQ-005
  - REQ-022
adrs:
  - ADR-0001
---

# Summary

Find the supported JavaScript files in a target directory and return them as
Cipherglass-owned `Artifact` values, in a deterministic order, together with a
record of every entry that discovery skipped and why. Discovery reads
filesystem metadata only. It never opens or reads file content. F-002 is
library-only: the pipeline and CLI that consume it arrive with F-003 to F-005.

# Context

- **Problem.** Every later stage starts from a list of artifacts. That list
  must be complete, stable across runs, and safe to build from a directory
  tree that may be large, oddly shaped or hostile (A-002).
- **V0 feature.** #2, Artifact Discovery.
- **Pipeline stages.** Artifact. Discovery produces the input to the parser
  adapter stage (F-003).
- **Constraints.**
  - No new third-party dependencies. The walk uses `std::fs` only.
  - `Artifact` lives in `cipherglass-core`, because evidence (REQ-051) and
    findings (REQ-091, REQ-135) must refer to it. The discovery function lives
    in the `cipherglass` facade, which ADR-0001 assigns discovery to.
  - Linux is the development platform. Symlink and permission tests are
    Unix-only.
- **Governing documents.** `docs/system-overview.md` v0.3.0; ADR-0001 (crate
  layout, determinism convention).

## Traceability

| REQ | Role in F-002 |
|-----|---------------|
| REQ-011 | **Implements.** Discover supported JavaScript files within a target directory. |
| REQ-010 | **Partial.** Implements the extension criterion (`.js`, `.mjs`, `.cjs`). The "parseable by the adapter" criterion belongs to F-003. |
| REQ-030 | **Implements** the JavaScript artifact type. |
| REQ-004, REQ-005 | **Implements** for discovery output: identical trees yield identical artifact and skip lists, in identical order. |
| REQ-022 | **Implements** for discovery: directory structure and names are untrusted. No links are followed and no content is read. |
| REQ-023 | **Preserved.** Discovery never opens artifact content. |
| REQ-021, REQ-021a | **Prepares for.** An unreadable entry is recorded and discovery continues. Parse-failure recording itself is F-003. |
| REQ-140 | **Preserved.** Local files only (A-001). |
| REQ-136 | **Partial.** Unit tests for the artifact stage. |

# Behavior

## User flow

- **Command / library entrypoint:** `cipherglass::discover(target, &options)`.
  No CLI subcommand in F-002.
- **Inputs:** a target directory path; `DiscoveryOptions` (maximum file size,
  `max_file_size: u64`, default 32 MiB = 33,554,432 bytes).
- **Outputs:** `Discovery { root, artifacts, skipped }`, or a `DiscoveryError`.
- **Error cases:** target does not exist; target is not a directory; target
  directory cannot be read.

## Detailed behavior

### Target

- The target must be a directory. Anything else returns `DiscoveryError`
  naming the target and the cause. Nothing is skipped silently.
- If the target itself is a symlink to a directory, it is followed (Q10).
  This is the only link discovery ever follows.
- A single file as the target is rejected (REQ-011 says a directory).
- Target errors, decided from `fs::metadata` (which follows the target link):
  a missing target or a dangling target link is `NotFound`; a file, or a link
  to a file, is `NotADirectory`; any other `fs::metadata` failure, or a
  directory whose listing cannot be opened, is `Unreadable`.

### Walk

- Traversal is iterative (an explicit stack), not recursive, so a deeply
  nested tree cannot overflow the call stack.
- Each entry is classified with `DirEntry::file_type()`, which does not follow
  symlinks.
- Hidden files and directories, and `node_modules`, are walked like any other
  entry. There are no default exclusions in v0.
- Below the target, discovery calls only directory listing and
  `symlink_metadata` (through `DirEntry`). The only call that follows a link
  is `fs::metadata` on the target itself (Q10). It never opens a file, so a
  FIFO or device node cannot block it.
- If listing a directory fails part-way (the `ReadDir` iterator yields an
  error), discovery stops listing that directory, keeps the entries already
  seen, and records the directory as `Unreadable`. For the target itself the
  recorded path is empty (`""`), meaning "the target".
- **Path length.** The walk opens directories by full path, so a directory
  whose full path is 4,096 bytes or longer (Linux, `ENAMETOOLONG`; 4,095
  works) cannot be listed. It is recorded as `Unreadable` like any other listing
  failure. Measured on the dev machine: with one-byte names the limit is hit
  at depth 1,993. `std` has no stable `openat`-style API that avoids this.
- **Known limitation: mount loops.** A bind mount can create a directory
  cycle without any symlink. Discovery does not detect it; the path-length
  limit ends the walk (Q11).

### Classification

For each entry:

| Entry | Result |
|-------|--------|
| Directory | Walked. |
| Symlink (to anything, including a dangling link) | Skipped: `Symlink`. |
| Regular file with extension exactly `js`, `mjs` or `cjs` and size ≤ limit | `Artifact`. |
| Regular file with a matching extension and size > limit | Skipped: `TooLarge { size, limit }`. |
| Non-regular file (FIFO, socket, device) with a matching extension | Skipped: `NotRegularFile`. |
| Any other file | Ignored, not recorded. |
| Directory that cannot be listed, or an entry whose type or metadata cannot be read | Skipped: `Unreadable { kind, os_code }`, and the walk continues. |

- The extension match is case-sensitive and uses `Path::extension`. `app.JS`
  is ignored, and a file named exactly `.js` has no extension and is ignored.
- Classification is by extension only, never by content. JavaScript is plain
  text with no magic number (a hashbang or a byte-order mark is optional),
  so sniffing could not identify it reliably, and it would mean opening files.
  Whether the content really is JavaScript is decided by the parser in F-003
  (REQ-010's second criterion).
- Every symlink is recorded, because a link to a directory can hide files the
  user expected to be scanned. This is noisy in `node_modules/.bin` and pnpm
  layouts; accepted, since links whose targets are inside the tree lose
  nothing (the targets are walked directly).
- Hard links, or the same file reached by two paths, are returned once per
  path. Empty files are returned; whether they parse is F-003's concern.

### Output

- `Artifact { path, size }`. `path` is relative to the target. `size` is the
  byte length from `symlink_metadata` at discovery time.
- `Discovery { root, artifacts, skipped }`. `root` is the target path as
  given. `skipped` holds `SkippedEntry { path, reason }`, with `path` relative
  to the target.
- `artifacts` and `skipped` are each sorted by `Path` ordering (component by
  component, bytes within each component on Unix). Paths are kept as
  `PathBuf`, so non-UTF-8 names are preserved, not lossily converted.
- An empty target, or one with no supported files, is a success with an empty
  `artifacts` list.
- `Unreadable` stores `kind: std::io::ErrorKind` and `os_code: Option<i32>`
  (from `raw_os_error`), not `io::Error`, so every output type can derive
  `Debug, Clone, PartialEq, Eq` and two runs can be compared with `==`.
- `DiscoveryOptions`, `SkipReason`, `DiscoveryError`, `Discovery`,
  `SkippedEntry` and `Artifact` are `#[non_exhaustive]`, so later options
  (exclusions, Q2), reasons and artifact types (REQ-030) are not breaking
  changes. `DiscoveryOptions` implements `Default`. `Artifact::new(path,
  size)` lets the facade build artifacts from outside `cipherglass-core`.
- There is no cap on the number of entries. A tree with millions of files
  costs memory in proportion; accepted for v0.

### Handoff to F-003

- The size check is made from metadata. A file can grow between discovery and
  parsing, so the parser stage must also bound how many bytes it reads.
- A file can be replaced by a symlink after discovery. The parser stage must
  open without following links, or re-check the file type.

# Non-Functional Requirements

- **Determinism (REQ-004, REQ-005):** both output lists are sorted before
  return. No `HashMap` iteration affects output (ADR-0001).
- **Untrusted input (REQ-022, REQ-023):** no links followed below the target,
  no file content read, iterative traversal, size limit enforced.
- **Fault tolerance (REQ-021):** one unreadable entry never stops the walk.
  Only a target-level failure returns an error.
- **Performance:** at most one metadata call per entry; the type usually
  comes free from the directory listing, and size is read only for regular
  files with a matching extension. No content reads.
- **Portability:** standard library only. Symlink, FIFO and permission tests
  are `cfg(unix)`. The non-UTF-8 and path-length tests are
  `cfg(target_os = "linux")`, since macOS filesystems reject non-UTF-8 names
  and have a different path limit.
- **UX:** every skipped entry carries a reason a user can act on.

# Acceptance Criteria

- [ ] AC-1 `discover` returns every `.js`, `.mjs` and `.cjs` regular file under the target, including inside hidden directories and `node_modules`, with paths relative to the target.
- [ ] AC-2 Files with other extensions, including `.JS` and a file named `.js`, are not returned and not recorded.
- [ ] AC-3 Two runs over the same tree return identical `artifacts` and `skipped` lists, both sorted by `Path` order.
- [ ] AC-4 A symlink below the target, to a file or a directory or dangling, is not followed and is recorded as `Symlink`.
- [ ] AC-5 A matching file larger than the limit is recorded as `TooLarge` with its size and the limit, and is not returned.
- [ ] AC-6 A directory that cannot be listed is recorded as `Unreadable`, and files in its sibling directories are still returned.
- [ ] AC-7 A FIFO with a matching extension is recorded as `NotRegularFile`, and discovery does not block on it. The test runs `discover` on a thread and fails if it has not returned within 10 seconds.
- [ ] AC-8 A missing target, a dangling target link, a target that is a file, and an unreadable target return `NotFound`, `NotFound`, `NotADirectory` and `Unreadable` respectively.
- [ ] AC-9 A target that is a symlink to a directory is walked.
- [ ] AC-10 A file name that is not valid UTF-8 is returned with its bytes preserved.
- [ ] AC-11 A chain of 1,900 nested directories with a `.js` file at the bottom (full path under 4,096 bytes) returns that file. A chain whose full path is 4,096 bytes or longer records the first too-long directory as `Unreadable` and still returns a `.js` file in a sibling of the chain. The test builds the long chain by renaming near-top directories to 255-byte names after creating it, because the process-wide working directory cannot be changed safely in parallel tests.
- [ ] AC-12 No new third-party dependency is added, and the architecture test still passes.
- [ ] AC-13 `scripts/check.sh` passes.
- [ ] AC-14 A directory whose listing fails part-way is recorded once as `Unreadable`; the entries returned before the failure are kept. A real filesystem cannot be made to fail mid-listing on demand, so the per-directory step takes an iterator of `io::Result` entries and is unit-tested with a synthetic iterator that fails part-way.

# Implementation Notes

- **Crates/modules to touch:**
  - `cipherglass-core`: `Artifact`.
  - `cipherglass`: `discover`, `DiscoveryOptions`, `Discovery`,
    `SkippedEntry`, `SkipReason`, `DiscoveryError`.
- **Refactors:** none.
- **Dependencies:** none added. Tests create trees under
  `env!("CARGO_TARGET_TMPDIR")`, so no `tempfile` crate is needed.
- **Owned types:** `Artifact` is the first Cipherglass-owned type. It is an
  artifact type, not an evidence type, so REQ-114 does not apply.
- **Fixtures:** none committed. Symlinks, FIFOs, permissions and non-UTF-8
  names cannot be stored reliably in git, so tests build their trees at run
  time. The unreadable-directory test is skipped when running as root, since
  root ignores permission bits.

# Open Questions

- Q11 (closed 2026-10-22): Detect directory cycles from bind mounts (track
  visited device and inode)? — No in v0. Only a privileged user can create a
  mount, a hostile archive cannot, and the path-length limit already ends the
  walk.
- Q3 (closed 2026-10-22): Default maximum file size? — 32 MiB. Large
  production bundles are usually under 15 MB.
- Q10 (closed 2026-10-22): Follow the target itself when it is a symlink? —
  Yes, because the user named it explicitly. Links below the target are never
  followed.
- Q1 (closed): Follow symlinks below the target? — No. Record each one as
  `Symlink`. Avoids loops and walking outside the target.
- Q2 (closed): Default exclusions (hidden directories, `node_modules`)? — None
  in v0. Exclusion options can come later.
- Q4 (closed): Unreadable entries? — Record as `Unreadable` and continue. A
  target-level failure is an error.
- Q5 (closed): Extension case? — Exact lowercase, as REQ-010 is written.
- Q6 (closed): Output order? — `Path` ordering on paths relative to the
  target. Non-UTF-8 names kept as `PathBuf`.
- Q7 (closed): Where do the types live? — `Artifact` in `cipherglass-core`;
  the discovery function and its result types in `cipherglass`.
- Q8 (closed): CLI subcommand? — No. Library only.
- Q9 (closed): Single file as target? — No. REQ-011 says a directory.
