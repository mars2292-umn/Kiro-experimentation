use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper: run the binary with given args from a temp directory.
/// Returns (exit_status, stdout, stderr, output_txt_contents).
fn run_in_tempdir(args: &[&str]) -> (std::process::ExitStatus, String, String, Option<String>) {
    let dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("multithreaded-calculator").unwrap();
    cmd.current_dir(dir.path());
    for arg in args {
        cmd.arg(arg);
    }

    let output = cmd.output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let output_txt = fs::read_to_string(dir.path().join("output.txt")).ok();

    (output.status, stdout, stderr, output_txt)
}

// --- Test 1: CLI string input end-to-end ---
// Requirements: 1.3, 7.1, 7.2, 7.3
#[test]
fn test_cli_string_input_end_to_end() {
    let (status, _stdout, _stderr, output_txt) = run_in_tempdir(&["1 + 2, 3 * 4"]);

    assert!(status.success(), "expected exit 0, got: {:?}", status);

    let content = output_txt.expect("output.txt should have been created");
    assert!(
        content.contains("1 + 2 = 3"),
        "output.txt should contain '1 + 2 = 3', got:\n{}",
        content
    );
    assert!(
        content.contains("3 * 4 = 12"),
        "output.txt should contain '3 * 4 = 12', got:\n{}",
        content
    );
}

// --- Test 2: File input end-to-end ---
// Requirements: 2.2, 7.1, 7.2, 7.3, 7.4
#[test]
fn test_file_input_end_to_end() {
    let dir = TempDir::new().unwrap();
    let input_file = dir.path().join("equations.txt");
    fs::write(&input_file, "10 - 3, 6 / 2").unwrap();

    let mut cmd = Command::cargo_bin("multithreaded-calculator").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg(input_file.to_str().unwrap());

    let output = cmd.output().unwrap();
    let status = output.status;
    let output_txt = fs::read_to_string(dir.path().join("output.txt")).ok();

    assert!(status.success(), "expected exit 0, got: {:?}", status);

    let content = output_txt.expect("output.txt should have been created");
    assert!(
        content.contains("10 - 3 = 7"),
        "output.txt should contain '10 - 3 = 7', got:\n{}",
        content
    );
    assert!(
        content.contains("6 / 2 = 3"),
        "output.txt should contain '6 / 2 = 3', got:\n{}",
        content
    );
}

// --- Test 3: Missing argument exits non-zero with usage message in stderr ---
// Requirements: 1.3
#[test]
fn test_no_args_exits_nonzero_with_usage_message() {
    let (status, _stdout, stderr, _output_txt) = run_in_tempdir(&[]);

    assert!(
        !status.success(),
        "expected non-zero exit when no args provided"
    );
    assert!(
        !stderr.is_empty(),
        "expected a message on stderr, got nothing"
    );
    // The usage/error message should mention usage or how to use the tool
    let stderr_lower = stderr.to_lowercase();
    assert!(
        stderr_lower.contains("usage") || stderr_lower.contains("error") || stderr_lower.contains("input"),
        "stderr should contain a descriptive message, got: {}",
        stderr
    );
}

// --- Test 4: All-invalid equations input exits non-zero ---
// Requirements: 1.3
#[test]
fn test_all_invalid_equations_exits_nonzero() {
    let (status, _stdout, stderr, _output_txt) = run_in_tempdir(&["foo ^ bar"]);

    assert!(
        !status.success(),
        "expected non-zero exit when all equations are invalid"
    );
    assert!(
        !stderr.is_empty(),
        "expected an error message on stderr, got nothing"
    );
}
