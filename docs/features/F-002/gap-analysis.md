# F-002 Gap Analysis

## Round 1: spec draft

Compared `spec.md` (draft of 2026-10-11, Q3 and Q10 closed 2026-10-22) against
`docs/system-overview.md` v0.3.0, ADR-0001, `docs/templates/feature.spec.md`,
and whether the design actually works on the dev machine (Linux, WSL2,
rustc 1.98.0).

Severity: **H** = would let a system-overview requirement be broken or makes
an AC impossible to verify; **M** = ambiguity an implementer would have to
guess at; **L** = polish.

| #  | Sev | Gap | Resolution in spec |
|----|-----|-----|--------------------|
| 1  | H | **AC-11 cannot be verified as written.** It asks for a tree deeper than recursion can handle, but the walk opens directories by full path, and the OS refuses paths over 4,096 bytes first. Probed: with one-byte names, listing fails with `ENAMETOOLONG` at depth 1,993, long before a recursive walk would overflow the stack. A test for the AC could never fail for the reason it names. The spec also never said what happens to a directory beyond the limit. | AC-11 now names concrete depths: a 1,900-deep chain is walked in full; a chain beyond 4,096 bytes records the first too-long directory as `Unreadable` and siblings are still returned. A new "Path length" rule in Walk explains this. The traversal stays iterative, which costs nothing. Test mechanism: rename near-top directories to 255-byte names after creating the chain, because changing the working directory is process-wide and unsafe in parallel tests. |
| 2  | H | **`Unreadable { error }` breaks AC-3.** `io::Error` is not `Clone` or `PartialEq`, so the two runs cannot be compared with `==`, and none of the output types can derive the traits the tests and later stages need. | `Unreadable { kind: io::ErrorKind, os_code: Option<i32> }`. All output types derive `Debug, Clone, PartialEq, Eq`. |
| 3  | M | **Errors part-way through a listing were not covered.** The `ReadDir` iterator can yield an `Err` with no entry name. The spec did not say which path to record, whether to keep listing (and risk looping on the same error), or what happens when this occurs in the target itself. | Stop listing that directory at the first iterator error, keep the entries already seen, and record the directory once as `Unreadable`. In the target the recorded path is `""`. Added AC-14, tested through a synthetic iterator because a real filesystem cannot be made to fail on demand. Which entries were seen before the error depends on the listing order. That is acceptable: an I/O failure means the input is not identical (REQ-004a). |
| 4  | M | **Bind-mount loops.** A bind mount can create a directory cycle without any symlink, which the "no links followed" rule does not prevent. | Recorded as a known limitation. Creating one needs privilege (a hostile archive cannot), and the path-length limit (#1) ends the walk. Left to you as **Q11**. |
| 5  | M | **AC-7 would hang, not fail.** If a regression made discovery open a FIFO, the test would block for ever, with no failure message. | The test runs `discover` on a thread and fails after 10 seconds. |
| 6  | M | **Public types could not be extended.** Q2 promises exclusion options later. Adding a field to `DiscoveryOptions`, or a variant to `SkipReason`/`DiscoveryError`, would be a breaking change. The default limit was also not written as a number. | These three types are `#[non_exhaustive]`. `DiscoveryOptions: Default` with `max_file_size: u64 = 33,554,432`. |
| 7  | L | **Target error cases were incomplete.** The spec did not cover a dangling target link or a link to a file. | `fs::metadata` decides: missing or dangling → `NotFound`, file or link to a file → `NotADirectory`, listing cannot be opened → `Unreadable`. AC-8 names each one. |
| 8  | L | **The non-UTF-8 test was `cfg(unix)`.** macOS filesystems reject non-UTF-8 names, so the test would fail there. | The non-UTF-8 and path-length tests are `cfg(target_os = "linux")`. |
| 9  | L | **The performance claim was inaccurate.** It said "one metadata call per entry", but the type usually comes free from the listing. | Reworded to "at most one". Size is read only for regular files with a matching extension. |
| 10 | L | **Hard links and empty files were not mentioned.** | The same file reached by two paths is returned once per path. Empty files are returned, and whether they parse is F-003's concern. |
| 11 | L | **Symlink noise.** `node_modules/.bin` and pnpm layouts are mostly symlinks, so `skipped` will be long. | Accepted and noted. A link whose target is inside the tree hides nothing, because the target is walked directly. Exclusions (Q2) can trim this later. |
| 12 | L | **Unbounded entry count.** A tree with millions of files uses memory in proportion. | Accepted for v0 and noted in Output. |

### Checked, no gap

- The frontmatter `requirements` matches the traceability table. Partial and
  prepares-for claims are marked as such (no repeat of F-001 round 1 #1).
- The spec has every section in `docs/templates/feature.spec.md`.
- Determinism: both lists are sorted with `Path` ordering, and no hash
  iteration reaches the output, as ADR-0001 requires.
- Hidden entries such as `.hidden.js` and `foo.min.js` have extension `js`
  and are returned. A directory named `x.js` is walked, not returned.
- `DirEntry::file_type` and `DirEntry::metadata` do not follow symlinks on
  Unix. Where the filesystem gives `DT_UNKNOWN`, std falls back to `lstat`.
- No new dependency is needed. The test trees go under `CARGO_TARGET_TMPDIR`,
  and std's `remove_dir_all` on Linux works through directory file descriptors
  (`openat`), so it can delete the over-limit chain from #1.

### Decisions (2026-10-22)

- **Q3:** 32 MiB default maximum file size, as proposed.
- **Q10:** the target is followed when it is a symlink, as proposed.
- **Q11:** no bind-mount cycle detection in v0, as proposed.

## Round 2: contract against spec

Compared `contract.yaml` against the revised `spec.md` and
`docs/system-overview.md`, and probed the test mechanisms that were not
obviously workable. Probes were a scratch Rust program built with rustc 1.98.0
on the dev machine (Linux, WSL2, ext4).

| #  | Sev | Gap | Resolution |
|----|-----|-----|------------|
| 1  | H | **Cleaning up the deep chain fails on a normal machine.** `fs::remove_dir_all` keeps one file descriptor open per level. Probed: on a 2,500-deep chain it failed with EMFILE under `ulimit -n 1024`, a common default. It works on the dev machine only because WSL sets the limit to 1,048,576. T-13's setup ("remove first if it exists") and its cleanup would both fail. | New M-5: T-13 creates its chain with a loop of `create_dir` and removes it bottom-up with a loop of `remove_dir`. M-3 (T-14) now uses at most 200 levels, and gets its length from 255-byte names, so `remove_dir_all` needs few descriptors. |
| 2  | M | **T-3 could pass without sorting.** Two runs over the same tree list entries in the same order, so an implementation that forgot to sort would still give equal output. With a few files, the listing order may also happen to be sorted already. | T-3 needs at least 50 artifacts across several directories and checks that both lists are sorted. With 50 entries, listing order matching sorted order by chance is very unlikely. Building the tree twice in different orders would not help: ext4 lists in name-hash order, whatever the creation order. |
| 3  | M | **A failed permission test blocks later runs.** If a test panics before restoring `chmod 000`, the next run's "remove first" cannot list that directory, and the test fails for an unrelated reason. | M-1 now restores permissions in a guard's `Drop`, which also runs when a test panics. |
| 4  | M | **`Artifact` could not grow.** REQ-030 names future artifact types (WebAssembly, Hermes), which will likely need a field. With plain public fields, adding one breaks every stage. `#[non_exhaustive]` alone would stop the facade, a different crate, from building artifacts. | LL-10: `Artifact`, `Discovery` and `SkippedEntry` are `#[non_exhaustive]`, and `Artifact::new(path, size)` is the way to build one. T-18 checks that it can be used from outside cipherglass-core. |
| 5  | L | **The path limit was off by one.** Probed: a 4,095-byte path lists; a 4,096-byte path fails with ENAMETOOLONG. The spec and contract said "exceeds 4,096". | Reworded to "4,096 bytes or longer". The error kind is `InvalidFilename` and the OS code is 36 (probed); T-14 asserts both. |
| 6  | L | **T-9 named the privilege probe without needing it.** Root can read the metadata of a `chmod 000` file too, so the result does not depend on privilege. | M-1 removed from T-9. |
| 7  | L | **T-13 depends on where the repository is checked out.** The 1,900 levels take 3,800 bytes. Under `target/tmp` (52 bytes here) that fits, but a checkout path longer than about 250 bytes would break it in a confusing way. | M-6: the test first asserts that the root path is short enough, with a message that says why. |
| 8  | L | **The spec and contract disagreed in wording.** The spec's User flow left `root` out of `Discovery`. Its Walk section said only `symlink_metadata` is called, but LL-1 uses `fs::metadata` on the target. | Spec updated: `root` is listed, and the Walk section names `fs::metadata` on the target as the only call that follows a link. |
| 9  | L | **No mapping from requirements to tests**, although the review checklist asks that "every requirements.system entry is traced to a test". | Added `requirements_map`. |
| 10 | L | **A target that is a symlink loop was not covered.** | It falls under ERR-3 (`Unreadable`, kind FilesystemLoop). Added to T-10. |

### Checked, no gap

- `requirements.system` matches the spec's frontmatter. Every spec AC (1 to
  14) maps to a test or check.
- `DirEntry::metadata` still works for an entry whose full path is over the
  limit (probed). It reads relative to the open directory, so only listing
  is affected, as KL-2 says.
- `mkfifo` is at `/usr/bin/mkfifo` (M-2).
- Integration tests are a separate crate, so they cannot build
  `DiscoveryOptions` with a struct literal. `DiscoveryOptions::default()`
  followed by setting the field works with `#[non_exhaustive]`, and T-5 uses
  it.
- No `writes_paths` entry overlaps `must_not_modify`.

### Question raised during round 2 (2026-10-22)

- **Check by content instead of extension?** No. JavaScript has no magic
  number, so content sniffing cannot identify it, and sniffing would break
  INV-2 (never open a file). Added to the spec's Classification section.

Nothing blocks moving the contract from `draft` to `in_progress`.
