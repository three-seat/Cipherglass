//! CLI skeleton behavior (F-001 T-6 to T-9).

use std::process::{Command, Output, Stdio};

fn cipherglass(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_cipherglass"))
        .args(args)
        .output()
        .expect("failed to run cipherglass binary")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output is not UTF-8")
}

#[test]
fn version_prints_name_and_version_to_stdout() {
    let out = cipherglass(&["--version"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(text(&out.stdout), "cipherglass 0.1.0\n");
    assert!(out.stderr.is_empty());
}

#[test]
fn help_prints_usage_to_stdout() {
    let out = cipherglass(&["--help"]);
    assert_eq!(out.status.code(), Some(0));
    let stdout = text(&out.stdout);
    let first_line = stdout.lines().next().unwrap_or_default();
    assert_eq!(
        first_line,
        "cipherglass - Find useful security info in deployed client files"
    );
    assert!(stdout.contains("Usage: cipherglass"));
    assert!(out.stderr.is_empty());
}

#[test]
fn bad_usage_exits_1_with_usage_on_stderr() {
    for args in [&[][..], &["--bogus"][..], &["bogus"][..]] {
        let out = cipherglass(args);
        assert_eq!(out.status.code(), Some(1), "args: {args:?}");
        assert!(out.stdout.is_empty(), "args: {args:?}");
        assert!(
            text(&out.stderr).contains("Usage: cipherglass"),
            "args: {args:?}"
        );
    }
}

#[test]
fn closed_stdout_exits_2_without_panicking() {
    // Drop the read end before spawning so the child's first write fails
    // deterministically, regardless of pipe buffer size.
    let (reader, writer) = std::io::pipe().expect("failed to create pipe");
    drop(reader);
    let out = Command::new(env!("CARGO_BIN_EXE_cipherglass"))
        .arg("--help")
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run cipherglass binary");
    assert_eq!(out.status.code(), Some(2));
    assert!(!text(&out.stderr).contains("panicked"));
}
