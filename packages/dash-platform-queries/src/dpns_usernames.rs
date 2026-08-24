//! Transport-free DPNS username helpers.
//!
//! The Sdk-bound DPNS surface (registration, availability checks, name
//! resolution) lives in `dash-sdk`; these free functions are pure string
//! validation/normalization shared with embedders.

/// Convert a string to homograph-safe characters by replacing 'o', 'i', and 'l'
/// with '0', '1', and '1' respectively to prevent homograph attacks
pub fn convert_to_homograph_safe_chars(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'o' | 'O' => '0',
            'i' | 'I' => '1',
            'l' | 'L' => '1',
            _ => c.to_ascii_lowercase(),
        })
        .collect()
}

/// Check if a username is valid according to DPNS rules
///
/// A username is valid if:
/// - It's between 3 and 63 characters long
/// - It starts and ends with alphanumeric characters (a-zA-Z0-9)
/// - It contains only alphanumeric characters and hyphens
/// - It doesn't have consecutive hyphens (enforced by the pattern)
///
/// Pattern: `^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$`
///
/// # Arguments
///
/// * `label` - The username label to check (e.g., "alice")
///
/// # Returns
///
/// Returns `true` if the username is valid, `false` otherwise
pub fn is_valid_username(label: &str) -> bool {
    // Check length
    if label.len() < 3 || label.len() > 63 {
        return false;
    }

    let chars: Vec<char> = label.chars().collect();

    // Check first character (must be alphanumeric)
    if !chars[0].is_ascii_alphanumeric() {
        return false;
    }

    // Check last character (must be alphanumeric)
    if !chars[chars.len() - 1].is_ascii_alphanumeric() {
        return false;
    }

    // Check middle characters (can be alphanumeric or hyphen)
    for &ch in &chars[1..chars.len() - 1] {
        if !ch.is_ascii_alphanumeric() && ch != '-' {
            return false;
        }
    }

    // Additional check: no consecutive hyphens (good practice)
    for i in 0..chars.len() - 1 {
        if chars[i] == '-' && chars[i + 1] == '-' {
            return false;
        }
    }

    true
}

/// Check if a username is contested (requires masternode voting)
///
/// A username is contested if its normalized label:
/// - Is between 3 and 19 characters long (inclusive)
/// - Contains only lowercase letters a-z, digits 0-1, and hyphens
///
/// # Arguments
///
/// * `label` - The username label to check (e.g., "alice")
///
/// # Returns
///
/// Returns `true` if the username would be contested, `false` otherwise
pub fn is_contested_username(label: &str) -> bool {
    let normalized = convert_to_homograph_safe_chars(label);

    // Check length
    if normalized.len() < 3 || normalized.len() > 19 {
        return false;
    }

    // Check if all characters match the pattern [a-z01-]
    normalized
        .chars()
        .all(|c| matches!(c, 'a'..='z' | '0' | '1' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_homograph_safe_chars() {
        assert_eq!(convert_to_homograph_safe_chars("alice"), "a11ce");
        assert_eq!(convert_to_homograph_safe_chars("bob"), "b0b");
        assert_eq!(convert_to_homograph_safe_chars("COOL"), "c001");
        assert_eq!(convert_to_homograph_safe_chars("test123"), "test123");
    }

    #[test]
    fn test_is_valid_username() {
        // Valid usernames
        assert!(is_valid_username("abc"));
        assert!(is_valid_username("alice"));
        assert!(is_valid_username("Alice123"));
        assert!(is_valid_username("dash-p2p"));
        assert!(is_valid_username("test-name-123"));
        assert!(is_valid_username("a-b-c"));
        assert!(is_valid_username("user2024"));
        assert!(is_valid_username("CryptoKing"));
        assert!(is_valid_username("web3-developer"));
        assert!(is_valid_username("a".repeat(63).as_str())); // Max length

        // Invalid - too short
        assert!(!is_valid_username("ab"));
        assert!(!is_valid_username("a"));
        assert!(!is_valid_username(""));

        // Invalid - too long
        assert!(!is_valid_username("a".repeat(64).as_str()));

        // Invalid - starts with hyphen
        assert!(!is_valid_username("-alice"));
        assert!(!is_valid_username("-test"));

        // Invalid - ends with hyphen
        assert!(!is_valid_username("alice-"));
        assert!(!is_valid_username("test-"));

        // Invalid - starts and ends with hyphen
        assert!(!is_valid_username("-alice-"));

        // Invalid - contains invalid characters
        assert!(!is_valid_username("alice_bob")); // underscore
        assert!(!is_valid_username("alice.bob")); // dot
        assert!(!is_valid_username("alice@dash")); // at sign
        assert!(!is_valid_username("alice!")); // exclamation
        assert!(!is_valid_username("alice bob")); // space
        assert!(!is_valid_username("alice#1")); // hash
        assert!(!is_valid_username("alice$")); // dollar
        assert!(!is_valid_username("alice%20")); // percent

        // Invalid - consecutive hyphens
        assert!(!is_valid_username("alice--bob"));
        assert!(!is_valid_username("test---name"));
    }

    #[test]
    fn test_is_contested_username() {
        // Contested usernames (3-19 chars, only [a-z01-])
        assert!(is_contested_username("abc"));
        assert!(is_contested_username("alice")); // becomes "a11ce"
        assert!(is_contested_username("b0b"));
        assert!(is_contested_username("cool")); // becomes "c001"
        assert!(is_contested_username("a-b-c"));
        assert!(is_contested_username("hello")); // becomes "he110"
        assert!(is_contested_username("world")); // becomes "w0r1d"
        assert!(is_contested_username("dash"));
        assert!(is_contested_username("a11ce")); // already normalized
        assert!(is_contested_username("dash-dao")); // becomes "dash-da0"

        // Not contested - too short
        assert!(!is_contested_username("ab"));
        assert!(!is_contested_username("io")); // becomes "10" which is 2 chars
        assert!(!is_contested_username("a"));

        // Not contested - too long (20+ chars)
        assert!(!is_contested_username("twenty-characters-ab")); // 20 chars
        assert!(!is_contested_username(
            "this-is-a-very-long-username-that-exceeds-limit"
        ));

        // Not contested - contains invalid characters after normalization
        assert!(!is_contested_username("alice2")); // contains '2'
        assert!(!is_contested_username("alice_bob")); // contains '_'
        assert!(!is_contested_username("alice.bob")); // contains '.'
        assert!(!is_contested_username("alice@dash")); // contains '@'
        assert!(!is_contested_username("alice!")); // contains '!'
        assert!(!is_contested_username("test123")); // contains '2' and '3'
        assert!(!is_contested_username("dash-p2p")); // contains 'p' and '2'
        assert!(!is_contested_username("user5")); // contains '5'
        assert!(!is_contested_username("name_with_underscore")); // contains '_'
    }
}
