//! Integration tests: spawn the built `gafira-cli` binary.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gafira-cli"))
        .args(args)
        .output()
        .expect("failed to spawn gafira-cli")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("output is valid UTF-8")
}

#[test]
fn version_prints_workspace_version() {
    let out = run(&["--version"]);
    assert_eq!(
        text(&out.stdout),
        format!("gafira-cli {}\n", env!("CARGO_PKG_VERSION"))
    );
    assert!(out.stderr.is_empty());
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn version_tracks_workspace_manifest() {
    let manifest = include_str!("../../../Cargo.toml");
    let declared = manifest
        .lines()
        .find_map(|line| line.strip_prefix("version = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .expect("workspace version line");
    let out = run(&["--version"]);
    assert_eq!(text(&out.stdout), format!("gafira-cli {declared}\n"));
}

#[test]
fn help_prints_usage_to_stdout() {
    let out = run(&["--help"]);
    let stdout = text(&out.stdout);
    assert!(stdout.contains("Usage: gafira-cli"));
    assert!(stdout.contains("--version"));
    assert!(stdout.contains("--help"));
    assert!(out.stderr.is_empty());
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn unknown_flag_exits_2_naming_the_argument() {
    let out = run(&["--bogus"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("--bogus"));
    assert!(out.stdout.is_empty());
}

#[test]
fn no_arguments_exits_2_with_usage() {
    let out = run(&[]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("Usage: gafira-cli"));
    assert!(out.stdout.is_empty());
}

#[test]
fn extra_arguments_are_rejected() {
    let out = run(&["--version", "extra"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("extra"));
    assert!(out.stdout.is_empty());
}
