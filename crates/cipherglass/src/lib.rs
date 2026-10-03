//! Cipherglass: static security reconnaissance of deployed client files.
//!
//! This facade wires the pipeline together (discovery, parser adapters,
//! analyzers, exporters) and is the single public library surface.
//!
//! Boundary: never re-export items from `cipherglass-js`. Only
//! Cipherglass-owned types may cross this API (REQ-044). See ADR-0001.
