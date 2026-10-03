# Cipherglass
Find useful security info in deployed client files.

## Build and test

Requires a stable Rust toolchain (edition 2024).

Run every check (format, build, lint, test, `cargo deny`, `cargo audit`):

```sh
scripts/check.sh
```

The script lists the exact `cargo-deny` and `cargo-audit` versions to
install. Nothing runs automatically on GitHub; run it before every commit.

Run the CLI:

```sh
cargo run -p cipherglass-cli -- --help
```

## Layout

The workspace is split into crates that enforce the parser adapter boundary.
See [ADR-0001](docs/adrs/ADR-0001-crate-layout.md) and
[the system overview](docs/system-overview.md).
