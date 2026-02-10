use crate::cli::args::{parse_hash_algorithm, parse_hash_encoding};
use crate::cli::config::{ConfigSettings, HELP};
use crate::core::types::{BasicHash, DEFAULT_HASH, HashAlgorithm, OutputEncoding};
use std::str::FromStr;

#[test]
fn test_parse_hash_algorithm_valid() {
    assert_eq!(
        parse_hash_algorithm(Some("SHA3-256")).unwrap(),
        HashAlgorithm::SHA3_256
    );
    assert_eq!(
        parse_hash_algorithm(Some("MD5")).unwrap(),
        HashAlgorithm::MD5
    );
    assert_eq!(
        parse_hash_algorithm(Some("CRC32")).unwrap(),
        HashAlgorithm::CRC32
    );
    assert_eq!(
        parse_hash_algorithm(Some("WHIRLPOOL")).unwrap(),
        HashAlgorithm::Whirlpool
    );
}

#[test]
fn test_parse_hash_algorithm_default() {
    assert_eq!(parse_hash_algorithm(None).unwrap(), DEFAULT_HASH);
    assert_eq!(parse_hash_algorithm(Some("")).unwrap(), DEFAULT_HASH);
}

#[test]
fn test_parse_hash_algorithm_invalid() {
    assert!(parse_hash_algorithm(Some("INVALID")).is_err());
}

#[test]
fn test_parse_hash_encoding_valid() {
    assert_eq!(
        parse_hash_encoding(Some("Hex")).unwrap(),
        OutputEncoding::Hex
    );
    assert_eq!(
        parse_hash_encoding(Some("Base64")).unwrap(),
        OutputEncoding::Base64
    );
    assert_eq!(
        parse_hash_encoding(Some("Base32")).unwrap(),
        OutputEncoding::Base32
    );
}

#[test]
fn test_parse_hash_encoding_default() {
    assert_eq!(
        parse_hash_encoding(None).unwrap(),
        OutputEncoding::Unspecified
    );
    assert_eq!(
        parse_hash_encoding(Some("")).unwrap(),
        OutputEncoding::Unspecified
    );
}

#[test]
fn test_hash_algorithm_from_str() {
    assert_eq!(
        HashAlgorithm::from_str("sha3-256").unwrap(),
        HashAlgorithm::SHA3_256
    );
    assert_eq!(
        HashAlgorithm::from_str("SHA2").unwrap(),
        HashAlgorithm::SHA2_256
    );
    assert_eq!(
        HashAlgorithm::from_str("sha1").unwrap(),
        HashAlgorithm::SHA1
    );
}

#[test]
fn test_config_settings_new() {
    let config = ConfigSettings::new(
        true,  // debug_mode
        false, // exclude_fn
        false, // single_thread
        true,  // case_sensitive
        false, // no_progress
        HashAlgorithm::SHA3_256,
        OutputEncoding::Hex,
        Some(100),
    );

    assert!(config.debug_mode);
    assert!(!config.exclude_fn);
    assert!(config.case_sensitive);
    assert_eq!(config.algorithm, HashAlgorithm::SHA3_256);
    assert_eq!(config.encoding, OutputEncoding::Hex);
    assert_eq!(config.limit_num, Some(100));
    assert!(config.supplied_paths.is_empty());
}

#[test]
fn test_config_settings_set_paths() {
    let mut config = ConfigSettings::new(
        false,
        false,
        false,
        false,
        false, // no_progress
        HashAlgorithm::MD5,
        OutputEncoding::Base64,
        None,
    );

    let paths = vec!["file1.txt".to_string(), "file2.txt".to_string()];
    config.set_supplied_paths(paths.clone());

    assert_eq!(config.supplied_paths, paths);
}

#[test]
fn test_basic_hash_display() {
    let hash = BasicHash("abc123".to_string());
    assert_eq!(format!("{hash}"), "abc123");
}

#[test]
fn test_help_content() {
    assert!(HELP.contains("USAGE:"));
    assert!(HELP.contains("FLAGS:"));
    assert!(HELP.contains("OPTIONS:"));
    assert!(HELP.contains("Algorithm can be:"));
    assert!(HELP.len() > 100);
}

// Core hashing function tests (Fix M5)
mod hash_tests {
    use super::*;
    use crate::hash::algorithms::call_hasher;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_file_with_content(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content).expect("Failed to write to temp file");
        file.flush().expect("Failed to flush temp file");
        file
    }

    #[test]
    fn test_call_hasher_sha3_256_hex() {
        let file = create_test_file_with_content(b"hello world");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Precomputed SHA3-256 hash of "hello world"
        assert_eq!(
            result.unwrap().0,
            "644bcc7e564373040999aac89e7622f3ca71fba1d972fd94a31c3bfbf24e3938"
        );
    }

    #[test]
    fn test_call_hasher_sha3_256_base64() {
        let file = create_test_file_with_content(b"hello world");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::Base64, &path);
        assert!(result.is_ok());

        // Base64 of SHA3-256 hash of "hello world"
        assert_eq!(
            result.unwrap().0,
            "ZEvMflZDcwQJmarInnYi88px+6HZcv2Uoxw7+/JOOTg="
        );
    }

    #[test]
    fn test_call_hasher_sha3_256_base32() {
        let file = create_test_file_with_content(b"hello world");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::Base32, &path);
        assert!(result.is_ok());

        // Base32 of SHA3-256 hash of "hello world"
        assert_eq!(
            result.unwrap().0,
            "MRF4Y7SWINZQICMZVLEJ45RC6PFHD65B3FZP3FFDDQ57X4SOHE4A===="
        );
    }

    #[test]
    fn test_call_hasher_md5_hex() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::MD5, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Precomputed MD5 hash of "test"
        assert_eq!(result.unwrap().0, "098f6bcd4621d373cade4e832627b4f6");
    }

    #[test]
    fn test_call_hasher_sha1_hex() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA1, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Precomputed SHA1 hash of "test"
        assert_eq!(
            result.unwrap().0,
            "a94a8fe5ccb19ba61c4c0873d391e987982fbbd3"
        );
    }

    #[test]
    fn test_call_hasher_sha2_256_hex() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA2_256, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Precomputed SHA2-256 hash of "test"
        assert_eq!(
            result.unwrap().0,
            "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"
        );
    }

    #[test]
    fn test_call_hasher_crc32_u32() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::CRC32, OutputEncoding::U32, &path);
        assert!(result.is_ok());

        // CRC32 of "test" is 0xD87F7E0C = 3632233996
        assert_eq!(result.unwrap().0, "3632233996");
    }

    #[test]
    fn test_call_hasher_crc32_hex_error() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        // CRC32 with non-U32 encoding should fail
        let result = call_hasher(HashAlgorithm::CRC32, OutputEncoding::Hex, &path);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("CRC32 must use U32 encoding"));
    }

    #[test]
    fn test_call_hasher_sha3_u32_error() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        // Non-CRC32 with U32 encoding should fail
        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::U32, &path);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("U32 encoding can only be used with CRC32"));
    }

    #[test]
    fn test_call_hasher_empty_file() {
        let file = create_test_file_with_content(b"");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // SHA3-256 of empty string
        assert_eq!(
            result.unwrap().0,
            "a7ffc6f8bf1ed76651c14756a061d662f580ff4de43b49fa82d80a4b80f8434a"
        );
    }

    #[test]
    fn test_call_hasher_large_file() {
        // Create a file larger than BUFFER_SIZE (32KB) to test chunked reading
        let content = vec![b'x'; 40_000];
        let file = create_test_file_with_content(&content);
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::SHA3_256, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Just verify it produces a valid hex hash
        let hash = result.unwrap().0;
        assert_eq!(hash.len(), 64); // SHA3-256 produces 32 bytes = 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_call_hasher_nonexistent_file() {
        let result = call_hasher(
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            "/nonexistent/file/path.txt",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_call_hasher_blake2s256() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::Blake2S256, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Blake2s-256 hash of "test"
        assert_eq!(
            result.unwrap().0,
            "f308fc02ce9172ad02a7d75800ecfc027109bc67987ea32aba9b8dcc7b10150e"
        );
    }

    #[test]
    fn test_call_hasher_whirlpool() {
        let file = create_test_file_with_content(b"test");
        let path = file.path().to_string_lossy().to_string();

        let result = call_hasher(HashAlgorithm::Whirlpool, OutputEncoding::Hex, &path);
        assert!(result.is_ok());

        // Just verify it produces a valid 128-char hex hash (64 bytes)
        let hash = result.unwrap().0;
        assert_eq!(hash.len(), 128);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
