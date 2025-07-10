use crate::platform::transition::put_document::PutDocument;
use crate::platform::{Document, FetchMany, Fetch};
use crate::{Error, Sdk};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::document::{DocumentV0, DocumentV0Getters};
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use std::collections::BTreeMap;

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

/// Hash a buffer twice using SHA256 (double SHA256)
fn hash_double(data: Vec<u8>) -> [u8; 32] {
    use dpp::dashcore::hashes::{sha256d, Hash};
    // sha256d already does double SHA256
    let hash = sha256d::Hash::hash(&data);
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_byte_array());
    result
}

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
        // Fetch the DPNS contract
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, dpp::platform_value::string_encoding::Encoding::Base58)
            .map_err(|e| Error::DapiClientError(format!("Invalid DPNS contract ID: {}", e)))?;
        
        let dpns_contract = crate::platform::DataContract::fetch(self, dpns_contract_id)
            .await?
            .ok_or_else(|| Error::DapiClientError("DPNS contract not found".to_string()))?;

        // Get document types
        let preorder_document_type = dpns_contract
            .document_type_for_name("preorder")
            .map_err(|_| Error::DapiClientError("DPNS preorder document type not found".to_string()))?;
        
        let domain_document_type = dpns_contract
            .document_type_for_name("domain")
            .map_err(|_| Error::DapiClientError("DPNS domain document type not found".to_string()))?;

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
                ("parentDomainName".to_string(), Value::Text("dash".to_string())),
                ("normalizedParentDomainName".to_string(), Value::Text("dash".to_string())),
                ("label".to_string(), Value::Text(input.label.clone())),
                ("normalizedLabel".to_string(), Value::Text(normalized_label.clone())),
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
        preorder_document
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

        // Submit domain document after preorder
        domain_document
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
            preorder_document,
            domain_document,
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
        
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, dpp::platform_value::string_encoding::Encoding::Base58)
            .map_err(|e| Error::DapiClientError(format!("Invalid DPNS contract ID: {}", e)))?;
        
        let dpns_contract = crate::platform::DataContract::fetch(self, dpns_contract_id)
            .await?
            .ok_or_else(|| Error::DapiClientError("DPNS contract not found".to_string()))?;

        let normalized_label = convert_to_homograph_safe_chars(label);
        
        // Query for existing domain with this label
        let query = DocumentQuery {
            data_contract: dpns_contract.into(),
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
        
        const DPNS_CONTRACT_ID: &str = "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec";
        let dpns_contract_id = Identifier::from_string(DPNS_CONTRACT_ID, dpp::platform_value::string_encoding::Encoding::Base58)
            .map_err(|e| Error::DapiClientError(format!("Invalid DPNS contract ID: {}", e)))?;
        
        let dpns_contract = crate::platform::DataContract::fetch(self, dpns_contract_id)
            .await?
            .ok_or_else(|| Error::DapiClientError("DPNS contract not found".to_string()))?;

        // Extract label from full name if needed
        let label = name.trim_end_matches(".dash");
        let normalized_label = convert_to_homograph_safe_chars(label);
        
        // Query for domain with this label
        let query = DocumentQuery {
            data_contract: dpns_contract.into(),
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
                            return Ok(Some(Identifier::from_bytes(id_bytes)
                                .map_err(|e| Error::DapiClientError(format!("Invalid identifier: {}", e)))?));
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
}