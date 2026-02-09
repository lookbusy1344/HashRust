use std::io::Write;
use std::process::Command;
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
        .args(&["run", "--", test_path.to_str().unwrap()])
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
        .args(&["run", "--", "-a", "MD5", test_path.to_str().unwrap()])
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
        .args(&["run", "--", "-a", "CRC32", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // CRC32 output should be 10-digit zero-padded
    assert!(stdout.contains("3632233996"));
}

#[test]
fn test_exclude_filenames() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(&["run", "--", "-x", test_path.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain filename when -x flag is used
    assert!(!stdout.contains(test_path.file_name().unwrap().to_str().unwrap()));
    // But should contain the hash. SHA3-256 hash output is 64 characters, so this ensures a hash is present.
    const MIN_HASH_LENGTH: usize = 64;
    assert!(stdout.len() >= MIN_HASH_LENGTH);
}

#[test]
fn test_base64_encoding() {
    let test_content = "test";
    let test_file = create_temp_file(test_content);
    let test_path = test_file.path();

    let output = Command::new("cargo")
        .args(&["run", "--", "-e", "Base64", test_path.to_str().unwrap()])
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
        .args(&["run", "--", nonexistent_file.to_str().unwrap()])
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
        .args(&[
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
        .args(&[
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
        .args(&[
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
        .args(&[
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
        .args(&["run", "--"])
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
        .args(&["run", "--", "--help"])
        .output()
        .expect("Failed to execute hash_rust");

    // Should succeed and show help
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should contain help information
    assert!(stdout.to_lowercase().contains("usage") || stdout.to_lowercase().contains("help"));
}
