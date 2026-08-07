//! Transport-free DPNS username helpers.
//!
//! The Sdk-bound DPNS surface (registration, availability checks, name
//! resolution) lives in `dash-sdk`; the free functions here are the pure
//! pieces shared with embedders: string validation/normalization and the
//! preorder/domain document assembly used to register a name.

use crate::Error;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0};
use dpp::platform_value::Value;
use dpp::prelude::Identifier;
use std::collections::BTreeMap;

/// Hash a buffer twice using SHA256 (double SHA256)
fn hash_double(data: Vec<u8>) -> [u8; 32] {
    use dpp::dashcore::hashes::{sha256d, Hash};
    // sha256d already does double SHA256
    let hash = sha256d::Hash::hash(&data);
    hash.to_byte_array()
}

/// Build the DPNS `preorder` and `domain` documents that register
/// `label`.dash for `identity_id`, exactly as platform consensus expects
/// them.
///
/// This is the pure document-assembly half of `dash-sdk`'s
/// `register_dpns_name`: no networking, and no randomness — the caller
/// supplies the `entropy` that derives both document ids (the same entropy
/// must later be attached to both create transitions) and the preorder
/// `salt`, whose double-SHA256 over `salt ‖ "<normalized label>.dash"`
/// becomes the preorder's `saltedDomainHash`.
///
/// The `label` must satisfy [`is_consensus_valid_label`]; the raw label is stored
/// in the domain document's `label` property while its
/// [homograph-safe](convert_to_homograph_safe_chars) form is stored in
/// `normalizedLabel`.
///
/// Returns `(preorder_document, domain_document)`.
pub fn build_dpns_preorder_and_domain_documents(
    contract: &DataContract,
    identity_id: Identifier,
    label: &str,
    entropy: [u8; 32],
    salt: [u8; 32],
) -> Result<(Document, Document), Error> {
    if !is_consensus_valid_label(label) {
        return Err(Error::InvalidInput(format!(
            "Invalid DPNS label \"{label}\": must be 3-63 characters, alphanumeric and hyphens \
             only, starting and ending with an alphanumeric character"
        )));
    }

    let preorder_document_type = contract
        .document_type_for_name("preorder")
        .map_err(|_| Error::InvalidInput("DPNS preorder document type not found".to_string()))?;

    let domain_document_type = contract
        .document_type_for_name("domain")
        .map_err(|_| Error::InvalidInput("DPNS domain document type not found".to_string()))?;

    let preorder_id = Document::generate_document_id_v0(
        &contract.id(),
        &identity_id,
        preorder_document_type.name(),
        entropy.as_slice(),
    );
    let domain_id = Document::generate_document_id_v0(
        &contract.id(),
        &identity_id,
        domain_document_type.name(),
        entropy.as_slice(),
    );

    // Create salted domain hash for preorder
    let normalized_label = convert_to_homograph_safe_chars(label);
    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(salt);
    salted_domain_buffer.extend((normalized_label.clone() + ".dash").as_bytes());
    let salted_domain_hash = hash_double(salted_domain_buffer);

    let preorder_document = Document::V0(DocumentV0 {
        id: preorder_id,
        owner_id: identity_id,
        properties: BTreeMap::from([(
            "saltedDomainHash".to_string(),
            Value::Bytes32(salted_domain_hash),
        )]),
        revision: None,
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    });

    let domain_document = Document::V0(DocumentV0 {
        id: domain_id,
        owner_id: identity_id,
        properties: BTreeMap::from([
            (
                "parentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            (
                "normalizedParentDomainName".to_string(),
                Value::Text("dash".to_string()),
            ),
            ("label".to_string(), Value::Text(label.to_string())),
            ("normalizedLabel".to_string(), Value::Text(normalized_label)),
            ("preorderSalt".to_string(), Value::Bytes32(salt)),
            (
                "records".to_string(),
                Value::Map(vec![(
                    Value::Text("identity".to_string()),
                    Value::Identifier(identity_id.to_buffer()),
                )]),
            ),
            (
                "subdomainRules".to_string(),
                Value::Map(vec![(
                    Value::Text("allowSubdomains".to_string()),
                    Value::Bool(false),
                )]),
            ),
        ]),
        revision: None,
        created_at: None,
        updated_at: None,
        transferred_at: None,
        created_at_block_height: None,
        updated_at_block_height: None,
        transferred_at_block_height: None,
        created_at_core_block_height: None,
        updated_at_core_block_height: None,
        transferred_at_core_block_height: None,
        creator_id: None,
    });

    Ok((preorder_document, domain_document))
}

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

/// Check whether a label satisfies the DPNS contract's `label` schema
/// pattern — exactly what consensus enforces, nothing stricter.
///
/// Pattern: `^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$`
/// (3-63 characters, alphanumeric and hyphens, alphanumeric at both ends;
/// consecutive hyphens ARE allowed by consensus).
pub fn is_consensus_valid_label(label: &str) -> bool {
    if label.len() < 3 || label.len() > 63 {
        return false;
    }
    let chars: Vec<char> = label.chars().collect();
    if !chars[0].is_ascii_alphanumeric() || !chars[chars.len() - 1].is_ascii_alphanumeric() {
        return false;
    }
    chars[1..chars.len() - 1]
        .iter()
        .all(|&ch| ch.is_ascii_alphanumeric() || ch == '-')
}

/// Check if a username is valid according to this crate's recommended
/// client-side policy: the consensus pattern plus a stricter rejection of
/// consecutive hyphens.
///
/// This is deliberately narrower than [`is_consensus_valid_label`] — a name
/// like `ab--cd` is consensus-valid but rejected here, matching the
/// pre-existing policy of the mobile SDK FFI and wasm-sdk gates. Callers
/// that must accept every consensus-valid label should use
/// [`is_consensus_valid_label`] instead.
///
/// # Arguments
///
/// * `label` - The username label to check (e.g., "alice")
///
/// # Returns
///
/// Returns `true` if the username is valid, `false` otherwise
pub fn is_valid_username(label: &str) -> bool {
    is_consensus_valid_label(label) && !label.contains("--")
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
    use dpp::document::DocumentV0Getters;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;

    fn dpns_contract() -> DataContract {
        load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("should load DPNS system contract")
    }

    #[test]
    fn build_dpns_documents_known_vector() {
        // Fixed (label, entropy, salt) must always produce the same document
        // ids and property maps: platform consensus recomputes the ids from
        // the entropy, and the resolved name matches on these exact fields.
        let contract = dpns_contract();
        let identity_id = Identifier::from([2u8; 32]);
        let entropy = [3u8; 32];
        let salt = [4u8; 32];

        let (preorder, domain) = build_dpns_preorder_and_domain_documents(
            &contract,
            identity_id,
            "Alice",
            entropy,
            salt,
        )
        .expect("valid label must build");

        // Pinned vectors: any drift in the id derivation (contract id, owner,
        // type name, entropy layout) or the salted-hash preimage
        // (salt ‖ "a11ce.dash", double SHA256) changes these values.
        assert_eq!(
            preorder
                .id()
                .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
            "8orwov4SyqCiCppTiEogdtFHSpGyPJUfR4MtHZW8mPBB"
        );
        assert_eq!(
            domain
                .id()
                .to_string(dpp::platform_value::string_encoding::Encoding::Base58),
            "CeNRVgX6wseeTeoiJEspAJstmACSV57VfsRjHChDh5ec"
        );

        // Both ids derive from the SAME entropy (only the document type name
        // differs), which is what lets one entropy drive both create
        // transitions.
        assert_eq!(
            preorder.id(),
            Document::generate_document_id_v0(
                &contract.id(),
                &identity_id,
                "preorder",
                entropy.as_slice()
            )
        );
        assert_eq!(
            domain.id(),
            Document::generate_document_id_v0(
                &contract.id(),
                &identity_id,
                "domain",
                entropy.as_slice()
            )
        );
        assert_eq!(preorder.owner_id(), identity_id);
        assert_eq!(domain.owner_id(), identity_id);
        assert_eq!(preorder.revision(), None);
        assert_eq!(domain.revision(), None);

        // saltedDomainHash = sha256d(salt ‖ "a11ce.dash"), pinned as a vector.
        let expected_hash: [u8; 32] =
            hex::decode("5396e080af450f80f4f8ddbfc3eb0a885674c9cf6edbea815dad7305b558e253")
                .expect("valid hex")
                .try_into()
                .expect("32 bytes");
        assert_eq!(
            preorder.properties(),
            &BTreeMap::from([(
                "saltedDomainHash".to_string(),
                Value::Bytes32(expected_hash)
            )])
        );

        assert_eq!(
            domain.properties(),
            &BTreeMap::from([
                (
                    "parentDomainName".to_string(),
                    Value::Text("dash".to_string())
                ),
                (
                    "normalizedParentDomainName".to_string(),
                    Value::Text("dash".to_string())
                ),
                ("label".to_string(), Value::Text("Alice".to_string())),
                (
                    "normalizedLabel".to_string(),
                    Value::Text("a11ce".to_string())
                ),
                ("preorderSalt".to_string(), Value::Bytes32(salt)),
                (
                    "records".to_string(),
                    Value::Map(vec![(
                        Value::Text("identity".to_string()),
                        Value::Identifier(identity_id.to_buffer())
                    )])
                ),
                (
                    "subdomainRules".to_string(),
                    Value::Map(vec![(
                        Value::Text("allowSubdomains".to_string()),
                        Value::Bool(false)
                    )])
                ),
            ])
        );
    }

    #[test]
    fn build_dpns_documents_rejects_invalid_label() {
        let contract = dpns_contract();
        let identity_id = Identifier::from([2u8; 32]);

        for bad in ["", "ab", "-alice", "alice-", "alice_bob"] {
            let result = build_dpns_preorder_and_domain_documents(
                &contract,
                identity_id,
                bad,
                [3u8; 32],
                [4u8; 32],
            );
            assert!(
                matches!(result, Err(Error::InvalidInput(_))),
                "label {bad:?} must be rejected"
            );
        }
    }

    /// Consecutive hyphens are consensus-valid (the DPNS contract pattern
    /// `^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$` allows them), so the
    /// builder must accept them even though the stricter client-side
    /// [`is_valid_username`] policy rejects them.
    #[test]
    fn build_dpns_documents_accepts_consensus_valid_double_hyphen() {
        let contract = dpns_contract();
        let identity_id = Identifier::from([2u8; 32]);

        assert!(is_consensus_valid_label("alice--bob"));
        assert!(!is_valid_username("alice--bob"));
        build_dpns_preorder_and_domain_documents(
            &contract,
            identity_id,
            "alice--bob",
            [3u8; 32],
            [4u8; 32],
        )
        .expect("consensus-valid label with consecutive hyphens must build");
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
