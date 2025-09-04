use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_sha3_256_hash() {
    // Create a temporary test file
    let test_content = "Hello, World!";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_file.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    // Run the hash_rust binary with SHA3-256 (default)
    let output = Command::new("cargo")
        .args(&["run", "--", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    // Clean up
    fs::remove_file(&test_file).ok();

    // Check output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // SHA3-256 hash of "Hello, World!" should be consistent
    assert!(stdout.contains("1af17a664e3fa8e419b8ba05c2a173169df76162a5a286e0c405b460d478f7ef"));
    assert!(stdout.contains(test_file.file_name().unwrap().to_str().unwrap()));
}

#[test]
fn test_md5_hash() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_md5.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-a", "MD5", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // MD5 hash of "test"
    assert!(stdout.contains("098f6bcd4621d373cade4e832627b4f6"));
}

#[test]
fn test_crc32_hash() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_crc32.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-a", "CRC32", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // CRC32 output should be 10-digit zero-padded
    assert!(stdout.contains("3632233996"));
}

#[test]
fn test_exclude_filenames() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_exclude.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-x", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not contain filename when -x flag is used
    assert!(!stdout.contains(test_file.file_name().unwrap().to_str().unwrap()));
    // But should contain the hash. SHA3-256 hash output is 64 characters, so this ensures a hash is present.
    const MIN_HASH_LENGTH: usize = 64;
    assert!(stdout.len() >= MIN_HASH_LENGTH);
}

#[test]
fn test_base64_encoding() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_base64.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-e", "Base64", test_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

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
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_invalid_algo.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "-a",
            "INVALID_ALGORITHM",
            test_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about invalid algorithm
    assert!(stderr.to_lowercase().contains("error"));
}

#[test]
fn test_invalid_encoding_error() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_invalid_encoding.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "-e",
            "INVALID_ENCODING",
            test_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about invalid encoding
    assert!(stderr.to_lowercase().contains("error"));
}

#[test]
fn test_crc32_with_invalid_encoding_error() {
    let test_content = "test";
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_crc32_invalid_encoding.txt");
    fs::write(&test_file, test_content).expect("Failed to write test file");

    // CRC32 should only work with Hex encoding (U32 format)
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--",
            "-a",
            "CRC32",
            "-e",
            "Base64",
            test_file.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(&test_file).ok();

    // Should fail with non-zero exit code
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should contain error message about incompatible algorithm/encoding combination
    assert!(stderr.to_lowercase().contains("error"));
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
