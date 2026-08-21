use std::process::Command;

fn run_ssharp(input: &str) -> String {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "examples/access_control.ssharp"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run");

    let mut child = output;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn run_ssharp_cli(args: &[&str]) -> (String, String, i32) {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run");

    let output = output.wait_with_output().expect("Failed to wait on child");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, stderr, code)
}

#[test]
fn test_access_granted() {
    let output = run_ssharp("25\n");
    assert!(output.contains("Access granted"), "Expected 'Access granted' in output: {}", output);
    assert!(!output.contains("Access denied"), "Should not contain 'Access denied': {}", output);
}

#[test]
fn test_access_denied() {
    let output = run_ssharp("15\n");
    assert!(output.contains("Access denied"), "Expected 'Access denied' in output: {}", output);
    assert!(!output.contains("Access granted"), "Should not contain 'Access granted': {}", output);
}

#[test]
fn test_version_flag() {
    let (stdout, stderr, code) = run_ssharp_cli(&["--version"]);
    assert_eq!(code, 0, "Expected exit code 0 for --version");
    assert!(stdout.contains("S# (ssharp) v"), "Expected version in stdout: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for --version");
}

#[test]
fn test_version_short_flag() {
    let (stdout, stderr, code) = run_ssharp_cli(&["-v"]);
    assert_eq!(code, 0, "Expected exit code 0 for -v");
    assert!(stdout.contains("S# (ssharp) v"), "Expected version in stdout: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for -v");
}

#[test]
fn test_help_flag() {
    let (stdout, stderr, code) = run_ssharp_cli(&["--help"]);
    assert_eq!(code, 0, "Expected exit code 0 for --help");
    assert!(stdout.contains("Usage:"), "Expected usage in stdout: {}", stdout);
    assert!(stdout.contains("--help"), "Expected --help in usage: {}", stdout);
    assert!(stdout.contains("--version"), "Expected --version in usage: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for --help");
}

#[test]
fn test_help_short_flag() {
    let (stdout, stderr, code) = run_ssharp_cli(&["-h"]);
    assert_eq!(code, 0, "Expected exit code 0 for -h");
    assert!(stdout.contains("Usage:"), "Expected usage in stdout: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for -h");
}

#[test]
fn test_no_args() {
    let (stdout, stderr, code) = run_ssharp_cli(&[]);
    assert_eq!(code, 1, "Expected exit code 1 for no arguments");
    assert!(stdout.contains("Usage:"), "Expected usage in stdout: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for no args");
}