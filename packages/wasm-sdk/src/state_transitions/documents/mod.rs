//! Document state transition implementations for the WASM SDK.
//!
//! This module provides WASM bindings for document operations like create, replace, delete, etc.

use crate::sdk::{WasmSdk, MAINNET_TRUSTED_CONTEXT, TESTNET_TRUSTED_CONTEXT};
use crate::context_provider::WasmTrustedContext;
use dash_sdk::dpp::dashcore::PrivateKey;
use dash_sdk::dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dash_sdk::dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::signer::Signer;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::platform_value::{Identifier, BinaryData, string_encoding::Encoding, Value as PlatformValue, Value};
use dash_sdk::dpp::prelude::UserFeeIncrease;
use dash_sdk::dpp::state_transition::batch_transition::BatchTransition;
use dash_sdk::dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dash_sdk::dpp::state_transition::batch_transition::methods::StateTransitionCreationOptions;
use dash_sdk::dpp::fee::Credits;
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::state_transition::StateTransition;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::dpp::document::{Document, DocumentV0Getters, DocumentV0};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use dash_sdk::dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use simple_signer::SingleKeySigner;
use serde_wasm_bindgen::to_value;
use serde_json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys;
use js_sys;

// WasmSigner has been replaced with SingleKeySigner from simple-signer crate

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
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
        
        // Parse entropy
        let entropy_bytes = hex::decode(&entropy)
            .map_err(|e| JsValue::from_str(&format!("Invalid entropy hex: {}", e)))?;
        
        if entropy_bytes.len() != 32 {
            return Err(JsValue::from_str("Entropy must be exactly 32 bytes"));
        }
        
        let mut entropy_array = [0u8; 32];
        entropy_array.copy_from_slice(&entropy_bytes);
        
        // Parse document data
        let document_data_value: serde_json::Value = serde_json::from_str(&document_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON document data: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result
            .map_err(|e| JsValue::from_str(&format!("Document type '{}' not found: {}", document_type, e)))?;
        
        // Convert JSON data to platform value
        let document_data_platform_value = convert_json_to_platform_value(document_data_value)?;
        
        // Create the document directly using the document type's method
        let platform_version = sdk.version();
        let document = document_type_ref.create_document_from_data(
            document_data_platform_value,
            owner_identifier,
            0, // block_time (will be set by platform)
            0, // core_block_height (will be set by platform)
            entropy_array,
            platform_version,
        ).map_err(|e| JsValue::from_str(&format!("Failed to create document: {}", e)))?;
        
        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        let private_key_bytes = signer.private_key().to_bytes();
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?;
        let secp_public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = secp_public_key.serialize().to_vec();
        
        // Log debug information
        web_sys::console::log_1(&JsValue::from_str(&format!(
            "Derived public key (hex): {}",
            hex::encode(&public_key_bytes)
        )));
        
        // Find a matching authentication key from the identity
        // For ECDSA_HASH160 keys, we need to hash the public key
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes).to_byte_array().to_vec()
        };
        
        let matching_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                if key.purpose() != Purpose::AUTHENTICATION {
                    return false;
                }
                
                let matches = match key.key_type() {
                    KeyType::ECDSA_SECP256K1 => {
                        // For ECDSA_SECP256K1, compare the full public key
                        key.data().as_slice() == public_key_bytes.as_slice()
                    },
                    KeyType::ECDSA_HASH160 => {
                        // For ECDSA_HASH160, compare the hash of the public key
                        key.data().as_slice() == public_key_hash160.as_slice()
                    },
                    _ => false
                };
                
                if key.purpose() == Purpose::AUTHENTICATION {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Checking auth key ID {}: type={:?}, data={}, pubkey_hash160={}, matches={}",
                        key.id(),
                        key.key_type(),
                        hex::encode(key.data().as_slice()),
                        hex::encode(&public_key_hash160),
                        matches
                    )));
                }
                
                matches
            })
            .ok_or_else(|| {
                let error_msg = format!(
                    "No matching authentication key found for the provided private key, these are the keys on this identity\n\
                    {:<10} {:<40} {:<20} {:<20} {:<20} {:<10} {:<}\n{}",
                    "Key Id", "Public Key Hash", "Type", "Purpose", "Security Level", "Read Only", "Data",
                    identity.public_keys()
                        .iter()
                        .map(|(_, key)| format!(
                            "{:<10} {:<40} {:<20} {:<20} {:<20} {:<10} {:<}",
                            key.id(),
                            hex::encode(key.data().as_slice()),
                            format!("{:?}", key.key_type()).replace("KeyType::", ""),
                            format!("{:?}", key.purpose()).replace("Purpose::", ""),
                            format!("{:?}", key.security_level()).replace("SecurityLevel::", ""),
                            if key.read_only() { "True" } else { "False" },
                            hex::encode(key.data().as_slice())
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                );
                JsValue::from_str(&error_msg)
            })?;
        
        let public_key = matching_key.1.clone();
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
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
        ).map_err(|e| JsValue::from_str(&format!("Failed to create document transition: {}", e)))?;
        
        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;
        
        // Log the result for debugging
        web_sys::console::log_1(&JsValue::from_str("Processing state transition proof result"));
        
        // Convert result to JsValue based on the type
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                web_sys::console::log_1(&JsValue::from_str(&format!(
                    "Documents in result: {}",
                    documents.len()
                )));
                
                // Try to find the created document
                for (doc_id, maybe_doc) in documents.iter() {
                    web_sys::console::log_1(&JsValue::from_str(&format!(
                        "Document ID: {}, Document present: {}",
                        doc_id.to_string(Encoding::Base58),
                        maybe_doc.is_some()
                    )));
                }
                
                if let Some((doc_id, maybe_doc)) = documents.into_iter().next() {
                    if let Some(doc) = maybe_doc {
                        // Create JsValue directly instead of using serde_wasm_bindgen
                        let js_result = js_sys::Object::new();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentCreated"),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        // Create document object
                        let js_document = js_sys::Object::new();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("id"),
                            &JsValue::from_str(&doc.id().to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("ownerId"),
                            &JsValue::from_str(&doc.owner_id().to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("dataContractId"),
                            &JsValue::from_str(&data_contract_id),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("documentType"),
                            &JsValue::from_str(&document_type),
                        ).unwrap();
                        
                        if let Some(revision) = doc.revision() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("revision"),
                                &JsValue::from_f64(revision as f64),
                            ).unwrap();
                        }
                        
                        if let Some(created_at) = doc.created_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("createdAt"),
                                &JsValue::from_f64(created_at as f64),
                            ).unwrap();
                        }
                        
                        if let Some(updated_at) = doc.updated_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("updatedAt"),
                                &JsValue::from_f64(updated_at as f64),
                            ).unwrap();
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
                                    ).unwrap();
                                }
                            }
                        }
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("data"),
                            &data_obj,
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("document"),
                            &js_document,
                        ).unwrap();
                        
                        web_sys::console::log_1(&JsValue::from_str("Document created successfully, returning JS object"));
                        
                        Ok(js_result.into())
                    } else {
                        // Document was created but not included in response (this is normal)
                        let js_result = js_sys::Object::new();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentCreated"),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("message"),
                            &JsValue::from_str("Document created successfully"),
                        ).unwrap();
                        
                        Ok(js_result.into())
                    }
                } else {
                    // No documents in result, but transition was successful
                    let js_result = js_sys::Object::new();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("DocumentCreated"),
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("documentId"),
                        &JsValue::from_str(&document.id().to_string(Encoding::Base58)),
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("message"),
                        &JsValue::from_str("Document created successfully"),
                    ).unwrap();
                    
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
                ).unwrap();
                
                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("documentId"),
                    &JsValue::from_str(&document.id().to_string(Encoding::Base58)),
                ).unwrap();
                
                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("message"),
                    &JsValue::from_str("Document created successfully"),
                ).unwrap();
                
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
    /// * `key_id` - The key ID to use for signing
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
        _key_id: u32,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
            
        let doc_id = Identifier::from_string(&document_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid document ID: {}", e)))?;
        
        // Parse document data
        let document_data_value: serde_json::Value = serde_json::from_str(&document_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid JSON document data: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result
            .map_err(|e| JsValue::from_str(&format!("Document type '{}' not found: {}", document_type, e)))?;
        
        // Convert JSON data to platform value
        let document_data_platform_value = convert_json_to_platform_value(document_data_value)?;
        
        // Create the document using the DocumentV0 constructor
        let platform_version = sdk.version();
        let document = Document::V0(DocumentV0 {
            id: doc_id,
            owner_id: owner_identifier,
            properties: document_data_platform_value
                .into_btree_string_map()
                .map_err(|e| JsValue::from_str(&format!("Failed to convert document data: {}", e)))?,
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
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        let private_key_bytes = signer.private_key().to_bytes();
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?;
        let secp_public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = secp_public_key.serialize().to_vec();
        
        // Find a matching authentication key from the identity
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes).to_byte_array().to_vec()
        };
        
        let matching_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                if key.purpose() != Purpose::AUTHENTICATION {
                    return false;
                }
                
                match key.key_type() {
                    KeyType::ECDSA_SECP256K1 => {
                        key.data().as_slice() == public_key_bytes.as_slice()
                    },
                    KeyType::ECDSA_HASH160 => {
                        key.data().as_slice() == public_key_hash160.as_slice()
                    },
                    _ => false
                }
            })
            .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?;
        
        let public_key = matching_key.1.clone();
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
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
        ).map_err(|e| JsValue::from_str(&format!("Failed to create document replace transition: {}", e)))?;
        
        // Broadcast the transition
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast transition: {}", e)))?;
        
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
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        // Create document object
                        let js_document = js_sys::Object::new();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("id"),
                            &JsValue::from_str(&doc.id().to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("ownerId"),
                            &JsValue::from_str(&doc.owner_id().to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("dataContractId"),
                            &JsValue::from_str(&data_contract_id),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("documentType"),
                            &JsValue::from_str(&document_type),
                        ).unwrap();
                        
                        if let Some(revision) = doc.revision() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("revision"),
                                &JsValue::from_f64(revision as f64),
                            ).unwrap();
                        }
                        
                        if let Some(created_at) = doc.created_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("createdAt"),
                                &JsValue::from_f64(created_at as f64),
                            ).unwrap();
                        }
                        
                        if let Some(updated_at) = doc.updated_at() {
                            js_sys::Reflect::set(
                                &js_document,
                                &JsValue::from_str("updatedAt"),
                                &JsValue::from_f64(updated_at as f64),
                            ).unwrap();
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
                                    ).unwrap();
                                }
                            }
                        }
                        
                        js_sys::Reflect::set(
                            &js_document,
                            &JsValue::from_str("data"),
                            &data_obj,
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("document"),
                            &js_document,
                        ).unwrap();
                        
                        web_sys::console::log_1(&JsValue::from_str("Document replaced successfully"));
                        
                        Ok(js_result.into())
                    } else {
                        // Document was replaced but not included in response
                        let js_result = js_sys::Object::new();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("type"),
                            &JsValue::from_str("DocumentReplaced"),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("documentId"),
                            &JsValue::from_str(&doc_id.to_string(Encoding::Base58)),
                        ).unwrap();
                        
                        js_sys::Reflect::set(
                            &js_result,
                            &JsValue::from_str("message"),
                            &JsValue::from_str("Document replaced successfully"),
                        ).unwrap();
                        
                        Ok(js_result.into())
                    }
                } else {
                    // No documents in result, but transition was successful
                    let js_result = js_sys::Object::new();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("type"),
                        &JsValue::from_str("DocumentReplaced"),
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("documentId"),
                        &JsValue::from_str(&document_id),
                    ).unwrap();
                    
                    js_sys::Reflect::set(
                        &js_result,
                        &JsValue::from_str("message"),
                        &JsValue::from_str("Document replaced successfully"),
                    ).unwrap();
                    
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
                ).unwrap();
                
                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("documentId"),
                    &JsValue::from_str(&document_id),
                ).unwrap();
                
                js_sys::Reflect::set(
                    &js_result,
                    &JsValue::from_str("message"),
                    &JsValue::from_str("Document replaced successfully"),
                ).unwrap();
                
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
    /// * `key_id` - The key ID to use for signing
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
        _key_id: u32,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
            
        let doc_id = Identifier::from_string(&document_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid document ID: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result
            .map_err(|e| JsValue::from_str(&format!("Document type '{}' not found: {}", document_type, e)))?;
        
        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        let private_key_bytes = signer.private_key().to_bytes();
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?;
        let secp_public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = secp_public_key.serialize().to_vec();
        
        // Find a matching authentication key from the identity
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes).to_byte_array().to_vec()
        };
        
        let matching_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION &&
                key.key_type() == KeyType::ECDSA_HASH160 &&
                key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(_, key)| key)
            .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?;
        
        // Create a minimal document for deletion
        let document = Document::V0(DocumentV0 {
            id: doc_id,
            owner_id: owner_identifier,
            properties: Default::default(),
            revision: Some(1), // Use initial revision for deletion
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
            &SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
                .map_err(|e| JsValue::from_str(&e))?,
            sdk.version(),
            None, // options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create transition: {}", e)))?;
        
        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();
        
        // Broadcast the state transition
        state_transition
            .broadcast(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast: {}", e)))?;
        
        // Return the result with document ID
        // For deletion, we just need to confirm the broadcast succeeded
        let result_obj = js_sys::Object::new();
        
        // Set document ID
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("documentId"),
            &JsValue::from_str(&document_id),
        ).unwrap();
        
        // Set deleted status
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("deleted"),
            &JsValue::from_bool(true),
        ).unwrap();
        
        Ok(result_obj.into())
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
    /// * `key_id` - The key ID to use for signing
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
        _key_id: u32,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
        
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
            
        let recipient_identifier = Identifier::from_string(&recipient_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid recipient ID: {}", e)))?;
            
        let doc_id = Identifier::from_string(&document_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid document ID: {}", e)))?;
        
        // Fetch the data contract first to ensure it's in the cache
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Add the contract to the context provider's cache if using trusted mode
        match sdk.network {
            dash_sdk::dpp::dashcore::Network::Testnet => {
                if let Some(ref context) = *TESTNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            dash_sdk::dpp::dashcore::Network::Dash => {
                if let Some(ref context) = *MAINNET_TRUSTED_CONTEXT.lock().unwrap() {
                    context.add_known_contract(data_contract.clone());
                }
            }
            _ => {} // Other networks don't use trusted context
        }
        
        // Get document type
        let document_type_result = data_contract.document_type_for_name(&document_type);
        let document_type_ref = document_type_result
            .map_err(|e| JsValue::from_str(&format!("Document type '{}' not found: {}", document_type, e)))?;
        
        // Fetch the document to get its current state
        use dash_sdk::platform::DocumentQuery;
        
        let query = DocumentQuery::new_with_data_contract_id(
            &sdk,
            contract_id,
            &document_type,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);
        
        let document = dash_sdk::platform::Document::fetch(&sdk, query)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch document: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Document not found"))?;
        
        // Fetch the identity to get the correct key
        let identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Create public key from the private key
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        let private_key_bytes = signer.private_key().to_bytes();
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?;
        let secp_public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = secp_public_key.serialize().to_vec();
        
        // Find a matching authentication key from the identity
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes).to_byte_array().to_vec()
        };
        
        let matching_key = identity
            .public_keys()
            .iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION &&
                key.key_type() == KeyType::ECDSA_HASH160 &&
                key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(_, key)| key)
            .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?;
        
        // Create a transfer transition
        let transition = BatchTransition::new_document_transfer_transition_from_document(
            document,
            document_type_ref,
            recipient_identifier,
            matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
                .map_err(|e| JsValue::from_str(&e))?,
            sdk.version(),
            None, // options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create transition: {}", e)))?;
        
        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();
        
        // Broadcast the state transition
        state_transition
            .broadcast(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast: {}", e)))?;
        
        // Return the result with document ID and new owner
        // Create result object
        let result_obj = js_sys::Object::new();
        
        // Set document ID
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("documentId"),
            &JsValue::from_str(&document_id),
        ).unwrap();
        
        // Set new owner
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("newOwnerId"),
            &JsValue::from_str(&recipient_id),
        ).unwrap();
        
        // Set transferred status
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("transferred"),
            &JsValue::from_bool(true),
        ).unwrap();
        
        Ok(result_obj.into())
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
    /// * `key_id` - The key ID to use for signing
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
        key_id: u32,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
            
        let doc_id = Identifier::from_string(&document_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid document ID: {}", e)))?;
            
        let buyer_identifier = Identifier::from_string(&buyer_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid buyer ID: {}", e)))?;
        
        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Get document type from contract
        let document_type_ref = data_contract
            .document_type_for_name(&document_type)
            .map_err(|e| JsValue::from_str(&format!("Document type not found: {}", e)))?;
        
        // Fetch the document to purchase
        let query = dash_sdk::platform::documents::document_query::DocumentQuery::new_with_data_contract_id(
            &sdk,
            contract_id,
            &document_type,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);
        
        let document = dash_sdk::platform::Document::fetch(&sdk, query)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch document: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Document not found"))?;
        
        // Verify the document has a price and it matches
        let listed_price = document
            .properties()
            .get_optional_integer::<u64>("$price")
            .map_err(|e| JsValue::from_str(&format!("Failed to get document price: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Document is not for sale (no price set)"))?;
        
        if listed_price != price {
            return Err(JsValue::from_str(&format!(
                "Price mismatch: document is listed for {} but purchase attempted with {}", 
                listed_price, price
            )));
        }
        
        // Fetch buyer identity
        let buyer_identity = dash_sdk::platform::Identity::fetch(&sdk, buyer_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch buyer identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Buyer identity not found"))?;
        
        // Parse private key
        let private_key_bytes = dash_sdk::dpp::dashcore::PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid private key: {}", e)))?
            .inner
            .secret_bytes();
        
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Find matching authentication key
        let matching_key = buyer_identity.public_keys().iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION &&
                key.key_type() == KeyType::ECDSA_HASH160 &&
                key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(_, key)| key)
            .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(buyer_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get identity contract nonce: {}", e)))?;
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
            .map_err(|e| JsValue::from_str(&e))?;
        
        // Create document purchase transition
        let transition = BatchTransition::new_document_purchase_transition_from_document(
            document.into(),
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
        .map_err(|e| JsValue::from_str(&format!("Failed to create purchase transition: {}", e)))?;
        
        // Broadcast the transition
        let proof_result = transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast purchase: {}", e)))?;
        
        // Handle the proof result
        match proof_result {
            StateTransitionProofResult::VerifiedDocuments(documents) => {
                // Document purchase was successful
                let result_obj = js_sys::Object::new();
                
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("documentId"), &JsValue::from_str(&doc_id.to_string(Encoding::Base58)))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set documentId: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("newOwnerId"), &JsValue::from_str(&buyer_id))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set newOwnerId: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("pricePaid"), &JsValue::from_f64(price as f64))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set pricePaid: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Document purchased successfully"))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
                
                // If we have the updated document in the response, include basic info
                if let Some((_, maybe_doc)) = documents.into_iter().next() {
                    if let Some(doc) = maybe_doc {
                        js_sys::Reflect::set(&result_obj, &JsValue::from_str("documentUpdated"), &JsValue::from_bool(true))
                            .map_err(|e| JsValue::from_str(&format!("Failed to set documentUpdated: {:?}", e)))?;
                        js_sys::Reflect::set(&result_obj, &JsValue::from_str("revision"), &JsValue::from_f64(doc.revision().unwrap_or(0) as f64))
                            .map_err(|e| JsValue::from_str(&format!("Failed to set revision: {:?}", e)))?;
                    }
                }
                
                Ok(result_obj.into())
            },
            _ => {
                // Purchase was processed but document not returned
                let result_obj = js_sys::Object::new();
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("documentId"), &JsValue::from_str(&doc_id.to_string(Encoding::Base58)))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set documentId: {:?}", e)))?;
                js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Document purchase processed"))
                    .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
                
                Ok(result_obj.into())
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
    /// * `key_id` - The key ID to use for signing
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
        key_id: u32,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_id = Identifier::from_string(&data_contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
            
        let doc_id = Identifier::from_string(&document_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid document ID: {}", e)))?;
            
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
        
        // Fetch the data contract
        let data_contract = dash_sdk::platform::DataContract::fetch(&sdk, contract_id)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch data contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Data contract not found"))?;
        
        // Get document type from contract
        let document_type_ref = data_contract
            .document_type_for_name(&document_type)
            .map_err(|e| JsValue::from_str(&format!("Document type not found: {}", e)))?;
        
        // Fetch the existing document to update its price
        let query = dash_sdk::platform::documents::document_query::DocumentQuery::new_with_data_contract_id(
            &sdk,
            contract_id,
            &document_type,
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to create document query: {}", e)))?
        .with_document_id(&doc_id);
        
        let existing_doc = Document::fetch(&sdk, query)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch document: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Document not found"))?;
        
        // Verify ownership
        if existing_doc.owner_id() != owner_identifier {
            return Err(JsValue::from_str("Only the document owner can set its price"));
        }
        
        // Get existing document properties and convert to mutable map
        let mut properties = existing_doc.properties().clone();
        
        // Update the price in the document properties
        let price_value = if price > 0 {
            PlatformValue::U64(price)
        } else {
            PlatformValue::Null
        };
        
        properties.insert("$price".to_string(), price_value);
        
        // Create updated document with new properties
        let new_revision = existing_doc.revision().unwrap_or(0) + 1;
        let updated_doc = Document::V0(DocumentV0 {
            id: doc_id,
            owner_id: owner_identifier,
            properties,
            revision: Some(new_revision),
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
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Identity not found"))?;
        
        // Get the private key and derive public key hash
        let private_key = PrivateKey::from_wif(&private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid WIF private key: {}", e)))?;
        
        let secp = dash_sdk::dpp::dashcore::secp256k1::Secp256k1::new();
        let private_key_bytes = private_key.inner.secret_bytes();
        let secret_key = dash_sdk::dpp::dashcore::secp256k1::SecretKey::from_slice(&private_key_bytes)
            .map_err(|e| JsValue::from_str(&format!("Invalid secret key: {}", e)))?;
        let public_key = dash_sdk::dpp::dashcore::secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let public_key_bytes = public_key.serialize();
        
        // Create public key hash using hash160
        let public_key_hash160 = {
            use dash_sdk::dpp::dashcore::hashes::{Hash, hash160};
            hash160::Hash::hash(&public_key_bytes[..]).to_byte_array().to_vec()
        };
        
        // Find matching authentication key
        let matching_key = identity.public_keys().iter()
            .find(|(_, key)| {
                key.purpose() == Purpose::AUTHENTICATION &&
                key.key_type() == KeyType::ECDSA_HASH160 &&
                key.data().as_slice() == public_key_hash160.as_slice()
            })
            .map(|(_, key)| key)
            .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?;
        
        // Get identity contract nonce
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(owner_identifier, contract_id, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch nonce: {}", e)))?;
        
        // Generate entropy for the state transition
        let entropy_bytes = {
            let mut entropy = [0u8; 32];
            if let Some(window) = web_sys::window() {
                if let Ok(crypto) = window.crypto() {
                    let _ = crypto.get_random_values_with_u8_array(&mut entropy);
                }
            }
            entropy
        };
        
        // Create the price update transition
        let transition = BatchTransition::new_document_replacement_transition_from_document(
            updated_doc,
            document_type_ref,
            matching_key,
            identity_contract_nonce,
            UserFeeIncrease::default(),
            None, // token_payment_info
            &SingleKeySigner::from_string(&private_key_wif, dash_sdk::dpp::dashcore::Network::Testnet)
                .map_err(|e| JsValue::from_str(&e))?,
            sdk.version(),
            None, // options
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create transition: {}", e)))?;
        
        // The transition is already signed, convert to StateTransition
        let state_transition: StateTransition = transition.into();
        
        // Broadcast the state transition
        state_transition
            .broadcast(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast: {}", e)))?;
        
        // Return the result with document ID and price
        let result_obj = js_sys::Object::new();
        
        // Set document ID
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("documentId"),
            &JsValue::from_str(&document_id),
        ).unwrap();
        
        // Set price
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("price"),
            &JsValue::from_f64(price as f64),
        ).unwrap();
        
        // Set price set status
        js_sys::Reflect::set(
            &result_obj,
            &JsValue::from_str("priceSet"),
            &JsValue::from_bool(true),
        ).unwrap();
        
        Ok(result_obj.into())
    }
}

/// Helper function to convert serde_json::Value to platform_value::Value
fn convert_json_to_platform_value(json_value: serde_json::Value) -> Result<PlatformValue, JsValue> {
    match json_value {
        serde_json::Value::Null => Ok(PlatformValue::Null),
        serde_json::Value::Bool(b) => Ok(PlatformValue::Bool(b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(PlatformValue::I64(i))
            } else if let Some(u) = n.as_u64() {
                Ok(PlatformValue::U64(u))
            } else if let Some(f) = n.as_f64() {
                Ok(PlatformValue::Float(f))
            } else {
                Err(JsValue::from_str("Unsupported number type"))
            }
        }
        serde_json::Value::String(s) => Ok(PlatformValue::Text(s)),
        serde_json::Value::Array(arr) => {
            let mut vec = Vec::new();
            for item in arr {
                vec.push(convert_json_to_platform_value(item)?);
            }
            Ok(PlatformValue::Array(vec))
        }
        serde_json::Value::Object(obj) => {
            let mut vec = Vec::new();
            for (key, value) in obj {
                let key_value = PlatformValue::Text(key);
                let converted_value = convert_json_to_platform_value(value)?;
                vec.push((key_value, converted_value));
            }
            Ok(PlatformValue::Map(vec))
        }
    }
}