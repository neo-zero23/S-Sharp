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
fn test_lists_example() {
    let output = run_ssharp_file("examples/lists.ssharp", "");
    assert!(output.contains("[Ada, Grace, Linus]"), "Expected list display in output: {}", output);
    assert!(output.contains("3\n"), "Expected len 3 in output: {}", output);
    assert!(output.contains("Ada\n"), "Expected first item in output: {}", output);
    assert!(output.contains("Linus\n"), "Expected last item in output: {}", output);
    assert!(output.contains("20\n"), "Expected item 2 of scores in output: {}", output);
    assert!(output.contains("\nh\n") || output.ends_with("h\n"), "Expected 'h' from string index in output: {}", output);
}

#[test]
fn test_fizzbuzz_example() {
    let output = run_ssharp_file("examples/fizzbuzz.ssharp", "");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines, vec!["1", "2", "Fizz", "4", "Buzz", "Fizz", "7", "8", "Fizz", "Buzz", "11", "Fizz", "13", "14", "FizzBuzz"]);
}

#[test]
fn test_tanda1_example() {
    let output = run_ssharp_file("examples/tanda1.ssharp", "");
    assert!(output.contains("[10, 3, 4]"), "Expected mutated list in output: {}", output);
    assert!(output.contains("17\n"), "Expected sum(xs)=17 in output: {}", output);
    assert!(output.contains("15\n"), "Expected sum(range(5))=15 in output: {}", output);
    assert!(output.contains("2 is even"), "Expected even line in output: {}", output);
    assert!(output.contains("5 is odd"), "Expected odd line in output: {}", output);
}

#[test]
fn test_fmt_stdout() {
    let (stdout, _, code) = run_ssharp_cli(&["fmt", "examples/function_test.ssharp"]);
    assert_eq!(code, 0, "Expected exit code 0 for fmt");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, vec!["when (test).", "define function plus(a, b), return a + b.", "save plus(5, 3) to result.", "display result."]);
}

#[test]
fn test_fmt_write_and_reject() {
    // --write on a temp copy must be idempotent and runnable afterwards.
    let dir = std::env::temp_dir();
    let path = dir.join(format!("ssharp-fmt-{}.ssharp", std::process::id()));
    std::fs::write(&path, "when (test).   save 1 to x.\n\n display x.").unwrap();
    let path_str = path.to_string_lossy().to_string();

    let (_, _, code) = run_ssharp_cli(&["fmt", "--write", &path_str]);
    assert_eq!(code, 0, "Expected exit code 0 for fmt --write");
    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(after, "when (test).\nsave 1 to x.\ndisplay x.\n");

    let (_, _, code) = run_ssharp_cli(&[&path_str]);
    assert_eq!(code, 0, "Formatted file must still run");
    std::fs::remove_file(&path).ok();

    // Broken code is rejected, not reformatted.
    let bad = dir.join(format!("ssharp-fmt-bad-{}.ssharp", std::process::id()));
    std::fs::write(&bad, "when (test). display .").unwrap();
    let (_, _, code) = run_ssharp_cli(&["fmt", bad.to_str().unwrap()]);
    assert_ne!(code, 0, "fmt must fail on broken code");
    std::fs::remove_file(&bad).ok();
}

#[test]
fn test_break_demo_example() {
    let output = run_ssharp_file("examples/break_demo.ssharp", "");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines, vec!["1", "2", "4", "5", "6", "7", "8", "10", "20", "30", "done"]);
}

#[test]
fn test_ints_example() {
    let output = run_ssharp_file("examples/ints.ssharp", "");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines, vec!["int", "float", "int", "float", "3.5", "3", "-3", "30", "13"]);
}

#[test]
fn test_use_import_example() {
    // Relative import resolves against the importing file's directory.
    let output = run_ssharp_file("examples/use_import.ssharp", "");
    assert!(output.contains("HELLO!"), "Expected shout in output: {}", output);
    assert!(output.contains("[a, b, c]"), "Expected csv in output: {}", output);
    assert!(output.contains("AL"), "Expected initials in output: {}", output);
    assert!(output.contains("string\n"), "Expected type() in output: {}", output);
}

#[test]
fn test_try_demo_example() {
    let output = run_ssharp_file("examples/try_demo.ssharp", "");
    assert!(output.contains("fell back"), "Expected fallback in output: {}", output);
    assert!(!output.contains("should not print"), "Catch must not run without error: {}", output);
    assert!(output.contains("read failed"), "Expected read failure caught: {}", output);
    assert!(output.contains("Division by zero"), "Expected div-by-zero message: {}", output);
    assert!(output.contains("still running"), "Expected program to continue: {}", output);
}

#[test]
fn test_file_demo_example() {    let output = run_ssharp_file("examples/file_demo.ssharp", "");
    assert!(output.contains("line1\nline2"), "Expected roundtrip in output: {}", output);
    assert!(output.contains("2\n"), "Expected split count in output: {}", output);
    // The demo writes next to itself; clean up after the run.
    let _ = std::fs::remove_file(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/file_demo_out.txt"));
}

#[test]
fn test_operators_example() {    let output = run_ssharp_file("examples/operators.ssharp", "");
    assert!(output.contains("hello, Ada!"), "Expected greeting in output: {}", output);
    assert!(output.contains("equality still works"), "Expected else branch in output: {}", output);
    assert!(output.contains("12\n"), "Expected '12' concat in output: {}", output);
    assert!(output.contains("3\n"), "Expected '3' addition in output: {}", output);
    assert!(output.contains("numbers can differ"), "Expected != result in output: {}", output);
    assert!(!output.contains("broken"), "Should not contain 'broken': {}", output);
}