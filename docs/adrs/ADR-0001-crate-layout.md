---
id: ADR-0001
title: Crate layout and parser boundary enforcement
status: accepted
created_at: 2026-10-02
feature: F-001
---

# ADR-0001: Crate layout and parser boundary enforcement

## Context

Parser independence is Cipherglass's core architectural constraint: analyzers
must never see third-party parser types (REQ-041, REQ-043, REQ-111), and
public findings must not expose them (REQ-044). In a single crate, that rule
is enforced only by review, and one convenient `use` is enough to break it.

## Decision

Split the workspace into five crates, and constrain which may depend on which.

| Crate                 | Responsibility                                       | May depend on (workspace) |
|-----------------------|------------------------------------------------------|---------------------------|
| `cipherglass-core`    | Cipherglass-owned types: artifact, location, evidence, finding | nothing         |
| `cipherglass-js`      | JavaScript parser adapter; only home for a JS parser | `cipherglass-core`        |
| `cipherglass-graphql` | GraphQL analyzer                                     | `cipherglass-core`        |
| `cipherglass`         | Facade: discovery, pipeline wiring, exporters        | `cipherglass-core`, `cipherglass-js`, `cipherglass-graphql` |
| `cipherglass-cli`     | `cipherglass` binary                                 | `cipherglass`             |

The table applies to every dependency kind: normal, dev and build. Anything
not listed is forbidden.

Each crate also has an approved list of direct third-party dependencies, keyed
by dependency kind. A JavaScript parser can therefore appear only in
`cipherglass-js`, and an analyzer cannot name a parser type because it cannot
depend on the crate that defines it. A JavaScript engine or runtime is never
approved (REQ-023).

## Enforcement

`crates/cipherglass/tests/architecture.rs` reads declared dependencies from
`cargo metadata --no-deps --offline` and fails on:

- a forbidden workspace edge;
- an unapproved third-party dependency;
- a workspace crate missing from the table;
- a publishable crate (the workspace sets `publish = false`).

The checker is a pure function, unit-tested with synthetic violations. Adding
a dependency means updating the approved list in that file, which shows up in
review alongside the feature contract that approves it.

The workspace also sets `unsafe_code = "forbid"`, which every crate inherits.
Cipherglass parses untrusted input (REQ-022).

## Known limitation

The facade could still write `pub use cipherglass_js::SomeParserType`. Stable
Rust has no public/private dependency distinction, and neither cargo nor the
architecture test detects this. It is a review checklist item until a
mechanical check exists. F-003 must revisit it when `cipherglass-js` first
gains public items.

## Determinism convention

REQ-004 and REQ-005 require identical findings, in identical order, for
identical input. Any collection whose iteration order affects output must be
ordered: `BTreeMap`/`BTreeSet`, or a `Vec` sorted before output. `HashMap`
and `HashSet` are fine for lookups but must never be iterated to produce
output. Features that produce findings test this with repeated runs
(REQ-004a, REQ-005a).

## Consequences

- More manifests to maintain than a single crate.
- Every new dependency is an explicit, reviewable change.
- Swapping the JavaScript parser touches `cipherglass-js` only (REQ-120).
