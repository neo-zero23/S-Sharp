use std::process::Command;

fn run_ssharp(input: &str) -> String {
    run_ssharp_file("examples/access_control.ssharp", input)
}

fn run_ssharp_file(path: &str, input: &str) -> String {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", path])
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
    run_ssharp_cli_with_input(args, "")
}

fn run_ssharp_cli_with_input(args: &[&str], stdin_data: &str) -> (String, String, i32) {
    let mut child = Command::new("cargo")
        .args(["run", "--quiet", "--"])
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run");

    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(stdin_data.as_bytes()).expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait on child");
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
fn test_no_args_starts_repl() {
    // No file argument starts the REPL; empty stdin (EOF) exits cleanly.
    let (stdout, stderr, code) = run_ssharp_cli_with_input(&[], "");
    assert_eq!(code, 0, "Expected exit code 0 for REPL on EOF");
    assert!(stdout.contains("interactive mode"), "Expected REPL banner in stdout: {}", stdout);
    assert!(stdout.contains("Bye."), "Expected REPL goodbye in stdout: {}", stdout);
    assert!(stderr.is_empty(), "Expected empty stderr for REPL on EOF");
}

#[test]
fn test_repl_runs_fragments() {
    let (stdout, _, code) = run_ssharp_cli_with_input(
        &[],
        "display 1 ++ 2.\nif (1 != 2), display \"ne-ok\".\n:quit\n",
    );
    assert_eq!(code, 0, "Expected exit code 0 for REPL session");
    assert!(stdout.contains("12"), "Expected concat result in stdout: {}", stdout);
    assert!(stdout.contains("ne-ok"), "Expected condition result in stdout: {}", stdout);
}

#[test]
fn test_operators_example() {
    let output = run_ssharp_file("examples/operators.ssharp", "");
    assert!(output.contains("hello, Ada!"), "Expected greeting in output: {}", output);
    assert!(output.contains("equality still works"), "Expected else branch in output: {}", output);
    assert!(output.contains("12\n"), "Expected '12' concat in output: {}", output);
    assert!(output.contains("3\n"), "Expected '3' addition in output: {}", output);
    assert!(output.contains("numbers can differ"), "Expected != result in output: {}", output);
    assert!(!output.contains("broken"), "Should not contain 'broken': {}", output);
}