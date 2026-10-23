//! Cipherglass-owned types shared by every pipeline stage: artifact, source
//! location, evidence, and finding.
//!
//! Boundary: this crate depends on no other workspace crate and on no
//! third-party parser. Findings defined here therefore cannot expose parser
//! types (REQ-044, REQ-095). See ADR-0001.

mod artifact;

pub use artifact::Artifact;
