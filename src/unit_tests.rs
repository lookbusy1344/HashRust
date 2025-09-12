#[cfg(test)]
use super::*;

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
