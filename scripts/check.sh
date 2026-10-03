#!/usr/bin/env bash
# Runs every F-001 check locally. Nothing here runs on GitHub.
# Requires cargo-deny 0.19.8 and cargo-audit 0.22.2:
#   cargo install --locked cargo-deny --version 0.19.8
#   cargo install --locked cargo-audit --version 0.22.2
# cargo deny and cargo audit fetch the RustSec advisory database.
set -euo pipefail
cd "$(dirname "$0")/.."

step() { printf '\n==> %s\n' "$*"; "$@"; }

step cargo fmt --all --check
step cargo build --workspace --locked
step cargo clippy --workspace --all-targets --locked -- -D warnings
step cargo test --workspace --locked
step cargo deny --locked check
step cargo audit

printf '\nAll checks passed.\n'
