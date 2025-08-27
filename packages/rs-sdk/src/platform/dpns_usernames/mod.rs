mod contested_queries;
mod queries;

pub use contested_queries::ContestedDpnsUsername;
pub use queries::DpnsUsername;

use crate::platform::transition::put_document::PutDocument;
use crate::platform::{Document, Fetch, FetchMany};
use crate::{Error, Sdk};
use dash_context_provider::ContextProvider;
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{DocumentV0, DocumentV0Getters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use std::collections::BTreeMap;
use std::sync::Arc;

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
/// Pattern: ^[a-zA-Z0-9][a-zA-Z0-9-]{0,61}[a-zA-Z0-9]$
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

/// Hash a buffer twice using SHA256 (double SHA256)
fn hash_double(data: Vec<u8>) -> [u8; 32] {
    use dpp::dashcore::hashes::{sha256d, Hash};
    // sha256d already does double SHA256
    let hash = sha256d::Hash::hash(&data);
    hash.to_byte_array()
}

/// Callback type for preorder document
pub type PreorderCallback = Box<dyn FnOnce(&Document) + Send>;

/// Input for registering a DPNS name
pub struct RegisterDpnsNameInput<S: Signer> {
    /// The label for the domain (e.g., "alice" for "alice.dash")
    pub label: String,
    /// The identity that will own the domain
    pub identity: Identity,
    /// The identity public key to use for signing
    pub identity_public_key: IdentityPublicKey,
    /// The signer for the identity
    pub signer: S,
    /// Optional callback to be called with the preorder document result
    pub preorder_callback: Option<PreorderCallback>,
}

/// Result of a DPNS name registration
#[derive(Debug)]
pub struct RegisterDpnsNameResult {
    /// The preorder document that was created
    pub preorder_document: Document,
    /// The domain document that was created
    pub domain_document: Document,
    /// The full domain name (e.g., "alice.dash")
    pub full_domain_name: String,
}

impl Sdk {
    /// Helper method to get the DPNS contract ID
    fn get_dpns_contract_id(&self) -> Result<Identifier, Error> {
        // Get DPNS contract ID from system contract if available
        #[cfg(feature = "dpns-contract")]
        let dpns_contract_id = {
            use dpp::system_data_contracts::SystemDataContract;
            SystemDataContract::DPNS.id()
        };

        #[cfg(not(feature = "dpns-contract"))]
        let dpns_contract_id = {
            const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
            Identifier::from_string(
                DPNS_CONTRACT_ID,
                dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| Error::DapiClientError(format!("Invalid DPNS contract ID: {}", e)))?
        };

        Ok(dpns_contract_id)
    }

    /// Helper method to fetch the DPNS contract, checking context provider first
    async fn fetch_dpns_contract(&self) -> Result<Arc<dpp::data_contract::DataContract>, Error> {
        let dpns_contract_id = self.get_dpns_contract_id()?;

        // First check if the contract is available in the context provider
        let context_provider = self
            .context_provider()
            .ok_or_else(|| Error::DapiClientError("Context provider not set".to_string()))?;

        match context_provider.get_data_contract(&dpns_contract_id, self.version())? {
            Some(contract) => Ok(contract),
            None => {
                // If not in context, fetch from platform
                let contract = crate::platform::DataContract::fetch(self, dpns_contract_id)
                    .await?
                    .ok_or_else(|| Error::DapiClientError("DPNS contract not found".to_string()))?;
                Ok(Arc::new(contract))
            }
        }
    }

    /// Register a DPNS username in a single operation
    ///
    /// This method handles both the preorder and domain registration steps automatically.
    /// It generates the necessary entropy, creates both documents, and submits them in order.
    ///
    /// # Arguments
    ///
    /// * `input` - The registration input containing label, identity, public key, and signer
    ///
    /// # Returns
    ///
    /// Returns a `RegisterDpnsNameResult` containing both created documents and the full domain name
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The DPNS contract cannot be fetched
    /// - Document types are not found in the contract
    /// - Document creation or submission fails
    pub async fn register_dpns_name<S: Signer>(
        &self,
        input: RegisterDpnsNameInput<S>,
    ) -> Result<RegisterDpnsNameResult, Error> {
        let dpns_contract = self.fetch_dpns_contract().await?;

        // Get document types
        let preorder_document_type =
            dpns_contract
                .document_type_for_name("preorder")
                .map_err(|_| {
                    Error::DapiClientError("DPNS preorder document type not found".to_string())
                })?;

        let domain_document_type =
            dpns_contract
                .document_type_for_name("domain")
                .map_err(|_| {
                    Error::DapiClientError("DPNS domain document type not found".to_string())
                })?;

        // Generate entropy and salt
        let mut rng = StdRng::from_entropy();
        let entropy = Bytes32::random_with_rng(&mut rng);
        let salt: [u8; 32] = rng.gen();

        // Generate document IDs
        let identity_id = input.identity.id().to_owned();
        let preorder_id = Document::generate_document_id_v0(
            &dpns_contract.id(),
            &identity_id,
            preorder_document_type.name(),
            entropy.as_slice(),
        );
        let domain_id = Document::generate_document_id_v0(
            &dpns_contract.id(),
            &identity_id,
            domain_document_type.name(),
            entropy.as_slice(),
        );

        // Create salted domain hash for preorder
        let normalized_label = convert_to_homograph_safe_chars(&input.label);
        let mut salted_domain_buffer: Vec<u8> = vec![];
        salted_domain_buffer.extend(salt);
        salted_domain_buffer.extend((normalized_label.clone() + ".dash").as_bytes());
        let salted_domain_hash = hash_double(salted_domain_buffer);

        // Create preorder document
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
        });

        // Create domain document
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
                ("label".to_string(), Value::Text(input.label.clone())),
                (
                    "normalizedLabel".to_string(),
                    Value::Text(normalized_label.clone()),
                ),
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
        });

        // Submit preorder document first
        let platform_preorder_document = preorder_document
            .put_to_platform_and_wait_for_response(
                self,
                preorder_document_type.to_owned_document_type(),
                Some(entropy.0),
                input.identity_public_key.clone(),
                None, // token payment info
                &input.signer,
                None, // settings
            )
            .await?;

        // Call the preorder callback if provided
        if let Some(callback) = input.preorder_callback {
            callback(&platform_preorder_document);
        }

        // Submit domain document after preorder
        let platform_domain_document = domain_document
            .put_to_platform_and_wait_for_response(
                self,
                domain_document_type.to_owned_document_type(),
                Some(entropy.0),
                input.identity_public_key,
                None, // token payment info
                &input.signer,
                None, // settings
            )
            .await?;

        Ok(RegisterDpnsNameResult {
            preorder_document: platform_preorder_document,
            domain_document: platform_domain_document,
            full_domain_name: format!("{}.dash", normalized_label),
        })
    }

    /// Check if a DPNS name is available
    ///
    /// # Arguments
    ///
    /// * `label` - The username label to check (e.g., "alice")
    ///
    /// # Returns
    ///
    /// Returns `true` if the name is available, `false` if it's taken
    pub async fn is_dpns_name_available(&self, label: &str) -> Result<bool, Error> {
        use crate::platform::documents::document_query::DocumentQuery;
        use drive::query::WhereClause;
        use drive::query::WhereOperator;

        let dpns_contract = self.fetch_dpns_contract().await?;

        let normalized_label = convert_to_homograph_safe_chars(label);

        // Query for existing domain with this label
        let query = DocumentQuery {
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(normalized_label),
                },
            ],
            order_by_clauses: vec![],
            limit: 1,
            start: None,
        };

        let documents = Document::fetch_many(self, query).await?;

        // If no documents found, the name is available
        Ok(documents.is_empty())
    }

    /// Resolve a DPNS name to an identity ID
    ///
    /// # Arguments
    ///
    /// * `name` - The full domain name (e.g., "alice.dash") or just the label (e.g., "alice")
    ///
    /// # Returns
    ///
    /// Returns the identity ID associated with the domain, or None if not found
    pub async fn resolve_dpns_name(&self, name: &str) -> Result<Option<Identifier>, Error> {
        use crate::platform::documents::document_query::DocumentQuery;
        use drive::query::WhereClause;
        use drive::query::WhereOperator;

        let dpns_contract = self.fetch_dpns_contract().await?;

        // Extract label from full name if needed
        // Handle both "alice" and "alice.dash" formats
        let label = if let Some(dot_pos) = name.rfind('.') {
            let (label_part, suffix) = name.split_at(dot_pos);
            // Only strip the suffix if it's exactly ".dash"
            if suffix == ".dash" {
                label_part
            } else {
                // If it's not ".dash", treat the whole thing as the label
                name
            }
        } else {
            // No dot found, use the whole name as the label
            name
        };

        // Validate the label before proceeding
        if label.is_empty() {
            return Ok(None);
        }

        let normalized_label = convert_to_homograph_safe_chars(label);

        // Query for domain with this label
        let query = DocumentQuery {
            data_contract: dpns_contract,
            document_type_name: "domain".to_string(),
            where_clauses: vec![
                WhereClause {
                    field: "normalizedParentDomainName".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
                WhereClause {
                    field: "normalizedLabel".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Text(normalized_label),
                },
            ],
            order_by_clauses: vec![],
            limit: 1,
            start: None,
        };

        let documents = Document::fetch_many(self, query).await?;

        if let Some((_, Some(doc))) = documents.into_iter().next() {
            // Extract the identity from records.identity
            if let Some(Value::Map(records)) = doc.properties().get("records") {
                for (key, value) in records {
                    if let (Value::Text(k), Value::Identifier(id_bytes)) = (key, value) {
                        if k == "identity" {
                            return Ok(Some(Identifier::from_bytes(id_bytes).map_err(|e| {
                                Error::DapiClientError(format!("Invalid identifier: {}", e))
                            })?));
                        }
                    }
                }
            }
        }

        Ok(None)
    }
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
