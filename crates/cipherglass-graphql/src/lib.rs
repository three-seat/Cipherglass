//! GraphQL analyzer: finds GraphQL information in Cipherglass evidence.
//!
//! Boundary: this crate consumes Cipherglass evidence only. It must never
//! depend on `cipherglass-js` or a JavaScript parser (REQ-061, REQ-111). It
//! may depend on `cipherglass-core` only. See ADR-0001.
