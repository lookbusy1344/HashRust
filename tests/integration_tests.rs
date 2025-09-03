use std::fs;
use std::process::Command;

#[test]
fn test_sha3_256_hash() {
    // Create a temporary test file
    let test_content = "Hello, World!";
    let test_file = "test_file.txt";
    fs::write(test_file, test_content).expect("Failed to write test file");

    // Run the hash_rust binary with SHA3-256 (default)
    let output = Command::new("cargo")
        .args(&["run", "--", test_file])
        .output()
        .expect("Failed to execute hash_rust");

    // Clean up
    fs::remove_file(test_file).ok();

    // Check output
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // SHA3-256 hash of "Hello, World!" should be consistent
    assert!(stdout.contains("1af17a664e3fa8e419b8ba05c2a173169df76162a5a286e0c405b460d478f7ef"));
    assert!(stdout.contains(test_file));
}

#[test]
fn test_md5_hash() {
    let test_content = "test";
    let test_file = "test_md5.txt";
    fs::write(test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-a", "MD5", test_file])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // MD5 hash of "test"
    assert!(stdout.contains("098f6bcd4621d373cade4e832627b4f6"));
}

#[test]
fn test_crc32_hash() {
    let test_content = "test";
    let test_file = "test_crc32.txt";
    fs::write(test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-a", "CRC32", test_file])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // CRC32 output should be 10-digit zero-padded
    assert!(stdout.contains("3632233996"));
}

#[test]
fn test_exclude_filenames() {
    let test_content = "test";
    let test_file = "test_exclude.txt";
    fs::write(test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-x", test_file])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should not contain filename when -x flag is used
    assert!(!stdout.contains(test_file));
    // But should contain the hash
    assert!(stdout.len() > 10);
}

#[test]
fn test_base64_encoding() {
    let test_content = "test";
    let test_file = "test_base64.txt";
    fs::write(test_file, test_content).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "--", "-e", "Base64", test_file])
        .output()
        .expect("Failed to execute hash_rust");

    fs::remove_file(test_file).ok();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // SHA3-256 of "test" in Base64
    assert!(stdout.contains("NjNiMDJhNmM4OGY5NGMwYzQ4ODI5MTQ3MTUxZTgyNzE2ZWYxMmY3YTI2YmFmNzg5NjNiYWJhZGY1ZTM4N2FjOA=="));
}