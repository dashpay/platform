//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.

use crate::error::WasmSdkError;
use crate::sdk::{WasmSdk, MAINNET_TRUSTED_CONTEXT, TESTNET_TRUSTED_CONTEXT};
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose};
use dash_sdk::dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dash_sdk::dpp::platform_value::{
    string_encoding::Encoding, Identifier, Value as PlatformValue,
};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use js_sys;
use serde_json;
use simple_signer::SingleKeySigner;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys;

// WasmSigner has been replaced with SingleKeySigner from simple-signer crate

// Helper functions for document operations
impl WasmSdk {
    /// Parse identifier strings into Identifier objects
    fn parse_identifiers(
        contract_id_str: &str,
        owner_id_str: &str,
        doc_id_str: Option<&str>,
    ) -> Result<(Identifier, Identifier, Option<Identifier>), WasmSdkError> {
        let contract_id = Identifier::from_string(contract_id_str, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        let owner_id = Identifier::from_string(owner_id_str, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid owner ID: {}", e)))?;

        let doc_id = doc_id_str
            .map(|id| Identifier::from_string(id, Encoding::Base58))
            .transpose()
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid document ID: {}", e)))?;

        Ok((contract_id, owner_id, doc_id))
    }

    /// Fetch and cache data contract
    async fn fetch_and_cache_contract(
        &self,
        contract_id: Identifier,
    ) -> Result<dash_sdk::platform::DataContract, WasmSdkError> {
        // Fetch from network
        let sdk = self.inner_clone();
        let contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))?;

        // Cache the contract in the trusted context
        if self.network() == dash_sdk::dpp::dashcore::Network::Testnet {
            if let Some(ref context) = *TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                context.add_known_contract(contract.clone());
            }
        } else if self.network() == dash_sdk::dpp::dashcore::Network::Dash {
            if let Some(ref context) = *MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                context.add_known_contract(contract.clone());
            }
        }

        Ok(contract)
    }

    /// Find authentication key matching the provided private key
    pub(crate) fn find_authentication_key<'a>(
        identity: &'a dash_sdk::platform::Identity,
        private_key_wif: &str,
    ) -> Result<(u32, &'a IdentityPublicKey), WasmSdkError> {
        // Derive public key from private key
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;

        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let private_key_bytes = private_key.inner.secret_bytes();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(
            &private_key_bytes,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid private key: {}", e)))?;
        let public_key =
            dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize().to_vec();

        // Calculate hash160 for ECDSA_HASH160 keys
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{hash160, Hash};
            hash160::Hash::hash(&public_key_bytes)
                .to_byte_array()
                .to_vec()
        };

        // Log debug information
        tracing::debug!(
            target = "wasm_sdk",
            pubkey = %hex::encode(&public_key_bytes),
            hash160 = %hex::encode(&public_key_hash160),
            "Looking for authentication key"
        );

        // Find matching authentication key
        let (key_id, matching_key) = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                if key.purpose() != Purpose::AUTHENTICATION {
                    return false;
                }

                let matches = match key.key_type() {
                    KeyType::ECDSA_SECP256K1 => {
                        key.data().as_slice() == public_key_bytes.as_slice()
                    }
                    KeyType::ECDSA_HASH160 => {
                        key.data().as_slice() == public_key_hash160.as_slice()
                    }
                    _ => false,
                };

                if matches {
                    tracing::debug!(
                        target = "wasm_sdk",
                        id = key.id(),
                        key_type = ?key.key_type(),
                        "Found matching key"
                    );
                }

                matches
            })
            .ok_or_else(|| {
                WasmSdkError::not_found(
                    "No matching authentication key found for the provided private key",
                )
            })?;

        Ok((*key_id, matching_key))
    }

    /// Create a signer from WIF private key
    pub(crate) fn create_signer_from_wif(
        private_key_wif: &str,
        network: dash_sdk::dpp::dashcore::Network,
    ) -> Result<SingleKeySigner, WasmSdkError> {
        SingleKeySigner::from_string(private_key_wif, network)
            .map_err(WasmSdkError::invalid_argument)
    }

    /// Build JavaScript result object for state transition results
    fn build_js_result_object(
        transition_type: &str,
        document_id: &str,
        additional_fields: Vec<(&str, JsValue)>,
    ) -> Result<JsValue, WasmSdkError> {
        let result_obj = js_sys::Object::new();

        // Set type
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("type"),
            &JsValue::from_str(transition_type),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set type: {:?}", e)))?;

        // Set document ID
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("documentId"),
            &JsValue::from_str(document_id),
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to set documentId: {:?}", e)))?;

        // Set additional fields
        for (key, value) in additional_fields {
            js_sys::Reflect::set(&result_obj, &JsValue::from_str(key), &value)
                .map_err(|e| WasmSdkError::generic(format!("Failed to set {}: {:?}", key, e)))?;
        }

        Ok(result_obj.into())
    }

    /// Get the next revision for a document, handling errors for missing revisions and overflow
    fn get_next_revision(document: &dash_sdk::platform::Document) -> Result<u64, WasmSdkError> {
        let current_revision = document
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        current_revision
            .checked_add(1)
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision overflow"))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Create a new document on the platform.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `owner_id` - The identity ID of the document owner
    /// * `document_data` - The document data as a JSON string
    /// * `entropy` - 32 bytes of entropy for the state transition (hex string)
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the created document
    #[wasm_bindgen(js_name = documentCreate)]
    pub async fn document_create(
        &self,
        data_contract_id: String,
        document_type: String,
        owner_id: String,
        document_data: String,
        entropy: String,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_identifier, _) =
            Self::parse_identifiers(&data_contract_id, &owner_id, None)?;

        // Parse entropy
        let entropy_bytes = hex::decode(&entropy)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid entropy hex: {}", e)))?;

        if entropy_bytes.len() != 32 {
            return Err(WasmSdkError::invalid_argument(
                "Entropy must be exactly 32 bytes",
            ));
        }

        let mut entropy_array = [0u8; 32];
        entropy_array.copy_from_slice(&entropy_bytes);

        // Parse document data
        let document_data_value: serde_json::Value =
            serde_json::from_str(&document_data).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid JSON document data: {}", e))
            })?;

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result.map_err(|e| {
            WasmSdkError::not_found(format!(
                "Document type '{}' not found: {}",
                document_type, e
            ))
        })?;

        // Convert JSON data to platform value
        let document_data_platform_value: PlatformValue = document_data_value.into();

        // Create the document directly using the document type's method
        let platform_version = sdk.version();
        let document = document_type_ref
            .create_document_from_data(
                document_data_platform_value,
                owner_identifier,
                0, // block_time (will be set by platform)
                0, // core_block_height (will be set by platform)
                entropy_array,
                platform_version,
            )
            .map_err(|e| WasmSdkError::generic(format!("Failed to create document: {}", e)))?;

        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&identity, &private_key_wif)?;
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;
        let public_key = matching_key.clone();

        // Create the state transition
        let state_transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            document_type_ref,
            entropy_array,
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create document transition: {}", e))
        })?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transition: {}", e)))?;

        // Log the result for debugging
        tracing::debug!(
            target = "wasm_sdk",
            "Processing state transition proof result"
        );

        // Convert result to JsValue based on the type
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                tracing::debug!(
                    target = "wasm_sdk",
                    count = documents.len(),
                    "Documents in result"
                );

                // Try to find the created document
                for (doc_id, maybe_doc) in documents.iter() {
                    tracing::debug!(
                        target = "wasm_sdk",
                        id = %doc_id.to_string(Encoding::Base58),
                        present = maybe_doc.is_some(),
                        "Document entry"
                    );
                }

                if let Some((doc_id, maybe_doc)) = documents.into_iter().next() {
                    if let Some(doc) = maybe_doc {
                        // Create JsValue directly instead of using serde_wasm_bindgen
                        let js_result = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentCreated"),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        // Create document object
                        let js_document = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("id"),
                            &JsValue::from_str(&doc.id().to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("ownerId"),
                            &JsValue::from_str(&doc.owner_id().to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("dataContractId"),
                            &JsValue::from_str(&data_contract_id),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("documentType"),
                            &JsValue::from_str(&document_type),
                        )
                        .unwrap();

                        if let Some(revision) = doc.revision() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("revision"),
                                &JsValue::from_f64(revision as f64),
                            )
                            .unwrap();
                        }

                        if let Some(created_at) = doc.created_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("createdAt"),
                                &JsValue::from_f64(created_at as f64),
                            )
                            .unwrap();
                        }

                        if let Some(updated_at) = doc.updated_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("updatedAt"),
                                &JsValue::from_f64(updated_at as f64),
                            )
                            .unwrap();
                        }

                        // Add document properties in a "data" field (like DocumentResponse does)
                        let data_obj = js_sys::Object::new();
                        let properties = doc.properties();

                        for (key, value) in properties {
                            // Convert platform Value to JSON value first, then to JsValue
                            if let Ok(json_value) = serde_json::to_value(value) {
                                if let Ok(js_value) = serde_wasm_bindgen::to_value(&json_value) {
                                    js_sys::Reflect::set(
                                        &data_obj,
                                        &JsValue::from_str(key),
                                        &js_value,
                                    )
                                    .unwrap();
                                }
                            }
                        }

                        js_sys::Reflect::set(&js_document, &JsValue::from_str("data"), &data_obj)
                            .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("document"),
                            &js_document,
                        )
                        .unwrap();

                        tracing::debug!(target = "wasm_sdk", "Document created successfully");

                        Ok(js_result.into())
                    } else {
                        // Document was created but not included in response (this is normal)
                        let js_result = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentCreated"),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("message"),
                            &JsValue::from_str("Document created successfully"),
                        )
                        .unwrap();

                        Ok(js_result.into())
                    }
                } else {
                    // No documents in result, but transition was successful
                    let js_result = js_sys::Object::new();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("DocumentCreated"),
                    )
                    .unwrap();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("documentId"),
                        &JsValue::from_str(&document.id().to_string(Encoding::Base58)),
                    )
                    .unwrap();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("message"),
                        &JsValue::from_str("Document created successfully"),
                    )
                    .unwrap();

                    Ok(js_result.into())
                }
            }
            _ => {
                // For other result types, just indicate success
                let js_result = js_sys::Object::new();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("type"),
                    &JsValue::from_str("DocumentCreated"),
                )
                .unwrap();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("documentId"),
                    &JsValue::from_str(&document.id().to_string(Encoding::Base58)),
                )
                .unwrap();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("message"),
                    &JsValue::from_str("Document created successfully"),
                )
                .unwrap();

                Ok(js_result.into())
            }
        }
    }

    /// Replace an existing document on the platform.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `document_id` - The ID of the document to replace
    /// * `owner_id` - The identity ID of the document owner
    /// * `document_data` - The new document data as a JSON string
    /// * `revision` - The current revision of the document
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the replaced document
    #[wasm_bindgen(js_name = documentReplace)]
    pub async fn document_replace(
        &self,
        data_contract_id: String,
        document_type: String,
        document_id: String,
        owner_id: String,
        document_data: String,
        revision: u64,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_identifier, doc_id) =
            Self::parse_identifiers(&data_contract_id, &owner_id, Some(&document_id))?;
        let doc_id = doc_id.unwrap();

        // Parse document data
        let document_data_value: serde_json::Value =
            serde_json::from_str(&document_data).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid JSON document data: {}", e))
            })?;

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result.map_err(|e| {
            WasmSdkError::not_found(format!(
                "Document type '{}' not found: {}",
                document_type, e
            ))
        })?;

        // Convert JSON data to platform value
        let document_data_platform_value: PlatformValue = document_data_value.into();

        // Create the document using the DocumentV0 constructor
        let platform_version = sdk.version();
        let document = Document::V0(DocumentV0 {
            id: doc_id,
            owner_id: owner_identifier,
            properties: document_data_platform_value
                .into_btree_string_map()
                .map_err(|e| {
                    WasmSdkError::serialization(format!("Failed to convert document data: {}", e))
                })?,
            revision: Some(revision + 1),
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

        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&identity, &private_key_wif)?;
        let public_key = matching_key.clone();
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;

        // Create the state transition
        let state_transition = BatchTransition::new_document_replacement_transition_from_document(
            document,
            document_type_ref,
            &public_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            platform_version,
            None, // state_transition_creation_options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!(
                "Failed to create document replace transition: {}",
                e
            ))
        })?;

        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast transition: {}", e)))?;

        // Convert result to JsValue based on the type
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                if let Some((doc_id, maybe_doc)) = documents.into_iter().next() {
                    if let Some(doc) = maybe_doc {
                        // Create JsValue directly instead of using serde_wasm_bindgen
                        let js_result = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentReplaced"),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        // Create document object
                        let js_document = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("id"),
                            &JsValue::from_str(&doc.id().to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("ownerId"),
                            &JsValue::from_str(&doc.owner_id().to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("dataContractId"),
                            &JsValue::from_str(&data_contract_id),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("documentType"),
                            &JsValue::from_str(&document_type),
                        )
                        .unwrap();

                        if let Some(revision) = doc.revision() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("revision"),
                                &JsValue::from_f64(revision as f64),
                            )
                            .unwrap();
                        }

                        if let Some(created_at) = doc.created_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("createdAt"),
                                &JsValue::from_f64(created_at as f64),
                            )
                            .unwrap();
                        }

                        if let Some(updated_at) = doc.updated_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("updatedAt"),
                                &JsValue::from_f64(updated_at as f64),
                            )
                            .unwrap();
                        }

                        // Add document properties in a "data" field (like DocumentResponse does)
                        let data_obj = js_sys::Object::new();
                        let properties = doc.properties();

                        for (key, value) in properties {
                            // Convert platform Value to JSON value first, then to JsValue
                            if let Ok(json_value) = serde_json::to_value(value) {
                                if let Ok(js_value) = serde_wasm_bindgen::to_value(&json_value) {
                                    js_sys::Reflect::set(
                                        &data_obj,
                                        &JsValue::from_str(key),
                                        &js_value,
                                    )
                                    .unwrap();
                                }
                            }
                        }

                        js_sys::Reflect::set(&js_document, &JsValue::from_str("data"), &data_obj)
                            .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("document"),
                            &js_document,
                        )
                        .unwrap();

                        tracing::debug!(target = "wasm_sdk", "Document replaced successfully");

                        Ok(js_result.into())
                    } else {
                        // Document was replaced but not included in response
                        let js_result = js_sys::Object::new();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentReplaced"),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        )
                        .unwrap();

                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("message"),
                            &JsValue::from_str("Document replaced successfully"),
                        )
                        .unwrap();

                        Ok(js_result.into())
                    }
                } else {
                    // No documents in result, but transition was successful
                    let js_result = js_sys::Object::new();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("DocumentReplaced"),
                    )
                    .unwrap();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("documentId"),
                        &JsValue::from_str(&document_id),
                    )
                    .unwrap();

                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("message"),
                        &JsValue::from_str("Document replaced successfully"),
                    )
                    .unwrap();

                    Ok(js_result.into())
                }
            }
            _ => {
                // For other result types, just indicate success
                let js_result = js_sys::Object::new();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("type"),
                    &JsValue::from_str("DocumentReplaced"),
                )
                .unwrap();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("documentId"),
                    &JsValue::from_str(&document_id),
                )
                .unwrap();

                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("message"),
                    &JsValue::from_str("Document replaced successfully"),
                )
                .unwrap();

                Ok(js_result.into())
            }
        }
    }

    /// Delete a document from the platform.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `document_id` - The ID of the document to delete
    /// * `owner_id` - The identity ID of the document owner
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue confirming deletion
    #[wasm_bindgen(js_name = documentDelete)]
    pub async fn document_delete(
        &self,
        data_contract_id: String,
        document_type: String,
        document_id: String,
        owner_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_identifier, doc_id) =
            Self::parse_identifiers(&data_contract_id, &owner_id, Some(&document_id))?;
        let doc_id = doc_id.unwrap();

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result.map_err(|e| {
            WasmSdkError::not_found(format!(
                "Document type '{}' not found: {}",
                document_type, e
            ))
        })?;

        // Fetch the document to get its current revision
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(&sdk, contract_id, &document_type)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
            .with_document_id(&doc_id);

        let existing_doc = dash_sdk::platform::Document::fetch(&sdk, query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        let current_revision = existing_doc
            .revision()
            .ok_or_else(|| WasmSdkError::invalid_argument("Document revision is missing"))?;

        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&identity, &private_key_wif)?;
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;

        // Create a document for deletion with the correct revision
        let document = Document::V0(DocumentV0 {
            id: doc_id,
            owner_id: owner_identifier,
            properties: Default::default(),
            revision: Some(current_revision), // Use the actual current revision
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

        // Create a delete transition
        let transition = BatchTransition::new_document_deletion_transition_from_document(
            document,
            document_type_ref,
            matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            sdk.version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create transition: {}", e)))?;

        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();

        // Broadcast the state transition
        state_transition
            .broadcast(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast: {}", e)))?;

        // Return the result with document ID
        Self::build_js_result_object(
            "DocumentDeleted",
            &document_id,
            vec![("deleted", JsValue::from_bool(true))],
        )
    }

    /// Transfer document ownership to another identity.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `document_id` - The ID of the document to transfer
    /// * `owner_id` - The current owner's identity ID
    /// * `recipient_id` - The new owner's identity ID
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the transfer result
    #[wasm_bindgen(js_name = documentTransfer)]
    pub async fn document_transfer(
        &self,
        data_contract_id: String,
        document_type: String,
        document_id: String,
        owner_id: String,
        recipient_id: String,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_identifier, doc_id) =
            Self::parse_identifiers(&data_contract_id, &owner_id, Some(&document_id))?;
        let doc_id = doc_id.expect("Document ID was provided");

        let recipient_identifier = Identifier::from_string(&recipient_id, Encoding::Base58)
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid recipient ID: {}", e)))?;

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result.map_err(|e| {
            WasmSdkError::not_found(format!(
                "Document type '{}' not found: {}",
                document_type, e
            ))
        })?;

        // Fetch the document to get its current state
        use dash_sdk::platform::DocumentQuery;

        let query = DocumentQuery::new_with_data_contract_id(&sdk, contract_id, &document_type)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
            .with_document_id(&doc_id);

        let document = dash_sdk::platform::Document::fetch(&sdk, query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        // Get the current revision and increment it
        let next_revision = Self::get_next_revision(&document)?;

        // Create a modified document with incremented revision for the transfer transition
        let transfer_document = Document::V0(DocumentV0 {
            id: document.id(),
            owner_id: document.owner_id(),
            properties: document.properties().clone(),
            revision: Some(next_revision),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            transferred_at: document.transferred_at(),
            created_at_block_height: document.created_at_block_height(),
            updated_at_block_height: document.updated_at_block_height(),
            transferred_at_block_height: document.transferred_at_block_height(),
            created_at_core_block_height: document.created_at_core_block_height(),
            updated_at_core_block_height: document.updated_at_core_block_height(),
            transferred_at_core_block_height: document.transferred_at_core_block_height(),
        });

        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&identity, &private_key_wif)?;
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;

        // Create a transfer transition
        let transition = BatchTransition::new_document_transfer_transition_from_document(
            transfer_document,
            document_type_ref,
            recipient_identifier,
            matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            sdk.version(),
            None, // options
        )
        .map_err(|e| WasmSdkError::generic(format!("Failed to create transition: {}", e)))?;

        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();

        // Broadcast the state transition
        state_transition.broadcast(&sdk, None).await?;

        // Return the result with document ID and new owner
        Self::build_js_result_object(
            "DocumentTransferred",
            &document_id,
            vec![
                ("newOwnerId", JsValue::from_str(&recipient_id)),
                ("transferred", JsValue::from_bool(true)),
            ],
        )
    }

    /// Purchase a document that has a price set.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `document_id` - The ID of the document to purchase
    /// * `buyer_id` - The buyer's identity ID
    /// * `price` - The purchase price in credits
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the purchase result
    #[wasm_bindgen(js_name = documentPurchase)]
    pub async fn document_purchase(
        &self,
        data_contract_id: String,
        document_type: String,
        document_id: String,
        buyer_id: String,
        price: u64,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, buyer_identifier, doc_id) =
            Self::parse_identifiers(&data_contract_id, &buyer_id, Some(&document_id))?;
        let doc_id = doc_id.expect("Document ID was provided");

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type from contract
        let document_type_ref = data_contract
            .document_type_for_name(&document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Fetch the document to purchase
        let query = dash_sdk::platform::documents::document_query::DocumentQuery::new_with_data_contract_id(
            &sdk,
            contract_id,
            &document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);

        let document = dash_sdk::platform::Document::fetch(&sdk, query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        // Verify the document has a price and it matches
        let listed_price = document
            .properties()
            .get_optional_integer::<u64>("$price")
            .map_err(|e| WasmSdkError::generic(format!("Failed to get document price: {}", e)))?
            .ok_or_else(|| WasmSdkError::not_found("Document is not for sale (no price set)"))?;

        if listed_price != price {
            return Err(WasmSdkError::invalid_argument(format!(
                "Price mismatch: document is listed for {} but purchase attempted with {}",
                listed_price, price
            )));
        }

        // Get the current revision and increment it
        let next_revision = Self::get_next_revision(&document)?;

        // Create a modified document with incremented revision for the purchase transition
        let purchase_document = Document::V0(DocumentV0 {
            id: document.id(),
            owner_id: document.owner_id(),
            properties: document.properties().clone(),
            revision: Some(next_revision),
            created_at: document.created_at(),
            updated_at: document.updated_at(),
            transferred_at: document.transferred_at(),
            created_at_block_height: document.created_at_block_height(),
            updated_at_block_height: document.updated_at_block_height(),
            transferred_at_block_height: document.transferred_at_block_height(),
            created_at_core_block_height: document.created_at_core_block_height(),
            updated_at_core_block_height: document.updated_at_core_block_height(),
            transferred_at_core_block_height: document.transferred_at_core_block_height(),
        });

        // Fetch buyer identity
        let buyer_identity = dash_sdk::platform::Identity::fetch(&sdk, buyer_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Buyer identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&buyer_identity, &private_key_wif)?;
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(buyer_identifier, contract_id, true, None)
            .await?;

        // Create document purchase transition
        let transition = BatchTransition::new_document_purchase_transition_from_document(
            purchase_document,
            document_type_ref,
            buyer_identifier,
            price as Credits,
            matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // No token payment info
            &signer,
            sdk.version(),
            None, // Default options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create purchase transition: {}", e))
        })?;

        // Broadcast the transition
        let proof_result = transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to broadcast purchase: {}", e)))?;

        // Handle the proof result
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                // Document purchase was successful
                let mut additional_fields = vec![
                    ("status", JsValue::from_str("success")),
                    ("newOwnerId", JsValue::from_str(&buyer_id)),
                    ("pricePaid", JsValue::from_f64(price as f64)),
                    (
                        "message",
                        JsValue::from_str("Document purchased successfully"),
                    ),
                ];

                // If we have the updated document in the response, include basic info
                if let Some((_, maybe_doc)) = documents.into_iter().next() {
                    if let Some(doc) = maybe_doc {
                        additional_fields.push(("documentUpdated", JsValue::from_bool(true)));
                        additional_fields.push((
                            "revision",
                            JsValue::from_f64(doc.revision().unwrap_or(0) as f64),
                        ));
                    }
                }

                Self::build_js_result_object(
                    "DocumentPurchased",
                    &doc_id.to_string(Encoding::Base58),
                    additional_fields,
                )
            }
            _ => {
                // Purchase was processed but document not returned
                Self::build_js_result_object(
                    "DocumentPurchased",
                    &doc_id.to_string(Encoding::Base58),
                    vec![
                        ("status", JsValue::from_str("success")),
                        ("message", JsValue::from_str("Document purchase processed")),
                    ],
                )
            }
        }
    }

    /// Set a price for a document to enable purchases.
    ///
    /// # Arguments
    ///
    /// * `data_contract_id` - The ID of the data contract
    /// * `document_type` - The name of the document type
    /// * `document_id` - The ID of the document
    /// * `owner_id` - The owner's identity ID
    /// * `price` - The price in credits (0 to remove price)
    /// * `private_key_wif` - The private key in WIF format for signing
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the result
    #[wasm_bindgen(js_name = documentSetPrice)]
    pub async fn document_set_price(
        &self,
        data_contract_id: String,
        document_type: String,
        document_id: String,
        owner_id: String,
        price: u64,
        private_key_wif: String,
    ) -> Result<JsValue, WasmSdkError> {
        let sdk = self.inner_clone();

        // Parse identifiers
        let (contract_id, owner_identifier, doc_id) =
            Self::parse_identifiers(&data_contract_id, &owner_id, Some(&document_id))?;
        let doc_id = doc_id.expect("Document ID was provided");

        // Fetch and cache the data contract
        let data_contract = self.fetch_and_cache_contract(contract_id).await?;

        // Get document type from contract
        let document_type_ref = data_contract
            .document_type_for_name(&document_type)
            .map_err(|e| WasmSdkError::not_found(format!("Document type not found: {}", e)))?;

        // Fetch the existing document to update its price
        let query = dash_sdk::platform::documents::document_query::DocumentQuery::new_with_data_contract_id(
            &sdk,
            contract_id,
            &document_type,
        )
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);

        let existing_doc = Document::fetch(&sdk, query)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Document not found"))?;

        // Verify ownership
        if existing_doc.owner_id() != owner_identifier {
            return Err(WasmSdkError::invalid_argument(
                "Only the document owner can set its price",
            ));
        }

        // Get the current revision and increment it
        let next_revision = Self::get_next_revision(&existing_doc)?;

        // Create a modified document with incremented revision for the price update transition
        let price_update_document = Document::V0(DocumentV0 {
            id: existing_doc.id(),
            owner_id: existing_doc.owner_id(),
            properties: existing_doc.properties().clone(),
            revision: Some(next_revision),
            created_at: existing_doc.created_at(),
            updated_at: existing_doc.updated_at(),
            transferred_at: existing_doc.transferred_at(),
            created_at_block_height: existing_doc.created_at_block_height(),
            updated_at_block_height: existing_doc.updated_at_block_height(),
            transferred_at_block_height: existing_doc.transferred_at_block_height(),
            created_at_core_block_height: existing_doc.created_at_core_block_height(),
            updated_at_core_block_height: existing_doc.updated_at_core_block_height(),
            transferred_at_core_block_height: existing_doc.transferred_at_core_block_height(),
        });

        // Fetch the identity to get the authentication key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await?
            .ok_or_else(|| WasmSdkError::not_found("Identity not found"))?;

        // Find matching authentication key and create signer
        let (_, matching_key) = Self::find_authentication_key(&identity, &private_key_wif)?;
        let signer = Self::create_signer_from_wif(&private_key_wif, self.network())?;

        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await?;

        // Create the price update transition using the dedicated method
        let transition = BatchTransition::new_document_update_price_transition_from_document(
            price_update_document,
            document_type_ref,
            price,
            &matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &signer,
            sdk.version(),
            None, // options
        )
        .map_err(|e| {
            WasmSdkError::generic(format!("Failed to create price update transition: {}", e))
        })?;

        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();

        // Broadcast the state transition
        state_transition.broadcast(&sdk, None).await?;

        // Return the result with document ID and price
        Self::build_js_result_object(
            "DocumentPriceSet",
            &document_id,
            vec![
                ("price", JsValue::from_f64(price as f64)),
                ("priceSet", JsValue::from_bool(true)),
            ],
        )
    }
}
