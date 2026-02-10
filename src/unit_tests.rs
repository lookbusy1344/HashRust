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
        file.write_all(content)
            .expect("Failed to write to temp file");
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

    // Note: CRC32/U32 validation is tested at the CLI layer in integration tests
    // (test_crc32_with_invalid_encoding_error and test_u32_encoding_with_non_crc32_error)

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

// Glob pattern matching tests (Fix 10)
mod glob_tests {
    use super::*;
    use crate::io::files::get_required_filenames;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_directory_structure() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let base_path = temp_dir.path();

        // Create test files
        fs::write(base_path.join("test1.txt"), b"content1").unwrap();
        fs::write(base_path.join("test2.txt"), b"content2").unwrap();
        fs::write(base_path.join("Test3.TXT"), b"content3").unwrap();
        fs::write(base_path.join("file.md"), b"markdown").unwrap();
        fs::write(base_path.join("data.json"), b"{}").unwrap();

        // Create a subdirectory with files
        fs::create_dir(base_path.join("subdir")).unwrap();
        fs::write(base_path.join("subdir/nested.txt"), b"nested").unwrap();

        temp_dir
    }

    #[test]
    fn test_glob_wildcard_star() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern = format!("{}/*.txt", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true, // case_sensitive
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 2); // test1.txt and test2.txt (not Test3.TXT)
    }

    #[test]
    fn test_glob_case_insensitive() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern = format!("{}/*.txt", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            false, // case_insensitive
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 3); // test1.txt, test2.txt, and Test3.TXT
    }

    #[test]
    fn test_glob_specific_extension() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern = format!("{}/*.md", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert!(paths[0].ends_with("file.md"));
    }

    #[test]
    fn test_glob_no_matches() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern = format!("{}/*.xyz", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 0); // No .xyz files
    }

    #[test]
    fn test_literal_file_exists() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let literal_path = base_path.join("test1.txt").to_string_lossy().to_string();
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![literal_path.clone()]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], literal_path);
    }

    #[test]
    fn test_literal_file_not_found() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let nonexistent = base_path
            .join("nonexistent.txt")
            .to_string_lossy()
            .to_string();
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![nonexistent]);

        let result = get_required_filenames(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_directory_path_rejected() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let dir_path = base_path.join("subdir").to_string_lossy().to_string();
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![dir_path]);

        let result = get_required_filenames(&config);
        // Should return empty list or error (directory is ignored in non-debug mode)
        assert!(result.is_ok());
        let paths = result.unwrap();
        assert_eq!(paths.len(), 0);
    }

    #[test]
    fn test_multiple_patterns() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern1 = format!("{}/*.txt", base_path.display());
        let pattern2 = format!("{}/*.md", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern1, pattern2]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 3); // 2 .txt files + 1 .md file
    }

    #[test]
    fn test_glob_with_limit() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        let pattern = format!("{}/*", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            Some(2), // Limit to 2 files
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        assert_eq!(paths.len(), 2); // Limited to 2 files
    }

    #[test]
    fn test_glob_filters_directories() {
        let temp_dir = create_test_directory_structure();
        let base_path = temp_dir.path();

        // Pattern that would match both files and directories
        let pattern = format!("{}/*", base_path.display());
        let mut config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );
        config.set_supplied_paths(vec![pattern]);

        let result = get_required_filenames(&config);
        assert!(result.is_ok());

        let paths = result.unwrap();
        // Should only include files, not the 'subdir' directory
        for path in &paths {
            assert!(!path.ends_with("subdir"));
        }
    }

    #[test]
    fn test_empty_supplied_paths() {
        // This would normally read from stdin, but we're testing the config behavior
        let config = ConfigSettings::new(
            false,
            false,
            false,
            true,
            false,
            HashAlgorithm::SHA3_256,
            OutputEncoding::Hex,
            None,
        );

        assert!(config.supplied_paths.is_empty());
    }
}
