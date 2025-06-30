//! Document state transitions
//!
//! This module provides WASM bindings for document-related state transitions including:
//! - Document creation, updates, and deletion
//! - Document batch operations

use crate::error::to_js_error;
use dpp::identity::KeyID;
use dpp::prelude::{Identifier, UserFeeIncrease};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::{BatchTransition, BatchTransitionV0};
use dpp::state_transition::StateTransition;
use platform_value::Value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Number, Uint8Array};

/// Create a simple document batch transition
///
/// Note: This is a simplified implementation that creates a minimal batch transition.
/// In production, you would need to properly construct the document transitions.
#[wasm_bindgen]
pub fn create_document_batch_transition(
    owner_id: &str,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    // Parse owner ID
    let owner_id =
        Identifier::from_string(owner_id, platform_value::string_encoding::Encoding::Base58)
            .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

    // Parse signature public key ID
    let signature_public_key_id = signature_public_key_id
        .as_f64()
        .ok_or_else(|| JsError::new("signature_public_key_id must be a number"))?;

    let signature_public_key_id: KeyID = if signature_public_key_id.is_finite()
        && signature_public_key_id >= KeyID::MIN as f64
        && signature_public_key_id <= (KeyID::MAX as f64)
    {
        signature_public_key_id as KeyID
    } else {
        return Err(JsError::new(&format!(
            "signature_public_key_id {} out of valid range",
            signature_public_key_id
        )));
    };

    // Create a minimal batch transition
    // Note: In production, you would add actual document transitions here
    let batch_transition = BatchTransition::V0(BatchTransitionV0 {
        owner_id,
        transitions: vec![],
        user_fee_increase: 0,
        signature_public_key_id,
        signature: Default::default(),
    });

    // Serialize the transition
    StateTransition::Batch(batch_transition)
        .serialize_to_bytes()
        .map_err(to_js_error)
        .map(|bytes| Uint8Array::from(bytes.as_slice()))
}

/// Document transition builder for WASM
///
/// This is a simplified builder that helps construct document batch transitions.
#[wasm_bindgen]
pub struct DocumentBatchBuilder {
    owner_id: Identifier,
    transitions: Vec<Value>, // Simplified - store as Values
    user_fee_increase: UserFeeIncrease,
}

#[wasm_bindgen]
impl DocumentBatchBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(owner_id: &str) -> Result<DocumentBatchBuilder, JsError> {
        let owner_id =
            Identifier::from_string(owner_id, platform_value::string_encoding::Encoding::Base58)
                .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

        Ok(DocumentBatchBuilder {
            owner_id,
            transitions: vec![],
            user_fee_increase: 0,
        })
    }

    #[wasm_bindgen(js_name = setUserFeeIncrease)]
    pub fn set_user_fee_increase(&mut self, fee_increase: u16) {
        self.user_fee_increase = fee_increase;
    }

    #[wasm_bindgen(js_name = addCreateDocument)]
    pub fn add_create_document(
        &mut self,
        contract_id: &str,
        document_type: &str,
        data: JsValue,
        entropy: Vec<u8>,
    ) -> Result<(), JsError> {
        // Validate entropy
        let entropy_array: [u8; 32] = entropy
            .try_into()
            .map_err(|_| JsError::new("Entropy must be exactly 32 bytes"))?;

        // Parse contract ID
        let contract_id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

        // Convert JS data to Value
        let data_value: Value = serde_wasm_bindgen::from_value(data)
            .map_err(|e| JsError::new(&format!("Failed to parse document data: {}", e)))?;

        // Create a transition object as a Value
        let mut transition = BTreeMap::new();
        transition.insert(
            "$type".to_string(),
            Value::Text("documentCreate".to_string()),
        );
        transition.insert(
            "$dataContractId".to_string(),
            Value::Bytes(contract_id.to_vec()),
        );
        transition.insert(
            "$documentType".to_string(),
            Value::Text(document_type.to_string()),
        );
        transition.insert("$entropy".to_string(), Value::Bytes(entropy_array.to_vec()));

        // Add data fields
        if let Value::Map(data_map) = data_value {
            for (key, value) in data_map {
                if let Value::Text(key_str) = key {
                    transition.insert(key_str, value);
                }
            }
        }

        self.transitions.push(Value::Map(
            transition
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        ));
        Ok(())
    }

    #[wasm_bindgen(js_name = addDeleteDocument)]
    pub fn add_delete_document(
        &mut self,
        contract_id: &str,
        document_type: &str,
        document_id: &str,
    ) -> Result<(), JsError> {
        // Parse identifiers
        let contract_id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

        let document_id = Identifier::from_string(
            document_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid document ID: {}", e)))?;

        // Create a transition object as a Value
        let mut transition = BTreeMap::new();
        transition.insert(
            "$type".to_string(),
            Value::Text("documentDelete".to_string()),
        );
        transition.insert(
            "$dataContractId".to_string(),
            Value::Bytes(contract_id.to_vec()),
        );
        transition.insert(
            "$documentType".to_string(),
            Value::Text(document_type.to_string()),
        );
        transition.insert("$id".to_string(), Value::Bytes(document_id.to_vec()));

        self.transitions.push(Value::Map(
            transition
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        ));
        Ok(())
    }

    #[wasm_bindgen(js_name = addReplaceDocument)]
    pub fn add_replace_document(
        &mut self,
        contract_id: &str,
        document_type: &str,
        document_id: &str,
        revision: u32,
        data: JsValue,
    ) -> Result<(), JsError> {
        // Parse identifiers
        let contract_id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

        let document_id = Identifier::from_string(
            document_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid document ID: {}", e)))?;

        // Convert JS data to Value
        let data_value: Value = serde_wasm_bindgen::from_value(data)
            .map_err(|e| JsError::new(&format!("Failed to parse document data: {}", e)))?;

        // Create a transition object as a Value
        let mut transition = BTreeMap::new();
        transition.insert(
            "$type".to_string(),
            Value::Text("documentReplace".to_string()),
        );
        transition.insert(
            "$dataContractId".to_string(),
            Value::Bytes(contract_id.to_vec()),
        );
        transition.insert(
            "$documentType".to_string(),
            Value::Text(document_type.to_string()),
        );
        transition.insert("$id".to_string(), Value::Bytes(document_id.to_vec()));
        transition.insert("$revision".to_string(), Value::U32(revision));

        // Add data fields
        if let Value::Map(data_map) = data_value {
            for (key, value) in data_map {
                if let Value::Text(key_str) = key {
                    transition.insert(key_str, value);
                }
            }
        }

        self.transitions.push(Value::Map(
            transition
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect(),
        ));
        Ok(())
    }

    #[wasm_bindgen]
    pub fn build(self, signature_public_key_id: Number) -> Result<Uint8Array, JsError> {
        if self.transitions.is_empty() {
            return Err(JsError::new("No transitions added to the builder"));
        }

        // Parse signature public key ID
        let signature_public_key_id = signature_public_key_id
            .as_f64()
            .ok_or_else(|| JsError::new("signature_public_key_id must be a number"))?;

        let signature_public_key_id: KeyID = if signature_public_key_id.is_finite()
            && signature_public_key_id >= KeyID::MIN as f64
            && signature_public_key_id <= (KeyID::MAX as f64)
        {
            signature_public_key_id as KeyID
        } else {
            return Err(JsError::new(&format!(
                "signature_public_key_id {} out of valid range",
                signature_public_key_id
            )));
        };

        // For now, just create an empty batch transition
        // In production, you would properly convert the Value transitions to proper types
        let batch_transition = BatchTransition::V0(BatchTransitionV0 {
            owner_id: self.owner_id,
            transitions: vec![],
            user_fee_increase: self.user_fee_increase,
            signature_public_key_id,
            signature: Default::default(),
        });

        // Serialize the transition
        StateTransition::Batch(batch_transition)
            .serialize_to_bytes()
            .map_err(to_js_error)
            .map(|bytes| Uint8Array::from(bytes.as_slice()))
    }
}
