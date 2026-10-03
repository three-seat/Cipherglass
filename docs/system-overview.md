---
title: System Overview
system: cipherglass
status: baselined
version: 0.3.0
created_at: 2026-08-18
changelog: |
  0.2.0 - Added requirement IDs. Split multi-action SHALL statements into
          single-requirement sentences. Added GraphQL Wordlist Export section
          for Clairvoyance interop. Added determinism requirement.
  0.3.0 - Split remaining bundled requirements (determinism, error handling,
          casing, wordlist output destination, finding preservation,
          end-to-end success criteria) into fully atomic statements. Added
          artifact non-execution requirement. Aligned enum-value recovery
          between analyzer and wordlist sections. Changed finding dedup to
          preserve multiple source locations instead of discarding repeats.
          Tightened "GraphQL evidence" language in success criteria with an
          explicit definition. Demoted future-boundary continuity note to
          non-normative guidance. Removed silent default wordlist path.
          Generalized Clairvoyance description to "external GraphQL
          schema-recovery tools." Tightened Evidence definition wording.
          Moved assumption/rationale text out of normative requirement
          bodies into blockquote notes. BASELINED.
---

# System Overview

## Purpose

Cipherglass is a Rust library and command-line tool for static security reconnaissance.

**REQ-001** Cipherglass SHALL analyze deployed client files for security-relevant information.

**REQ-002** Cipherglass SHALL separate file parsing from security analysis.

**REQ-003** Cipherglass SHALL preserve the source of each reported finding.

GraphQL analysis is the first supported analysis feature.

**REQ-004** Cipherglass SHALL produce deterministic findings.

**REQ-004a** For identical input, the set of findings produced on repeated runs SHALL be identical.

**REQ-005** Cipherglass SHALL produce deterministic finding order.

**REQ-005a** For identical input, the order of findings produced on repeated runs SHALL be identical.

---

## V0 Scope

**REQ-010** Cipherglass v0 SHALL support local JavaScript files.

A supported JavaScript file MEETS the following criteria:
- file extension is `.js`, `.mjs`, or `.cjs`
- file is valid ECMAScript parseable by the configured parser adapter
- file MAY be minified or unminified
- file MAY be a module or a script

**REQ-010a** A file that the parser adapter parses only in error-tolerant or recovery mode SHALL still be treated as supported, provided the adapter returns partial output rather than a fatal error.

> Rationale: best-effort partial parsing is preferable to dropping a file entirely, since recon value can exist even in partially-malformed input.

**REQ-011** Cipherglass v0 SHALL discover supported JavaScript files within a target directory.

**REQ-012** Cipherglass v0 SHALL parse discovered JavaScript files through a parser adapter.

**REQ-013** Cipherglass v0 SHALL convert parser output into Cipherglass evidence.

**REQ-014** Cipherglass v0 SHALL analyze evidence for GraphQL information.

**REQ-015** Cipherglass v0 SHALL report findings in a human-readable format.

**REQ-016** Cipherglass v0 SHALL report findings in a machine-readable format.

**REQ-017** Cipherglass v0 SHALL export a wordlist for use by external GraphQL tools. *(promoted from MAY — see Wordlist Export section)*

---

## Processing Model

Cipherglass uses the following processing model:

```
Artifact
   ↓
Parser Adapter
   ↓
Evidence Extraction
   ↓
Cipherglass Evidence
   ↓
Analyzer
   ↓
Finding
   ↓
Exporter
```

**REQ-020** Each stage SHALL have a defined responsibility.

**REQ-021** Cipherglass SHALL NOT halt the pipeline on a single-file parse failure.

**REQ-021a** Cipherglass SHALL record a parse failure and continue processing remaining artifacts.

**REQ-022** Cipherglass SHALL treat all artifact content as untrusted input.

**REQ-023** Cipherglass SHALL NOT execute analyzed artifact content.

---

## Artifact

An artifact is an input file that Cipherglass can inspect.

**REQ-030** Cipherglass v0 SHALL support JavaScript artifacts.

Future versions MAY support other artifact types, including:
- WebAssembly modules
- Hermes bytecode
- source maps
- other deployed client files

---

## Parser Adapter

A parser adapter reads one artifact format.

**REQ-040** A parser adapter MAY depend on a third-party parser library.

**REQ-041** Cipherglass SHALL isolate third-party parser libraries behind parser adapters.

**REQ-042** Parser adapters SHALL convert third-party parser output into Cipherglass-owned types.

**REQ-043** Third-party parser AST types SHALL NOT be exposed to analyzers.

**REQ-044** Third-party parser AST types SHALL NOT be exposed in public findings.

This boundary allows Cipherglass to replace a parser library without changing analyzers.

---

## Evidence

Evidence is information recovered from an artifact for analysis by one or more analyzers.

**REQ-050** Cipherglass evidence SHALL use Cipherglass-owned types.

Evidence MAY represent:
- text
- identifiers
- structured objects
- function calls
- assignments
- relationships
- source locations
- parser context

**REQ-051** Evidence SHALL preserve the source artifact.

**REQ-052** Evidence SHOULD preserve the source location when the parser provides it.

**REQ-053** Evidence MAY preserve origin information.

Origin information MAY describe how the evidence was found. Examples include:
- JavaScript string literal
- JavaScript template literal
- JavaScript tagged template
- JavaScript object
- WebAssembly data section
- Hermes string table

**REQ-054** Analyzers MAY use origin information to adjust confidence or provide better provenance.

**REQ-055** Analyzers SHALL NOT depend on a specific parser implementation.

---

## Analyzer

An analyzer reads Cipherglass evidence and looks for information from one domain.

**REQ-060** An analyzer SHALL consume Cipherglass-owned evidence types.

**REQ-061** An analyzer SHALL NOT depend directly on third-party parser AST types.

**REQ-062** An analyzer SHALL determine whether evidence is relevant to its domain.

**REQ-063** An analyzer MAY use a domain-specific parser.

**REQ-064** GraphQL SHALL be the first supported analyzer.

Future analyzers MAY inspect evidence for:
- API endpoints
- authentication configuration
- WebSocket usage
- security-sensitive code patterns
- other application behavior

---

## GraphQL Analyzer

**REQ-070** The GraphQL analyzer SHALL consume Cipherglass evidence.

**REQ-071** The GraphQL analyzer SHALL detect evidence that contains or represents GraphQL information.

**REQ-072** The GraphQL analyzer SHALL use a GraphQL parser when valid GraphQL source is available.

**REQ-073** The GraphQL analyzer SHALL recover GraphQL structure when possible.

Recovered information MAY include:
- operation type
- operation name
- fields
- nested field paths
- arguments
- variables
- variable types
- fragments
- fragment spreads
- enum values

**REQ-074** The GraphQL analyzer MAY recognize compiled GraphQL representations.

**REQ-075** The GraphQL analyzer MAY identify persisted-query information.

**REQ-076** The GraphQL analyzer SHALL generate vocabulary for external schema-recovery tools. *(promoted from MAY — see Wordlist Export section)*

**REQ-077** Cipherglass SHALL NOT require Clairvoyance or another external GraphQL tool.

---

## GraphQL Wordlist Export

This section defines the interop contract with external GraphQL schema-recovery tools (e.g. Clairvoyance's `-w` custom-wordlist flag).

**REQ-080** Cipherglass SHALL export a wordlist as a plain-text file, one term per line, with no header row.

**REQ-081** The wordlist SHALL be encoded as UTF-8 with LF line endings.

**REQ-082** The wordlist SHALL include, when recovered, the following vocabulary classes:
- field names
- argument names
- type names
- enum values
- fragment names

**REQ-083** The wordlist SHALL NOT contain duplicate lines. Term comparison for deduplication SHALL be case-sensitive.

**REQ-084** The wordlist SHOULD preserve original casing as recovered from evidence.

**REQ-084a** Cipherglass SHALL NOT normalize case across variants.

> Example: `userId` and `UserId`, if both recovered, are retained as distinct terms.

**REQ-085** Cipherglass SHALL require an explicit output destination for the wordlist.

**REQ-085a** Cipherglass SHALL NOT write a default-named wordlist file when no destination is specified.

> Note: CLI-level detail deferred — a specific default-to-stdout behavior, e.g. `cipherglass graphql wordlist <target> --output <path>`, belongs in the CLI feature spec rather than this document. These requirements only fix the architectural rule.

**REQ-086** The wordlist export SHALL contain only vocabulary terms. It SHALL NOT contain URLs, endpoints, file paths, or other non-vocabulary evidence.

Note: Cipherglass does not discover or export GraphQL endpoint URLs in v0. Supplying a target URL to Clairvoyance (via `-e`) remains the user's responsibility.

---

## Finding

A finding is a normalized result produced by an analyzer.

**REQ-090** A finding SHALL identify its type.

**REQ-091** A finding SHALL preserve the source artifact.

**REQ-092** A finding SHOULD preserve the source location when available.

**REQ-093** A finding MAY include a confidence value.

**REQ-094** A finding MAY include analyzer-specific structured data.

**REQ-095** A finding SHALL use Cipherglass-owned types.

**REQ-096** Cipherglass SHALL preserve a finding for each source location at which a GraphQL element is recovered.

**REQ-096a** Cipherglass SHALL NOT discard findings to collapse repeated occurrences of the same element across source locations.

---

## Exporter

An exporter converts findings into an output format.

**REQ-100** Cipherglass v0 SHALL provide human-readable output.

**REQ-101** Cipherglass v0 SHALL provide machine-readable output.

**REQ-102** Cipherglass v0 MAY provide GraphQL-specific output.

**REQ-103** Cipherglass v0 SHALL provide a GraphQL vocabulary wordlist as defined in the GraphQL Wordlist Export section. *(promoted from MAY)*

---

## Architectural Boundaries

Parser independence is a core architectural constraint.

**REQ-110** Parser adapters MAY depend on third-party parser libraries.

**REQ-111** Analyzers SHALL NOT depend on third-party parser implementations.

**REQ-112** Analyzers SHALL consume Cipherglass evidence.

**REQ-113** Cipherglass SHALL NOT define a universal programming-language AST.

**REQ-114** Cipherglass SHOULD add new evidence types only when a feature requires information that the current evidence model cannot represent.

---

## Extensibility

**REQ-120** Cipherglass SHALL allow new artifact parsers without requiring changes to existing analyzers when the existing evidence model is sufficient.

**REQ-121** Cipherglass SHALL allow new analyzers without requiring changes to existing artifact parsers when the existing evidence model is sufficient.

This design allows the following model:

```
               GraphQL   API   Auth   Other
JavaScript        ✓       ✓     ✓      ✓
WebAssembly       ✓       ✓     ?      ✓
Hermes            ✓       ✓     ✓      ✓
Source maps       ✓       ✓     ✓      ✓
```

Future support for these combinations is not required for v0.

---

## V0 Features

Cipherglass v0 is expected to contain the following features:

1. Project Foundation
2. Artifact Discovery
3. JavaScript Parsing and Evidence Extraction
4. GraphQL Analysis
5. Findings and Export

---

## V0 Success Criteria

**REQ-130** Cipherglass v0 SHALL operate end to end without manual intervention between pipeline stages.

**REQ-130a** A user SHALL be able to point the CLI at a directory of supported JavaScript files and receive GraphQL reconnaissance output.

**REQ-131** For each supported file containing a valid, recoverable GraphQL document, Cipherglass SHALL produce at least one finding with source artifact and type populated.

*Definition: a "recoverable GraphQL document" is evidence that the GraphQL analyzer successfully parses into GraphQL structure per REQ-073, as distinct from evidence that merely contains a GraphQL-suggestive string (e.g. the substring "/graphql") without semantic structure.*

**REQ-132** The implementation SHALL preserve the parser adapter boundary.

**REQ-133** The GraphQL analyzer SHALL operate on Cipherglass evidence.

**REQ-134** The GraphQL analyzer SHALL NOT require JavaScript-specific AST types.

**REQ-135** Applicable findings SHALL identify their source artifact.

**REQ-136** The project SHALL include automated unit tests for each pipeline stage (parser adapter, evidence extraction, analyzer, exporter).

**REQ-137** The project SHALL include automated integration tests covering the end-to-end pipeline (REQ-130) against a fixture set of JavaScript files.

---

## V0 Non-Goals

**REQ-140** Cipherglass v0 SHALL NOT crawl websites.

**REQ-141** Cipherglass v0 SHALL NOT operate as an HTTP proxy.

**REQ-142** Cipherglass v0 SHALL NOT perform active GraphQL probing.

**REQ-143** Cipherglass v0 SHALL NOT perform schema brute forcing.

**REQ-144** Cipherglass v0 SHALL NOT exploit discovered vulnerabilities.

**REQ-145** Cipherglass v0 SHALL NOT perform full JavaScript taint analysis.

**REQ-146** Cipherglass v0 SHALL NOT analyze WebAssembly.

**REQ-147** Cipherglass v0 SHALL NOT analyze Hermes bytecode.

**REQ-148** Cipherglass v0 SHALL NOT require source maps.

**REQ-149** Cipherglass v0 SHALL NOT provide a graphical user interface.

---

## Future Direction

Future versions MAY support additional artifact formats. Candidate artifact formats include:
- WebAssembly
- Hermes bytecode
- source maps

Future versions MAY support additional analyzers. Candidate analyzers include:
- API discovery
- OAuth and OIDC configuration discovery
- WebSocket discovery
- prototype pollution candidate discovery
- other static security analysis

Future versions MAY collect artifacts from an intercepting proxy.

Future features are expected to use the same artifact, parser adapter, evidence, analyzer, finding, and exporter boundaries when those boundaries remain suitable. This is guidance for future design discussions, not a testable v0 requirement — architecture is expected to evolve as new artifact types and analyzers are added.
