//! The artifact type (REQ-030, F-002).

use std::path::PathBuf;

/// An input file that Cipherglass can inspect. Built by discovery.
///
/// `#[non_exhaustive]` so a field for a future artifact type (REQ-030) is not
/// a breaking change; other crates build it with [`Artifact::new`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Artifact {
    /// Path relative to the discovery root. May not be valid UTF-8.
    pub path: PathBuf,
    /// Byte length from metadata at discovery time. The file can change
    /// afterwards, so a reader must bound its own reads.
    pub size: u64,
}

impl Artifact {
    #[must_use]
    pub fn new(path: PathBuf, size: u64) -> Self {
        Self { path, size }
    }
}
