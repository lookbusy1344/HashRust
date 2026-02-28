use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

/// Helper function to create a temporary file with content
fn create_temp_file(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    temp_file
        .write_all(content.as_bytes())
        .expect("Failed to write to temp file");
    temp_file.flush().expect("Failed to flush temp file");
    temp_file
}

#[test]
fn test_sha3_256_hash() {
    let test_content = "Hello, World!";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    // Run the hash_rust binary with SHA3-256 (default)
    let output = Command::new("cargo")
        .args(["run", "--", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    // Check output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // SHA3-256 hash of "Hello, World!" should be consistent
    assert!(stdout.contains("1af17a664e3fa8e419b8ba05c2a173169df76162a5a286e0c405b460d478f7ef"));
    assert!(stdout.contains(test_path.file_name().unwrap().to_str().unwrap()));
}

#[test]
fn test_md5_hash() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(["run", "--", "-a", "MD5", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // MD5 hash of "test"
    assert!(stdout.contains("098f6bcd4621d373cade4e832627b4f6"));
}

#[test]
fn test_crc32_hash() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(["run", "--", "-a", "CRC32", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // CRC32 output should be 10-digit zero-padded
    assert!(stdout.contains("3632233996"));
}

// SHA3-256 hex output is 64 characters; the line must be at least that long.
const MIN_HASH_LENGTH: usize = 64;

#[test]
fn test_exclude_filenames() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(["run", "--", "-x", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain filename when -x flag is used
    assert!(!stdout.contains(test_path.file_name().unwrap().to_str().unwrap()));
    // But should contain the hash
    assert!(stdout.len() >= MIN_HASH_LENGTH);
}

#[test]
fn test_base64_encoding() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(["run", "--", "-e", "Base64", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // SHA3-256 of "test" in Base64
    assert!(stdout.contains("NvAoWAuwLMgnKpoCD0IA40bidq5mTkXugHRVdOL1q4A="));
}

// Error condition tests
#[test]
fn test_nonexistent_file_error() {
    let temp_dir = std::env::temp_dir();
    let nonexistent_file = temp_dir.join("nonexistent_file.txt");

    let output = Command::new("cargo")
        .args(["run", "--", nonexistent_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about file not found or access denied
    assert!(
        stderr.to_lowercase().contains("error") || stderr.to_lowercase().contains("no such file")
    );
}

#[test]
fn test_invalid_algorithm_error() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "-a",
            "INVALID_ALGORITHM",
            test_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about invalid algorithm
    assert!(stderr.to_lowercase().contains("error"));
}

#[test]
fn test_invalid_encoding_error() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "-e",
            "INVALID_ENCODING",
            test_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about invalid encoding
    assert!(stderr.to_lowercase().contains("error"));
}

#[test]
fn test_crc32_with_invalid_encoding_error() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    // CRC32 should only work with Hex encoding (U32 format)
    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "-a",
            "CRC32",
            "-e",
            "Base64",
            test_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about incompatible algorithm/encoding combination
    assert!(stderr.to_lowercase().contains("error"));
}

#[test]
fn test_u32_encoding_with_non_crc32_error() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "-a",
            "MD5",
            "-e",
            "U32",
            test_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about incompatible algorithm/encoding combination
    assert!(stderr.to_lowercase().contains("error"));
    assert!(stderr.contains("CRC32 must use U32 encoding"));
}

#[test]
fn test_empty_file_path_error() {
    // Test with no file arguments
    let output = Command::new("cargo")
        .args(["run", "--"])
        .output()
        .expect("Failed to execute hash_rust");

    // Should fail with non-zero exit code or show help
    // This might succeed if it reads from stdin, so we check for either error or help output
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.to_lowercase().contains("error") || stderr.to_lowercase().contains("usage"));
    }
}

#[test]
fn test_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute hash_rust");

    // Should succeed and show help
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain help information
    assert!(stdout.to_lowercase().contains("usage") || stdout.to_lowercase().contains("help"));
}

#[test]
fn test_multi_file_parallel_hashing() {
    // Create multiple temp files to exercise parallel path
    let files: Vec<_> = (0..5)
        .map(|i| {
            let content = format!("test content {i}");
            create_temp_file(&content)
        })
        .collect();

    // Build args with all file paths
    let mut args = vec!["run", "--"];
    let paths: Vec<_> = files.iter().map(|f| f.path().to_str().unwrap()).collect();
    args.extend(&paths);

    let output = Command::new("cargo")
        .args(&args)
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify we got output for all 5 files
    let lines: Vec<_> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 5, "Should have output for all 5 files");

    // Verify each line starts with a 64-char SHA3-256 hex hash followed by a space and path.
    // Using splitn(2) rather than split_whitespace so paths containing spaces are handled correctly.
    for (i, line) in lines.iter().enumerate() {
        let mut parts = line.splitn(2, ' ');
        let hash = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        assert_eq!(
            hash.len(),
            64,
            "Line {i} hash should be 64 hex chars, got: {hash}"
        );
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "Line {i} hash should be hex, got: {hash}"
        );
        assert!(!path.is_empty(), "Line {i} should include a file path");
    }
}

#[test]
fn test_stdin_file_paths() {
    // Create test files
    let file1 = create_temp_file("content1");
    let file2 = create_temp_file("content2");
    let file3 = create_temp_file("content3");

    // Prepare stdin with file paths (one per line)
    let stdin_input = format!(
        "{}\n{}\n{}",
        file1.path().display(),
        file2.path().display(),
        file3.path().display()
    );

    // Run with paths from stdin
    let mut child = Command::new("cargo")
        .args(["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn hash_rust");

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait on hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have output for all 3 files
    let lines: Vec<_> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        lines.len(),
        3,
        "Should have output for all 3 files from stdin"
    );

    // Verify each output line starts with a hash followed by a path
    for line in lines {
        assert!(
            line.splitn(2, ' ').count() == 2,
            "Each line should have hash and path"
        );
    }
}

#[test]
fn test_stdin_with_nonexistent_paths() {
    // Mix valid and invalid paths
    let valid_file = create_temp_file("valid content");
    let stdin_input = format!(
        "{}\n/nonexistent/file1.txt\n/nonexistent/file2.txt",
        valid_file.path().display()
    );

    let mut child = Command::new("cargo")
        .args(["run", "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn hash_rust");

    // Write to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(stdin_input.as_bytes())
            .expect("Failed to write to stdin");
    }

    let output = child
        .wait_with_output()
        .expect("Failed to wait on hash_rust");

    // Should succeed but only hash the valid file
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should have output for only 1 valid file
    let lines: Vec<_> = stdout.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), 1, "Should only hash the valid file");
}

#[test]
#[cfg(unix)]
fn test_unreadable_file_exits_nonzero() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let temp_file = create_temp_file("content");
    let path = temp_file.path().to_path_buf();

    // Remove all permissions so the file exists but cannot be read
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000))
        .expect("Failed to set permissions");

    let output = Command::new("cargo")
        .args(["run", "--", path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    // Restore permissions so tempfile cleanup can remove the file
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))
        .expect("Failed to restore permissions");

    assert!(
        !output.status.success(),
        "Should exit non-zero when a file cannot be read"
    );
}

#[test]
fn test_error_help_goes_to_stderr_not_stdout() {
    // When a configuration error occurs (invalid algorithm), help text must go to
    // stderr so that stdout stays clean for piped consumers.
    let temp_file = create_temp_file("test");

    let output = Command::new("cargo")
        .args([
            "run",
            "--",
            "-a",
            "INVALID_ALGORITHM",
            temp_file.path().to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(!output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.contains("USAGE"),
        "Help text must not appear on stdout, got: {stdout}"
    );
    assert!(
        stderr.contains("USAGE"),
        "Help text must appear on stderr, got: {stderr}"
    );
}
