//! Transport-free DPNS username helpers.
//!
//! The Sdk-bound DPNS surface (registration, availability checks, name
//! resolution) lives in `dash-sdk`; these free functions are pure string
//! validation/normalization and document assembly shared with embedders.
//! Normalization is dpp's consensus implementation
//! ([`convert_to_homograph_safe_chars`]), the same one the DPNS data trigger
//! checks `normalizedLabel` against.

use crate::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use dpp::system_data_contracts::dpns_contract::v1::document_types::domain;
use dpp::util::hash::hash_double;
use std::collections::BTreeMap;

/// Document type name of the DPNS preorder document.
pub const PREORDER_DOCUMENT_TYPE: &str = "preorder";
/// The only parent domain names can currently be registered under.
pub const DASH_PARENT_DOMAIN: &str = "dash";

pub use dpp::util::strings::convert_to_homograph_safe_chars;

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

/// The DPNS `preorder` document that blinds `label`.dash behind `salt`.
///
/// `saltedDomainHash` is `sha256d(salt ‖ "<normalized label>.dash")` — the
/// same preimage the DPNS data trigger recomputes when the paired `domain`
/// document is created. The document id derives from `entropy`, which must
/// be reused on the create transition.
///
/// Callers driving their own registration must draw `salt` from a CSPRNG and
/// keep it, the label, and the domain document private until the preorder is
/// confirmed, or the preorder's front-running protection is lost.
pub fn build_dpns_preorder_document(
    contract: &DataContract,
    owner_id: Identifier,
    label: &str,
    salt: [u8; 32],
    entropy: [u8; 32],
) -> Result<Document, Error> {
    let document_type = contract.document_type_for_name(PREORDER_DOCUMENT_TYPE)?;
    let properties = BTreeMap::from([(
        "saltedDomainHash".to_string(),
        Value::Bytes32(salted_domain_hash(label, salt)),
    )]);
    Ok(new_document(
        contract,
        document_type.name(),
        owner_id,
        entropy,
        properties,
    ))
}

/// The DPNS `domain` document registering `label`.dash for `owner_id`, with
/// the identity record pointing at the owner and subdomains disallowed.
///
/// Rejects a label the DPNS contract's consensus pattern would refuse, so an
/// invalid name fails before a preorder is paid for. `normalizedLabel` is the
/// consensus normalization of `label`.
pub fn build_dpns_domain_document(
    contract: &DataContract,
    owner_id: Identifier,
    label: &str,
    salt: [u8; 32],
    entropy: [u8; 32],
) -> Result<Document, Error> {
    if !is_valid_username(label) {
        return Err(Error::Config(format!(
            "DPNS label {label:?} does not match the contract's label pattern"
        )));
    }
    let document_type = contract.document_type_for_name(domain::NAME)?;
    let properties = BTreeMap::from([
        (
            domain::properties::PARENT_DOMAIN_NAME.to_string(),
            Value::Text(DASH_PARENT_DOMAIN.to_string()),
        ),
        (
            domain::properties::NORMALIZED_PARENT_DOMAIN_NAME.to_string(),
            Value::Text(DASH_PARENT_DOMAIN.to_string()),
        ),
        (
            domain::properties::LABEL.to_string(),
            Value::Text(label.to_string()),
        ),
        (
            domain::properties::NORMALIZED_LABEL.to_string(),
            Value::Text(convert_to_homograph_safe_chars(label)),
        ),
        (
            domain::properties::PREORDER_SALT.to_string(),
            Value::Bytes32(salt),
        ),
        (
            domain::properties::RECORDS.to_string(),
            Value::Map(vec![(
                Value::Text(domain::properties::IDENTITY.to_string()),
                Value::Identifier(owner_id.to_buffer()),
            )]),
        ),
        (
            "subdomainRules".to_string(),
            Value::Map(vec![(
                Value::Text("allowSubdomains".to_string()),
                Value::Bool(false),
            )]),
        ),
    ]);
    Ok(new_document(
        contract,
        document_type.name(),
        owner_id,
        entropy,
        properties,
    ))
}

/// `sha256d(salt ‖ "<normalized label>.dash")`: the preorder commitment the
/// DPNS data trigger recomputes from the domain document.
pub fn salted_domain_hash(label: &str, salt: [u8; 32]) -> [u8; 32] {
    let mut preimage = salt.to_vec();
    preimage.extend_from_slice(convert_to_homograph_safe_chars(label).as_bytes());
    preimage.extend_from_slice(b".");
    preimage.extend_from_slice(DASH_PARENT_DOMAIN.as_bytes());
    hash_double(preimage)
}

/// A fresh document whose id derives from `entropy`, with every
/// chain-assigned field left unset (they never enter a create transition).
pub(crate) fn new_document(
    contract: &DataContract,
    document_type_name: &str,
    owner_id: Identifier,
    entropy: [u8; 32],
    properties: BTreeMap<String, Value>,
) -> Document {
    Document::V0(DocumentV0 {
        id: Document::generate_document_id_v0(
            &contract.id(),
            &owner_id,
            document_type_name,
            &entropy,
        ),
        owner_id,
        properties,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_build_preorder_and_domain_documents_that_agree() {
        use dpp::document::DocumentV0Getters;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;

        let contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("dpns contract");
        let owner = Identifier::from([1u8; 32]);
        let salt = [5u8; 32];
        let entropy = [9u8; 32];

        let preorder = build_dpns_preorder_document(&contract, owner, "Alice", salt, entropy)
            .expect("preorder");
        let domain =
            build_dpns_domain_document(&contract, owner, "Alice", salt, entropy).expect("domain");

        // The commitment in the preorder is the one the DPNS data trigger
        // recomputes from the domain document's salt and normalized label.
        assert_eq!(
            preorder.get("saltedDomainHash"),
            Some(&Value::Bytes32(salted_domain_hash("Alice", salt)))
        );
        assert_eq!(
            domain.get(domain::properties::NORMALIZED_LABEL),
            Some(&Value::Text("a11ce".to_string()))
        );
        assert_eq!(
            domain.get(domain::properties::LABEL),
            Some(&Value::Text("Alice".to_string()))
        );
        assert_eq!(
            domain.get(domain::properties::PREORDER_SALT),
            Some(&Value::Bytes32(salt))
        );
        assert_eq!(
            preorder.id(),
            Document::generate_document_id_v0(
                &contract.id(),
                &owner,
                PREORDER_DOCUMENT_TYPE,
                &entropy
            )
        );
        assert_ne!(preorder.id(), domain.id());
    }

    #[test]
    fn should_refuse_a_label_the_contract_pattern_rejects() {
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;

        let contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("dpns contract");
        build_dpns_domain_document(
            &contract,
            Identifier::from([1u8; 32]),
            "-bad",
            [0; 32],
            [0; 32],
        )
        .expect_err("leading hyphen violates the label pattern");
    }

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
