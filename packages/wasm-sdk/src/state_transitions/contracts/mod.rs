use crate::sdk::WasmSdk;
use dash_sdk::dpp::identity::accessors::IdentityGettersV0;
use dash_sdk::dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dash_sdk::dpp::identity::{KeyType, Purpose};
use dash_sdk::dpp::platform_value::{Identifier, string_encoding::Encoding};
use dash_sdk::dpp::data_contract::DataContract;
use dash_sdk::dpp::data_contract::accessors::v0::DataContractV0Getters;
use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dash_sdk::dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dash_sdk::dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dash_sdk::platform::transition::broadcast::BroadcastStateTransition;
use dash_sdk::platform::transition::put_contract::PutContract;
use dash_sdk::platform::Fetch;
use simple_signer::SingleKeySigner;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys;
use std::collections::BTreeMap;

#[wasm_bindgen]
impl WasmSdk {
    /// Create a new data contract on Dash Platform.
    ///
    /// # Arguments
    ///
    /// * `owner_id` - The identity ID that will own the contract
    /// * `contract_definition` - JSON string containing the contract definition
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - Optional key ID to use for signing (if None, will auto-select)
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the created contract
    #[wasm_bindgen(js_name = contractCreate)]
    pub async fn contract_create(
        &self,
        owner_id: String,
        contract_definition: String,
        private_key_wif: String,
        key_id: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse owner identifier
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
        
        // Parse contract definition JSON
        let contract_json: serde_json::Value = serde_json::from_str(&contract_definition)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract definition JSON: {}", e)))?;
        
        // Fetch owner identity
        let owner_identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch owner identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Owner identity not found"))?;
        
        // Parse private key and find matching public key
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
        
        // Find matching key - prioritize key_id if provided, otherwise find any authentication key
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            owner_identity.public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    key.purpose() == Purpose::AUTHENTICATION &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| JsValue::from_str(&format!("Key with ID {} not found or doesn't match private key", requested_key_id)))?
                .clone()
        } else {
            // Find any matching authentication key
            owner_identity.public_keys().iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::AUTHENTICATION &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .map(|(_, key)| key.clone())
                .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?
        };
        
        // Create the data contract from JSON definition
        let data_contract = DataContract::from_json(
            contract_json,
            true, // validate
            sdk.version(),
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create data contract from JSON: {}", e)))?;
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(|e| JsValue::from_str(&e))?;
        
        // Create and broadcast the contract
        let created_contract = data_contract
            .put_to_platform_and_wait_for_response(&sdk, matching_key, &signer, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to create contract: {}", e)))?;
        
        // Create JavaScript result object
        let result_obj = js_sys::Object::new();
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        
        // Convert contract ID to base58
        let contract_id_base58 = created_contract.id().to_string(Encoding::Base58);
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("contractId"), &JsValue::from_str(&contract_id_base58))
            .map_err(|e| JsValue::from_str(&format!("Failed to set contractId: {:?}", e)))?;
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("ownerId"), &JsValue::from_str(&owner_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set ownerId: {:?}", e)))?;
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("version"), &JsValue::from_f64(created_contract.version() as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set version: {:?}", e)))?;
        
        // Add document type names
        let schema = created_contract.document_types();
        let doc_types_array = js_sys::Array::new();
        for (doc_type_name, _) in schema.iter() {
            doc_types_array.push(&JsValue::from_str(doc_type_name));
        }
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("documentTypes"), &doc_types_array)
            .map_err(|e| JsValue::from_str(&format!("Failed to set documentTypes: {:?}", e)))?;
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Data contract created successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
        Ok(result_obj.into())
    }
    
    /// Update an existing data contract on Dash Platform.
    ///
    /// # Arguments
    ///
    /// * `contract_id` - The ID of the contract to update
    /// * `owner_id` - The identity ID that owns the contract
    /// * `contract_updates` - JSON string containing the updated contract definition
    /// * `private_key_wif` - The private key in WIF format for signing
    /// * `key_id` - Optional key ID to use for signing (if None, will auto-select)
    ///
    /// # Returns
    ///
    /// Returns a Promise that resolves to a JsValue containing the update result
    #[wasm_bindgen(js_name = contractUpdate)]
    pub async fn contract_update(
        &self,
        contract_id: String,
        owner_id: String,
        contract_updates: String,
        private_key_wif: String,
        key_id: Option<u32>,
    ) -> Result<JsValue, JsValue> {
        let sdk = self.inner_clone();
        
        // Parse identifiers
        let contract_identifier = Identifier::from_string(&contract_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {}", e)))?;
            
        let owner_identifier = Identifier::from_string(&owner_id, Encoding::Base58)
            .map_err(|e| JsValue::from_str(&format!("Invalid owner ID: {}", e)))?;
        
        // Parse contract updates JSON
        let updates_json: serde_json::Value = serde_json::from_str(&contract_updates)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract updates JSON: {}", e)))?;
        
        // Fetch the existing contract
        let existing_contract = DataContract::fetch(&sdk, contract_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch contract: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Contract not found"))?;
        
        // Verify ownership
        if existing_contract.owner_id() != owner_identifier {
            return Err(JsValue::from_str("Identity does not own this contract"));
        }
        
        // Fetch owner identity
        let owner_identity = dash_sdk::platform::Identity::fetch(&sdk, owner_identifier)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to fetch owner identity: {}", e)))?
            .ok_or_else(|| JsValue::from_str("Owner identity not found"))?;
        
        // Parse private key and find matching public key
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
        
        // Find matching key - prioritize key_id if provided, otherwise find any authentication key
        let matching_key = if let Some(requested_key_id) = key_id {
            // Find specific key by ID
            owner_identity.public_keys()
                .get(&requested_key_id)
                .filter(|key| {
                    key.purpose() == Purpose::AUTHENTICATION &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .ok_or_else(|| JsValue::from_str(&format!("Key with ID {} not found or doesn't match private key", requested_key_id)))?
                .clone()
        } else {
            // Find any matching authentication key
            owner_identity.public_keys().iter()
                .find(|(_, key)| {
                    key.purpose() == Purpose::AUTHENTICATION &&
                    key.key_type() == KeyType::ECDSA_HASH160 &&
                    key.data().as_slice() == public_key_hash160.as_slice()
                })
                .map(|(_, key)| key.clone())
                .ok_or_else(|| JsValue::from_str("No matching authentication key found for the provided private key"))?
        };
        
        // Create updated contract from JSON definition
        // Note: The updates should be a complete contract definition with incremented version
        let updated_contract = DataContract::from_json(
            updates_json,
            true, // validate
            sdk.version(),
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create updated contract from JSON: {}", e)))?;
        
        // Verify the version was incremented
        if updated_contract.version() <= existing_contract.version() {
            return Err(JsValue::from_str(&format!(
                "Contract version must be incremented. Current: {}, Provided: {}", 
                existing_contract.version(), 
                updated_contract.version()
            )));
        }
        
        // Get identity nonce
        let identity_nonce = sdk
            .get_identity_nonce(owner_identifier, true, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to get identity nonce: {}", e)))?;
        
        // Create partial identity for signing
        let partial_identity = dash_sdk::dpp::identity::PartialIdentity {
            id: owner_identifier,
            loaded_public_keys: BTreeMap::from([(matching_key.id(), matching_key.clone())]),
            balance: None,
            revision: None,
            not_found_public_keys: Default::default(),
        };
        
        // Create signer
        let signer = SingleKeySigner::from_string(&private_key_wif, self.network())
            .map_err(|e| JsValue::from_str(&e))?;
        
        // Create the update transition
        let state_transition = DataContractUpdateTransition::new_from_data_contract(
            updated_contract.clone(),
            &partial_identity,
            matching_key.id(),
            identity_nonce,
            dash_sdk::dpp::prelude::UserFeeIncrease::default(),
            &signer,
            sdk.version(),
            None,
        )
        .map_err(|e| JsValue::from_str(&format!("Failed to create update transition: {}", e)))?;
        
        // Broadcast the transition
        use dash_sdk::dpp::state_transition::proof_result::StateTransitionProofResult;
        let result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(&sdk, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Failed to broadcast update: {}", e)))?;
        
        // Extract updated contract from result
        let updated_version = match result {
            StateTransitionProofResult::VerifiedDataContract(contract) => contract.version(),
            _ => updated_contract.version(),
        };
        
        // Create JavaScript result object
        let result_obj = js_sys::Object::new();
        
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("status"), &JsValue::from_str("success"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set status: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("contractId"), &JsValue::from_str(&contract_id))
            .map_err(|e| JsValue::from_str(&format!("Failed to set contractId: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("version"), &JsValue::from_f64(updated_version as f64))
            .map_err(|e| JsValue::from_str(&format!("Failed to set version: {:?}", e)))?;
        js_sys::Reflect::set(&result_obj, &JsValue::from_str("message"), &JsValue::from_str("Data contract updated successfully"))
            .map_err(|e| JsValue::from_str(&format!("Failed to set message: {:?}", e)))?;
        
        Ok(result_obj.into())
    }
}