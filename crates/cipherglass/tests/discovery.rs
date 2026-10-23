//! Artifact discovery behavior (F-002 T-1 to T-14, T-16, T-18).
//!
//! Each test builds its own tree at run time under CARGO_TARGET_TMPDIR.
//! Symlinks, FIFOs, permissions and non-UTF-8 names cannot be committed to
//! git reliably, so there are no fixture files.

use std::fs;
use std::path::{Path, PathBuf};

use cipherglass::{
    Artifact, Discovery, DiscoveryError, DiscoveryOptions, SkipReason, SkippedEntry, discover,
};

/// An empty directory for one test, removed first if a previous run left it.
fn fresh(test: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("discovery")
        .join(test);
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove previous test tree");
    }
    fs::create_dir_all(&root).expect("create test root");
    root
}

/// Writes `size` bytes to `root/rel`, creating parent directories.
fn file(root: &Path, rel: &str, size: usize) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().expect("file has a parent")).expect("create parent");
    fs::write(&path, vec![b'x'; size]).expect("write file");
}

fn dir(root: &Path, rel: &str) {
    fs::create_dir_all(root.join(rel)).expect("create directory");
}

fn run(root: &Path) -> Discovery {
    discover(root, &DiscoveryOptions::default()).expect("discovery succeeds")
}

fn artifact_paths(discovery: &Discovery) -> Vec<&Path> {
    discovery
        .artifacts
        .iter()
        .map(|a| a.path.as_path())
        .collect()
}

fn skipped(discovery: &Discovery) -> Vec<(&Path, &SkipReason)> {
    discovery
        .skipped
        .iter()
        .map(|s| (s.path.as_path(), &s.reason))
        .collect()
}

// T-1
#[test]
fn returns_supported_files_with_relative_paths() {
    let root = fresh("supported");
    file(&root, "a.js", 3);
    file(&root, "b.mjs", 4);
    file(&root, "c.cjs", 5);
    file(&root, ".hidden/d.js", 1);
    file(&root, "node_modules/pkg/e.js", 2);
    file(&root, "x.min.js", 0);
    dir(&root, "dir.js");

    let found = run(&root);
    assert_eq!(
        found.artifacts,
        vec![
            Artifact::new(".hidden/d.js".into(), 1),
            Artifact::new("a.js".into(), 3),
            Artifact::new("b.mjs".into(), 4),
            Artifact::new("c.cjs".into(), 5),
            Artifact::new("node_modules/pkg/e.js".into(), 2),
            Artifact::new("x.min.js".into(), 0),
        ]
    );
    assert!(found.skipped.is_empty());
}

// T-2
#[test]
fn ignores_other_extensions_without_recording_them() {
    let root = fresh("other_extensions");
    file(&root, "app.JS", 1);
    file(&root, ".js", 1);
    file(&root, "readme.md", 1);
    file(&root, "a.ts", 1);

    let found = run(&root);
    assert!(found.artifacts.is_empty());
    assert!(found.skipped.is_empty());
}

// T-5
#[test]
fn applies_size_limit_inclusively() {
    let root = fresh("size_limit");
    file(&root, "fits.js", 10);
    file(&root, "big.js", 11);
    file(&root, "big.bin", 1024 * 1024);

    let mut options = DiscoveryOptions::default();
    options.max_file_size = 10;
    let found = discover(&root, &options).expect("discovery succeeds");

    assert_eq!(artifact_paths(&found), vec![Path::new("fits.js")]);
    assert_eq!(
        skipped(&found),
        vec![(
            Path::new("big.js"),
            &SkipReason::TooLarge {
                size: 11,
                limit: 10
            }
        )]
    );
}

// T-6
#[test]
fn default_size_limit_is_32_mib() {
    assert_eq!(DiscoveryOptions::default().max_file_size, 33_554_432);
}

// T-10 (portable cases)
#[test]
fn missing_target_is_not_found() {
    let root = fresh("missing_target");
    let target = root.join("missing");
    assert_eq!(
        discover(&target, &DiscoveryOptions::default()),
        Err(DiscoveryError::NotFound { target })
    );
}

#[test]
fn file_target_is_not_a_directory() {
    let root = fresh("file_target");
    file(&root, "a.js", 1);
    let target = root.join("a.js");
    assert_eq!(
        discover(&target, &DiscoveryOptions::default()),
        Err(DiscoveryError::NotADirectory { target })
    );
}

// T-16
#[test]
fn empty_target_succeeds_with_nothing() {
    let root = fresh("empty");
    let found = run(&root);
    assert_eq!(found.root, root);
    assert!(found.artifacts.is_empty());
    assert!(found.skipped.is_empty());
}

// T-18: compiles only if `Artifact` can be built outside cipherglass-core.
#[test]
fn artifact_can_be_built_outside_core() {
    let artifact = Artifact::new("a.js".into(), 7);
    assert_eq!(artifact.path, Path::new("a.js"));
    assert_eq!(artifact.size, 7);
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::*;

    /// Removes all permissions from a directory and restores them on drop,
    /// including when the test panics, so the next run can remove the tree.
    struct Locked(PathBuf);

    impl Locked {
        fn new(path: PathBuf) -> Self {
            fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("chmod 000");
            Self(path)
        }

        /// False when locking had no effect (root, or a filesystem that
        /// ignores mode bits). std cannot query the effective uid.
        fn denies_access(&self) -> bool {
            fs::read_dir(&self.0).is_err()
        }
    }

    impl Drop for Locked {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        }
    }

    fn mkfifo(path: &Path) {
        let status = Command::new("mkfifo")
            .arg(path)
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo failed");
    }

    /// Runs `discover` on a thread and fails if it blocks.
    fn discover_with_timeout(root: &Path) -> Discovery {
        let root = root.to_path_buf();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = sender.send(discover(&root, &DiscoveryOptions::default()));
        });
        receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("discover blocked for 10 seconds")
            .expect("discovery succeeds")
    }

    // T-3
    #[test]
    fn output_is_deterministic_and_sorted() {
        let root = fresh("deterministic");
        for i in 0..50 {
            // Names that interleave across directories, so listing order is
            // unlikely to already be sorted.
            file(&root, &format!("d{}/f{:02}.js", i % 5, (i * 37) % 50), 1);
        }
        file(&root, "big.js", 200);
        symlink("d0", root.join("link")).expect("create symlink");
        mkfifo(&root.join("pipe.js"));
        file(&root, "locked/x.js", 1);
        let locked = Locked::new(root.join("locked"));

        let mut options = DiscoveryOptions::default();
        options.max_file_size = 100;
        let first = discover(&root, &options).expect("discovery succeeds");
        let second = discover(&root, &options).expect("discovery succeeds");

        assert_eq!(first, second);
        assert_eq!(first.artifacts.len(), 50);
        assert!(first.artifacts.is_sorted_by(|a, b| a.path < b.path));
        assert!(first.skipped.is_sorted_by(|a, b| a.path < b.path));
        let reasons: Vec<_> = first.skipped.iter().map(|s| &s.reason).collect();
        assert!(reasons.contains(&&SkipReason::Symlink));
        assert!(reasons.contains(&&SkipReason::NotRegularFile));
        assert!(
            reasons
                .iter()
                .any(|r| matches!(r, SkipReason::TooLarge { .. }))
        );
        if locked.denies_access() {
            assert!(
                reasons
                    .iter()
                    .any(|r| matches!(r, SkipReason::Unreadable { .. }))
            );
        }
        // Strictly sorted lists have no repeats; also none across the lists.
        let artifact_set: std::collections::BTreeSet<_> =
            first.artifacts.iter().map(|a| &a.path).collect();
        assert!(
            first
                .skipped
                .iter()
                .all(|s| !artifact_set.contains(&s.path))
        );
    }

    // T-4
    #[test]
    fn symlinks_are_recorded_not_followed() {
        let root = fresh("symlinks");
        let target = root.join("target");
        file(&target, "real.js", 1);
        file(&root, "outside/x.js", 1);
        symlink("real.js", target.join("link_file.js")).expect("symlink");
        symlink(root.join("outside"), target.join("link_dir")).expect("symlink");
        symlink("nowhere.js", target.join("dangling.js")).expect("symlink");
        symlink(".", target.join("loop")).expect("symlink");

        let found = run(&target);
        assert_eq!(artifact_paths(&found), vec![Path::new("real.js")]);
        assert_eq!(
            skipped(&found),
            vec![
                (Path::new("dangling.js"), &SkipReason::Symlink),
                (Path::new("link_dir"), &SkipReason::Symlink),
                (Path::new("link_file.js"), &SkipReason::Symlink),
                (Path::new("loop"), &SkipReason::Symlink),
            ]
        );
    }

    // T-7
    #[test]
    fn unreadable_directory_is_recorded_and_walk_continues() {
        let root = fresh("unreadable_dir");
        file(&root, "locked/hidden.js", 1);
        file(&root, "open/seen.js", 1);
        let locked = Locked::new(root.join("locked"));
        if !locked.denies_access() {
            return;
        }

        let found = run(&root);
        assert_eq!(artifact_paths(&found), vec![Path::new("open/seen.js")]);
        assert_eq!(
            skipped(&found),
            vec![(
                Path::new("locked"),
                &SkipReason::Unreadable {
                    kind: std::io::ErrorKind::PermissionDenied,
                    os_code: Some(13), // EACCES
                }
            )]
        );
    }

    // T-8
    #[test]
    fn fifo_is_recorded_without_blocking() {
        let root = fresh("fifo");
        mkfifo(&root.join("p.js"));
        mkfifo(&root.join("p.txt"));

        let found = discover_with_timeout(&root);
        assert!(found.artifacts.is_empty());
        assert_eq!(
            skipped(&found),
            vec![(Path::new("p.js"), &SkipReason::NotRegularFile)]
        );
    }

    // T-9
    #[test]
    fn unreadable_file_is_returned_because_content_is_never_opened() {
        let root = fresh("unreadable_file");
        file(&root, "secret.js", 6);
        fs::set_permissions(root.join("secret.js"), fs::Permissions::from_mode(0o000))
            .expect("chmod 000");

        let found = run(&root);
        assert_eq!(found.artifacts, vec![Artifact::new("secret.js".into(), 6)]);
    }

    // T-10 (Unix cases)
    #[test]
    fn target_link_errors() {
        let root = fresh("target_link_errors");
        file(&root, "a.js", 1);
        let dangling = root.join("dangling");
        symlink("nowhere", &dangling).expect("symlink");
        let to_file = root.join("to_file");
        symlink("a.js", &to_file).expect("symlink");

        let options = DiscoveryOptions::default();
        assert_eq!(
            discover(&dangling, &options),
            Err(DiscoveryError::NotFound { target: dangling })
        );
        assert_eq!(
            discover(&to_file, &options),
            Err(DiscoveryError::NotADirectory { target: to_file })
        );
    }

    #[test]
    fn unreadable_target_is_an_error() {
        let root = fresh("unreadable_target");
        let target = root.join("target");
        file(&target, "a.js", 1);
        let locked = Locked::new(target.clone());
        if !locked.denies_access() {
            return;
        }

        assert_eq!(
            discover(&target, &DiscoveryOptions::default()),
            Err(DiscoveryError::Unreadable {
                target: target.clone(),
                kind: std::io::ErrorKind::PermissionDenied,
                os_code: Some(13), // EACCES
            })
        );
    }

    // T-11
    #[test]
    fn target_symlink_to_directory_is_walked() {
        let root = fresh("target_symlink");
        file(&root, "real/a.js", 1);
        let link = root.join("link");
        symlink("real", &link).expect("symlink");

        let found = run(&link);
        assert_eq!(found.root, link);
        assert_eq!(artifact_paths(&found), vec![Path::new("a.js")]);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    use super::*;

    const ENAMETOOLONG: i32 = 36;
    const ELOOP: i32 = 40;

    // T-10: `ErrorKind::FilesystemLoop` is unstable, so only the code is
    // checked.
    #[test]
    fn self_referencing_target_link_is_unreadable() {
        let root = fresh("target_loop");
        let target = root.join("loop");
        symlink("loop", &target).expect("symlink");

        let error = discover(&target, &DiscoveryOptions::default()).expect_err("loop fails");
        assert!(
            matches!(
                &error,
                DiscoveryError::Unreadable { target: t, os_code: Some(ELOOP), .. } if *t == target
            ),
            "unexpected error: {error:?}"
        );
    }

    // T-12
    #[test]
    fn non_utf8_name_is_preserved() {
        let root = fresh("non_utf8");
        let name = OsStr::from_bytes(b"bad\xff.js");
        fs::write(root.join(name), b"x").expect("write file");

        let found = run(&root);
        assert_eq!(found.artifacts.len(), 1);
        assert_eq!(
            found.artifacts[0].path.as_os_str().as_bytes(),
            b"bad\xff.js"
        );
    }

    /// Removes a tree one entry at a time by full path. `fs::remove_dir_all`
    /// keeps one descriptor open per level and fails with EMFILE on a deep
    /// chain under a 1,024 open-file limit (F-002 contract M-5).
    fn remove_deep(root: &Path) {
        if !root.exists() {
            return;
        }
        let mut dirs = vec![root.to_path_buf()];
        let mut index = 0;
        while index < dirs.len() {
            for entry in fs::read_dir(&dirs[index]).expect("list") {
                let entry = entry.expect("entry");
                if entry.file_type().expect("type").is_dir() {
                    dirs.push(entry.path());
                } else {
                    fs::remove_file(entry.path()).expect("remove file");
                }
            }
            index += 1;
        }
        for dir in dirs.iter().rev() {
            fs::remove_dir(dir).expect("remove dir");
        }
    }

    // T-13
    #[test]
    fn deep_chain_is_walked() {
        const DEPTH: usize = 1_900;
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("discovery")
            .join("deep_chain");
        assert!(
            root.as_os_str().len() < 250,
            "checkout path too long for this test: {} bytes",
            root.as_os_str().len()
        );
        remove_deep(&root);
        fs::create_dir_all(&root).expect("create test root");

        let mut path = root.clone();
        for _ in 0..DEPTH {
            path.push("d");
            fs::create_dir(&path).expect("create level");
        }
        fs::write(path.join("deep.js"), b"x").expect("write file");

        let found = run(&root);
        remove_deep(&root);

        let expected: PathBuf = std::iter::repeat_n("d", DEPTH).chain(["deep.js"]).collect();
        assert_eq!(artifact_paths(&found), vec![expected.as_path()]);
        assert!(found.skipped.is_empty());
    }

    // T-14
    #[test]
    fn directory_beyond_path_limit_is_unreadable() {
        const PATH_MAX: usize = 4_096;
        // Enough one-byte levels that the levels still below each rename
        // (2 bytes each) leave room for the next 256-byte step.
        const LEVELS: usize = 200;
        let root = fresh("path_limit");
        file(&root, "side/ok.js", 1);

        let mut bottom = root.join("chain");
        for _ in 0..LEVELS {
            bottom.push("a");
        }
        fs::create_dir_all(&bottom).expect("create chain");
        fs::write(bottom.join("deep.js"), b"x").expect("write file");

        // Lengthen the chain by renaming its top levels to 255-byte names,
        // using paths under the limit, until the bottom is too long.
        let long = "l".repeat(255);
        let mut parent = root.join("chain");
        let mut renamed = 0;
        while parent.as_os_str().len() + 2 * (LEVELS - renamed) < PATH_MAX {
            let next = parent.join(&long);
            fs::rename(parent.join("a"), &next).expect("rename level");
            parent = next;
            renamed += 1;
        }

        let found = run(&root);
        assert_eq!(artifact_paths(&found), vec![Path::new("side/ok.js")]);
        assert_eq!(found.skipped.len(), 1, "skipped: {:?}", found.skipped);
        let SkippedEntry { path, reason, .. } = &found.skipped[0];
        assert!(path.starts_with("chain"));
        assert!(root.join(path).as_os_str().len() >= PATH_MAX);
        assert!(
            root.join(path)
                .parent()
                .is_some_and(|p| p.as_os_str().len() < PATH_MAX)
        );
        assert_eq!(
            reason,
            &SkipReason::Unreadable {
                kind: std::io::ErrorKind::InvalidFilename,
                os_code: Some(ENAMETOOLONG),
            }
        );
    }
}
