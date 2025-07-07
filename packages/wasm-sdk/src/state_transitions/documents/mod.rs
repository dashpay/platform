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
use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
use dash_sdk::dpp::ProtocolError;
use dash_sdk::dpp::document::{Document, DocumentV0Getters, DocumentV0};
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dash_sdk::dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::Fetch;
use serde_wasm_bindgen::to_value;
use serde_json;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys;
use js_sys;

/// A simple signer for WASM that uses a single private key
struct WasmSigner {
    private_key: PrivateKey,
}

impl WasmSigner {
    fn new(private_key_wif: &str) -> Result<Self, JsValue> {
        let private_key = PrivateKey::from_wif(private_key_wif)
            .map_err(|e| JsValue::from_str(&format!("Invalid WIF private key: {}", e)))?;
        Ok(Self { private_key })
    }
}

impl Signer for WasmSigner {
    fn sign(
        &self,
        _identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        use dash_sdk::dpp::dashcore::signer;
        let signature = signer::sign(data, &self.private_key.inner.secret_bytes())?;
        Ok(signature.to_vec().into())
    }

    fn can_sign_with(&self, _identity_public_key: &IdentityPublicKey) -> bool {
        // For simplicity, we assume the signer can sign with any key
        // In a real implementation, you'd check if the public key matches
        true
    }
}

impl std::fmt::Debug for WasmSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmSigner").finish()
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
        let private_key_bytes = WasmSigner::new(&private_key_wif)?.private_key.to_bytes();
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
        let signer = WasmSigner::new(&private_key_wif)?;
        
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
        key_id: u32,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Document replace not yet implemented"))
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
        key_id: u32,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Document delete not yet implemented"))
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
        key_id: u32,
    ) -> Result<JsValue, JsValue> {
        Err(JsValue::from_str("Document transfer not yet implemented"))
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
        Err(JsValue::from_str("Document purchase not yet implemented"))
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
        Err(JsValue::from_str("Document set price not yet implemented"))
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