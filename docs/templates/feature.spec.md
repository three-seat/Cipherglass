---
id: F-XXX
title: <title>
type: feature        # feature | refactor | bugfix | chore
system: cipherglass
status: draft
area: foundation | discovery | parser-adapter | evidence | analyzer | finding | exporter | cli | docs
owners:
  - <you>
created_at: <YYYY-MM-DD>
contract: docs/features/F-XXX/contract.yaml
requirements:        # system-overview REQ IDs this feature implements
  - REQ-XXX
adrs: []
---

# Summary

One paragraph: what are we building and why.

# Context

- What problem does this solve?
- Which V0 feature does this belong to (see docs/system-overview.md, "V0 Features")?
- Which pipeline stage(s) does it touch (artifact, parser adapter, evidence, analyzer, finding, exporter)?
- Any constraints (platform, toolchain, crate layout, compatibility)?
- Relevant ADRs or system-overview requirements that govern this feature.

# Behavior

## User flow

- Command / library entrypoint:
- Inputs:
- Outputs:
- Error cases:

## Detailed behavior

- Bullet list of steps, in order.

# Non-Functional Requirements

- Determinism: identical input yields identical findings and order (REQ-004, REQ-005)?
- Untrusted input: how is malformed or hostile artifact content handled (REQ-022, REQ-023)?
- Fault tolerance: does a single-file failure halt the pipeline (REQ-021)?
- Performance:
- Portability:
- UX:

# Acceptance Criteria

- [ ] AC-1
- [ ] AC-2
- [ ] AC-3

# Implementation Notes

- Expected crates/modules to touch:
- Any refactors allowed/required:
- Any dependencies (avoid adding crates unless needed; third-party parsers stay behind a parser adapter, REQ-041):
- Does this feature add or change Cipherglass-owned types (evidence, finding)? Justify per REQ-114.
- Test fixtures needed (sample JavaScript files, expected outputs):

# Open Questions

- Q1: <question>
- Q1 (closed): <question> — <decision>
