//! JavaScript parser adapter: parses JavaScript artifacts and converts the
//! parser's output into Cipherglass evidence.
//!
//! Boundary: this is the only crate allowed to depend on a third-party
//! JavaScript parser (REQ-041). Its public API must expose only
//! Cipherglass-owned types (REQ-042). It may depend on `cipherglass-core`
//! only. See ADR-0001.
