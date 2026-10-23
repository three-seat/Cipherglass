//! Artifact discovery (F-002; REQ-010, REQ-011, REQ-022).
//!
//! Walks a target directory using metadata only: no file is opened and no
//! link below the target is followed. Every entry that matters but cannot be
//! returned is recorded with a reason. Both output lists are sorted by path
//! (REQ-004, REQ-005).

use std::ffi::OsString;
use std::fmt;
use std::fs::{self, DirEntry};
use std::io;
use std::path::{Path, PathBuf};

use cipherglass_core::Artifact;

/// Default `max_file_size`: 32 MiB (F-002 Q3).
const DEFAULT_MAX_FILE_SIZE: u64 = 32 * 1024 * 1024;

/// Extensions of supported JavaScript files, matched exactly (REQ-010).
const EXTENSIONS: &[&str] = &["js", "mjs", "cjs"];

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiscoveryOptions {
    /// Matching files larger than this many bytes are skipped as `TooLarge`.
    pub max_file_size: u64,
}

impl Default for DiscoveryOptions {
    fn default() -> Self {
        Self {
            max_file_size: DEFAULT_MAX_FILE_SIZE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Discovery {
    /// The target as given.
    pub root: PathBuf,
    /// Sorted by path.
    pub artifacts: Vec<Artifact>,
    /// Sorted by path.
    pub skipped: Vec<SkippedEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedEntry {
    /// Relative to the target. Empty means the target itself.
    pub path: PathBuf,
    pub reason: SkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipReason {
    /// A symlink, which is never followed.
    Symlink,
    /// A matching file larger than `max_file_size`.
    TooLarge { size: u64, limit: u64 },
    /// A matching entry that is not a regular file (FIFO, socket, device).
    NotRegularFile,
    /// A directory that could not be listed, or an entry whose type or
    /// metadata could not be read.
    Unreadable {
        kind: io::ErrorKind,
        os_code: Option<i32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DiscoveryError {
    NotFound {
        target: PathBuf,
    },
    NotADirectory {
        target: PathBuf,
    },
    Unreadable {
        target: PathBuf,
        kind: io::ErrorKind,
        os_code: Option<i32>,
    },
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { target } => {
                write!(f, "target {} does not exist", target.display())
            }
            Self::NotADirectory { target } => {
                write!(f, "target {} is not a directory", target.display())
            }
            Self::Unreadable {
                target,
                kind,
                os_code,
            } => {
                write!(f, "cannot read target {}: {kind}", target.display())?;
                if let Some(code) = os_code {
                    write!(f, " (os error {code})")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Finds supported JavaScript files under `target`.
///
/// `target` must be a directory; a link to one is followed (F-002 Q10). Links
/// below it are recorded, never followed. Only a failure on the target itself
/// is an error; any other failure is recorded in `skipped`.
///
/// # Errors
///
/// `NotFound` if the target (or its link target) does not exist,
/// `NotADirectory` if it is not a directory, and `Unreadable` if its metadata
/// or listing cannot be read.
pub fn discover(target: &Path, options: &DiscoveryOptions) -> Result<Discovery, DiscoveryError> {
    let unreadable = |e: &io::Error| DiscoveryError::Unreadable {
        target: target.to_path_buf(),
        kind: e.kind(),
        os_code: e.raw_os_error(),
    };
    let metadata = fs::metadata(target).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => DiscoveryError::NotFound {
            target: target.to_path_buf(),
        },
        _ => unreadable(&e),
    })?;
    if !metadata.is_dir() {
        return Err(DiscoveryError::NotADirectory {
            target: target.to_path_buf(),
        });
    }
    let listing = fs::read_dir(target).map_err(|e| unreadable(&e))?;

    let mut walk = Walk::new(options.max_file_size);
    walk.list(PathBuf::new(), listing);
    // Iterative, so tree depth is bounded by the OS path limit, not the stack.
    while let Some(dir) = walk.pending.pop() {
        match fs::read_dir(target.join(&dir)) {
            Ok(listing) => walk.list(dir, listing),
            Err(e) => walk.skip(dir, unreadable_reason(&e)),
        }
    }

    walk.artifacts.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    walk.skipped.sort_unstable_by(|a, b| a.path.cmp(&b.path));
    Ok(Discovery {
        root: target.to_path_buf(),
        artifacts: walk.artifacts,
        skipped: walk.skipped,
    })
}

fn unreadable_reason(e: &io::Error) -> SkipReason {
    SkipReason::Unreadable {
        kind: e.kind(),
        os_code: e.raw_os_error(),
    }
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| EXTENSIONS.iter().any(|&supported| ext == supported))
}

/// Entry type as seen without following links.
#[derive(Debug, Clone, Copy)]
enum EntryType {
    Directory,
    Symlink,
    File,
    Other,
}

/// One directory entry. `DirEntry` in real walks; a fake in unit tests, since
/// a real filesystem cannot be made to fail mid-listing on demand.
trait Entry {
    fn name(&self) -> OsString;
    fn entry_type(&self) -> io::Result<EntryType>;
    fn size(&self) -> io::Result<u64>;
}

impl Entry for DirEntry {
    fn name(&self) -> OsString {
        self.file_name()
    }

    // `DirEntry::file_type` and `DirEntry::metadata` do not follow symlinks.
    fn entry_type(&self) -> io::Result<EntryType> {
        let file_type = self.file_type()?;
        Ok(if file_type.is_symlink() {
            EntryType::Symlink
        } else if file_type.is_dir() {
            EntryType::Directory
        } else if file_type.is_file() {
            EntryType::File
        } else {
            EntryType::Other
        })
    }

    fn size(&self) -> io::Result<u64> {
        Ok(self.metadata()?.len())
    }
}

struct Walk {
    max_file_size: u64,
    /// Directories still to list, relative to the target.
    pending: Vec<PathBuf>,
    artifacts: Vec<Artifact>,
    skipped: Vec<SkippedEntry>,
}

impl Walk {
    fn new(max_file_size: u64) -> Self {
        Self {
            max_file_size,
            pending: Vec::new(),
            artifacts: Vec::new(),
            skipped: Vec::new(),
        }
    }

    fn skip(&mut self, path: PathBuf, reason: SkipReason) {
        self.skipped.push(SkippedEntry { path, reason });
    }

    /// Classifies the entries of `dir` (relative to the target). At the first
    /// listing error, keeps what was seen, records `dir` once as
    /// `Unreadable`, and stops listing it.
    fn list<E: Entry>(&mut self, dir: PathBuf, entries: impl IntoIterator<Item = io::Result<E>>) {
        for entry in entries {
            match entry {
                Ok(entry) => self.classify(&dir, &entry),
                Err(e) => {
                    self.skip(dir, unreadable_reason(&e));
                    return;
                }
            }
        }
    }

    fn classify(&mut self, dir: &Path, entry: &impl Entry) {
        let path = dir.join(entry.name());
        let entry_type = match entry.entry_type() {
            Ok(entry_type) => entry_type,
            Err(e) => return self.skip(path, unreadable_reason(&e)),
        };
        match entry_type {
            EntryType::Directory => self.pending.push(path),
            EntryType::Symlink => self.skip(path, SkipReason::Symlink),
            EntryType::File if is_supported(&path) => match entry.size() {
                Ok(size) if size <= self.max_file_size => {
                    self.artifacts.push(Artifact::new(path, size));
                }
                Ok(size) => {
                    let limit = self.max_file_size;
                    self.skip(path, SkipReason::TooLarge { size, limit });
                }
                Err(e) => self.skip(path, unreadable_reason(&e)),
            },
            EntryType::Other if is_supported(&path) => {
                self.skip(path, SkipReason::NotRegularFile);
            }
            EntryType::File | EntryType::Other => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeEntry {
        name: &'static str,
        entry_type: EntryType,
        size: u64,
    }

    impl Entry for FakeEntry {
        fn name(&self) -> OsString {
            self.name.into()
        }

        fn entry_type(&self) -> io::Result<EntryType> {
            Ok(self.entry_type)
        }

        fn size(&self) -> io::Result<u64> {
            Ok(self.size)
        }
    }

    fn file(name: &'static str) -> io::Result<FakeEntry> {
        Ok(FakeEntry {
            name,
            entry_type: EntryType::File,
            size: 1,
        })
    }

    fn listing_error() -> io::Result<FakeEntry> {
        Err(io::Error::from_raw_os_error(5)) // EIO
    }

    // F-002 T-15.
    #[test]
    fn listing_error_keeps_earlier_entries_and_records_directory_once() {
        let mut walk = Walk::new(100);
        let entries = vec![file("before.js"), listing_error(), file("after.js")];
        walk.list(PathBuf::from("dir"), entries);

        let paths: Vec<_> = walk.artifacts.iter().map(|a| a.path.clone()).collect();
        assert_eq!(paths, vec![PathBuf::from("dir/before.js")]);
        assert_eq!(
            walk.skipped,
            vec![SkippedEntry {
                path: PathBuf::from("dir"),
                reason: unreadable_reason(&io::Error::from_raw_os_error(5)),
            }]
        );
    }

    #[test]
    fn listing_error_in_target_records_empty_path() {
        let mut walk = Walk::new(100);
        walk.list(PathBuf::new(), vec![listing_error()]);

        assert_eq!(walk.skipped.len(), 1);
        assert_eq!(walk.skipped[0].path, PathBuf::new());
        assert!(matches!(
            walk.skipped[0].reason,
            SkipReason::Unreadable {
                os_code: Some(5),
                ..
            }
        ));
    }
}
