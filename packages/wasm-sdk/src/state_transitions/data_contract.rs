//! Data contract state transitions
//!
//! This module provides WASM bindings for data contract-related state transitions including:
//! - Data contract creation and updates

use crate::error::to_js_error;
use dpp::data_contract::DataContract;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::conversion::value::v0::DataContractValueConversionMethodsV0;
use dpp::data_contract::accessors::v0::{DataContractV0Getters, DataContractV0Setters};
use dpp::version::{PlatformVersion, FeatureVersion};
use dpp::version::TryFromPlatformVersioned;
use platform_version::TryFromPlatformVersioned as TryFromPlatformVersionedTrait;
use dpp::identity::KeyID;
use dpp::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::data_contract_create_transition::{
    DataContractCreateTransition, DataContractCreateTransitionV0,
};
use dpp::state_transition::data_contract_update_transition::{
    DataContractUpdateTransition, DataContractUpdateTransitionV0,
};
use dpp::state_transition::StateTransition;
use platform_value::Value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;
use web_sys::js_sys::{Number, Uint8Array};

/// Create a new data contract
#[wasm_bindgen]
pub fn create_data_contract(
    owner_id: &str,
    contract_definition: JsValue,
    identity_nonce: u64,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    // Parse owner ID
    let owner_id = Identifier::from_string(
        owner_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

    // Parse contract definition
    let contract_value: Value = serde_wasm_bindgen::from_value(contract_definition)
        .map_err(|e| JsError::new(&format!("Failed to parse contract definition: {}", e)))?;

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

    // Parse the contract definition to extract document schemas
    let mut document_schemas = BTreeMap::new();
    let mut schema_defs = None;
    
    if let Ok(contract_map) = contract_value.into_btree_string_map() {
        // Extract document schemas from the "documents" field
        if let Some(Value::Map(docs)) = contract_map.get("documents") {
            for (key_val, doc_val) in docs {
                if let (Value::Text(doc_name), doc_schema) = (key_val, doc_val) {
                    document_schemas.insert(doc_name.clone(), doc_schema.clone());
                }
            }
        }
        
        // Extract schema definitions if present
        if let Some(defs) = contract_map.get("$defs") {
            if let Ok(defs_map) = defs.clone().into_btree_string_map() {
                schema_defs = Some(defs_map);
            }
        }
    }
    
    // Create the data contract using the factory
    let platform_version = PlatformVersion::latest();
    let factory = dpp::data_contract::factory::DataContractFactory::new(platform_version.protocol_version)
        .map_err(|e| JsError::new(&format!("Failed to create factory: {}", e)))?;
    
    // Create documents value
    let documents_value = Value::Map(
        document_schemas
            .into_iter()
            .map(|(k, v)| (Value::Text(k), v))
            .collect()
    );
    
    // Create definitions value if present
    let definitions_value = schema_defs.map(|defs| {
        Value::Map(
            defs.into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect()
        )
    });
    
    let created_contract = factory
        .create(
            owner_id,
            identity_nonce,
            documents_value,
            None, // config
            definitions_value,
        )
        .map_err(|e| JsError::new(&format!("Failed to create contract: {}", e)))?;
    
    let data_contract = created_contract.data_contract().clone();
    
    // Convert data contract to serialization format
    let data_contract_serialization = DataContractInSerializationFormat::try_from_platform_versioned(
        data_contract,
        &platform_version,
    )
    .map_err(|e| JsError::new(&format!("Failed to convert contract to serialization format: {}", e)))?;
    
    // Create the state transition
    let transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
        data_contract: data_contract_serialization,
        identity_nonce,
        user_fee_increase: 0,
        signature_public_key_id,
        signature: Default::default(),
    });
    
    let state_transition = StateTransition::DataContractCreate(transition);
    
    // Serialize the state transition
    let bytes = state_transition
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Update an existing data contract
#[wasm_bindgen]
pub fn update_data_contract(
    contract_id: &str,
    owner_id: &str,
    contract_definition: JsValue,
    identity_contract_nonce: u64,
    signature_public_key_id: Number,
) -> Result<Uint8Array, JsError> {
    // Parse identifiers
    let contract_id = Identifier::from_string(
        contract_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let owner_id = Identifier::from_string(
        owner_id,
        platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

    // Parse contract definition
    let contract_value: Value = serde_wasm_bindgen::from_value(contract_definition)
        .map_err(|e| JsError::new(&format!("Failed to parse contract definition: {}", e)))?;

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

    // Parse the contract definition to extract document schemas
    let mut document_schemas = BTreeMap::new();
    let mut schema_defs = None;
    
    if let Ok(contract_map) = contract_value.into_btree_string_map() {
        // Extract document schemas from the "documents" field
        if let Some(Value::Map(docs)) = contract_map.get("documents") {
            for (key_val, doc_val) in docs {
                if let (Value::Text(doc_name), doc_schema) = (key_val, doc_val) {
                    document_schemas.insert(doc_name.clone(), doc_schema.clone());
                }
            }
        }
        
        // Extract schema definitions if present
        if let Some(defs) = contract_map.get("$defs") {
            if let Ok(defs_map) = defs.clone().into_btree_string_map() {
                schema_defs = Some(defs_map);
            }
        }
    }
    
    // Create the updated data contract using the factory
    let platform_version = PlatformVersion::latest();
    let factory = dpp::data_contract::factory::DataContractFactory::new(platform_version.protocol_version)
        .map_err(|e| JsError::new(&format!("Failed to create factory: {}", e)))?;
    
    // Create documents value
    let documents_value = Value::Map(
        document_schemas
            .into_iter()
            .map(|(k, v)| (Value::Text(k), v))
            .collect()
    );
    
    // Create definitions value if present
    let definitions_value = schema_defs.map(|defs| {
        Value::Map(
            defs.into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect()
        )
    });
    
    // For updates, we need to create a contract with the existing ID
    // First create it normally, then update the ID
    let created_contract = factory
        .create(
            owner_id,
            identity_contract_nonce,
            documents_value,
            None, // config
            definitions_value,
        )
        .map_err(|e| JsError::new(&format!("Failed to create contract: {}", e)))?;
    
    let mut data_contract = created_contract.data_contract().clone();
    
    // Update the contract ID to match the existing contract
    match &mut data_contract {
        DataContract::V0(ref mut v0) => v0.set_id(contract_id),
        DataContract::V1(ref mut v1) => v1.id = contract_id,
    }
    
    // Increment the version for update
    match &mut data_contract {
        DataContract::V0(ref mut v0) => v0.increment_version(),
        DataContract::V1(ref mut v1) => v1.version += 1,
    }
    
    // Convert data contract to serialization format
    let data_contract_serialization = DataContractInSerializationFormat::try_from_platform_versioned(
        data_contract,
        &platform_version,
    )
    .map_err(|e| JsError::new(&format!("Failed to convert contract to serialization format: {}", e)))?;
    
    // Create the state transition
    let transition = DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
        data_contract: data_contract_serialization,
        identity_contract_nonce,
        user_fee_increase: 0,
        signature_public_key_id,
        signature: Default::default(),
    });
    
    let state_transition = StateTransition::DataContractUpdate(transition);
    
    // Serialize the state transition
    let bytes = state_transition
        .serialize_to_bytes()
        .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))?;
    
    Ok(Uint8Array::from(&bytes[..]))
}

/// Builder for creating data contract transitions
#[wasm_bindgen]
pub struct DataContractTransitionBuilder {
    owner_id: Identifier,
    contract_id: Option<Identifier>,
    contract_definition: BTreeMap<String, Value>,
    version: u32,
    user_fee_increase: UserFeeIncrease,
    identity_nonce: IdentityNonce,
    identity_contract_nonce: IdentityNonce,
}

#[wasm_bindgen]
impl DataContractTransitionBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(owner_id: &str) -> Result<DataContractTransitionBuilder, JsError> {
        let owner_id = Identifier::from_string(
            owner_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid owner ID: {}", e)))?;

        Ok(DataContractTransitionBuilder {
            owner_id,
            contract_id: None,
            contract_definition: BTreeMap::new(),
            version: 1,
            user_fee_increase: 0,
            identity_nonce: 0,
            identity_contract_nonce: 0,
        })
    }

    #[wasm_bindgen(js_name = setContractId)]
    pub fn set_contract_id(&mut self, contract_id: &str) -> Result<(), JsError> {
        let id = Identifier::from_string(
            contract_id,
            platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| JsError::new(&format!("Invalid contract ID: {}", e)))?;

        self.contract_id = Some(id);
        Ok(())
    }

    #[wasm_bindgen(js_name = setVersion)]
    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    #[wasm_bindgen(js_name = setUserFeeIncrease)]
    pub fn set_user_fee_increase(&mut self, fee_increase: u16) {
        self.user_fee_increase = fee_increase;
    }

    #[wasm_bindgen(js_name = setIdentityNonce)]
    pub fn set_identity_nonce(&mut self, nonce: u64) {
        self.identity_nonce = nonce;
    }

    #[wasm_bindgen(js_name = setIdentityContractNonce)]
    pub fn set_identity_contract_nonce(&mut self, nonce: u64) {
        self.identity_contract_nonce = nonce;
    }

    #[wasm_bindgen(js_name = addDocumentSchema)]
    pub fn add_document_schema(
        &mut self,
        document_type: &str,
        schema: JsValue,
    ) -> Result<(), JsError> {
        let schema_value: Value = serde_wasm_bindgen::from_value(schema)
            .map_err(|e| JsError::new(&format!("Failed to parse document schema: {}", e)))?;

        // Initialize documents object if it doesn't exist
        if !self.contract_definition.contains_key("documents") {
            self.contract_definition
                .insert("documents".to_string(), Value::Map(vec![]));
        }

        // Add the document schema
        if let Some(Value::Map(documents)) = self.contract_definition.get_mut("documents") {
            documents.push((Value::Text(document_type.to_string()), schema_value));
        }

        Ok(())
    }

    #[wasm_bindgen(js_name = setContractDefinition)]
    pub fn set_contract_definition(&mut self, definition: JsValue) -> Result<(), JsError> {
        let definition_value: Value = serde_wasm_bindgen::from_value(definition)
            .map_err(|e| JsError::new(&format!("Failed to parse contract definition: {}", e)))?;

        self.contract_definition = definition_value
            .into_btree_string_map()
            .map_err(|e| JsError::new(&format!("Contract definition must be an object: {}", e)))?;

        Ok(())
    }

    #[wasm_bindgen(js_name = buildCreateTransition)]
    pub fn build_create_transition(
        self,
        signature_public_key_id: Number,
    ) -> Result<Uint8Array, JsError> {
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

        // Parse the contract definition to extract document schemas
        let mut document_schemas = BTreeMap::new();
        let mut schema_defs = None;
        
        // Extract document schemas from the "documents" field
        if let Some(Value::Map(docs)) = self.contract_definition.get("documents") {
            for (key_val, doc_val) in docs {
                if let (Value::Text(doc_name), doc_schema) = (key_val, doc_val) {
                    document_schemas.insert(doc_name.clone(), doc_schema.clone());
                }
            }
        }
        
        // Extract schema definitions if present
        if let Some(defs) = self.contract_definition.get("$defs") {
            if let Ok(defs_map) = defs.clone().into_btree_string_map() {
                schema_defs = Some(defs_map);
            }
        }
        
        // Create the data contract using the factory
        let platform_version = PlatformVersion::latest();
        let factory = dpp::data_contract::factory::DataContractFactory::new(platform_version.protocol_version)
            .map_err(|e| JsError::new(&format!("Failed to create factory: {}", e)))?;
        
        // Create documents value
        let documents_value = Value::Map(
            document_schemas
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect()
        );
        
        // Create definitions value if present
        let definitions_value = schema_defs.map(|defs| {
            Value::Map(
                defs.into_iter()
                    .map(|(k, v)| (Value::Text(k), v))
                    .collect()
            )
        });
        
        let created_contract = factory
            .create(
                self.owner_id,
                self.identity_nonce,
                documents_value,
                None, // config
                definitions_value,
            )
            .map_err(|e| JsError::new(&format!("Failed to create contract: {}", e)))?;
        
        let data_contract = created_contract.data_contract().clone();
        
        // Convert data contract to serialization format
        let data_contract_serialization = DataContractInSerializationFormat::try_from_platform_versioned(
            data_contract,
            &platform_version,
        )
        .map_err(|e| JsError::new(&format!("Failed to convert contract to serialization format: {}", e)))?;
        
        // Create the state transition
        let transition = DataContractCreateTransition::V0(DataContractCreateTransitionV0 {
            data_contract: data_contract_serialization,
            identity_nonce: self.identity_nonce,
            user_fee_increase: self.user_fee_increase,
            signature_public_key_id,
            signature: Default::default(),
        });
        
        let state_transition = StateTransition::DataContractCreate(transition);
        
        // Serialize the state transition
        let bytes = state_transition
            .serialize_to_bytes()
            .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))?;
        
        Ok(Uint8Array::from(&bytes[..]))
    }

    #[wasm_bindgen(js_name = buildUpdateTransition)]
    pub fn build_update_transition(
        self,
        signature_public_key_id: Number,
    ) -> Result<Uint8Array, JsError> {
        let contract_id = self
            .contract_id
            .ok_or_else(|| JsError::new("Contract ID must be set for update transition"))?;

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

        // Parse the contract definition to extract document schemas
        let mut document_schemas = BTreeMap::new();
        let mut schema_defs = None;
        
        // Extract document schemas from the "documents" field
        if let Some(Value::Map(docs)) = self.contract_definition.get("documents") {
            for (key_val, doc_val) in docs {
                if let (Value::Text(doc_name), doc_schema) = (key_val, doc_val) {
                    document_schemas.insert(doc_name.clone(), doc_schema.clone());
                }
            }
        }
        
        // Extract schema definitions if present
        if let Some(defs) = self.contract_definition.get("$defs") {
            if let Ok(defs_map) = defs.clone().into_btree_string_map() {
                schema_defs = Some(defs_map);
            }
        }
        
        // Create the updated data contract using the factory
        let platform_version = PlatformVersion::latest();
        let factory = dpp::data_contract::factory::DataContractFactory::new(platform_version.protocol_version)
            .map_err(|e| JsError::new(&format!("Failed to create factory: {}", e)))?;
        
        // Create documents value
        let documents_value = Value::Map(
            document_schemas
                .into_iter()
                .map(|(k, v)| (Value::Text(k), v))
                .collect()
        );
        
        // Create definitions value if present
        let definitions_value = schema_defs.map(|defs| {
            Value::Map(
                defs.into_iter()
                    .map(|(k, v)| (Value::Text(k), v))
                    .collect()
            )
        });
        
        // For updates, we need to create a contract with the existing ID
        // First create it normally, then update the ID
        let created_contract = factory
            .create(
                self.owner_id,
                self.identity_contract_nonce,
                documents_value,
                None, // config
                definitions_value,
            )
            .map_err(|e| JsError::new(&format!("Failed to create contract: {}", e)))?;
        
        let mut data_contract = created_contract.data_contract().clone();
        
        // Update the contract ID to match the existing contract
        match &mut data_contract {
            DataContract::V0(ref mut v0) => {
                v0.set_id(contract_id);
                v0.set_version(self.version);
            },
            DataContract::V1(ref mut v1) => {
                v1.id = contract_id;
                v1.version = self.version;
            },
        }
        
        // Convert data contract to serialization format
        let data_contract_serialization = DataContractInSerializationFormat::try_from_platform_versioned(
            data_contract,
            &platform_version,
        )
        .map_err(|e| JsError::new(&format!("Failed to convert contract to serialization format: {}", e)))?;
        
        // Create the state transition
        let transition = DataContractUpdateTransition::V0(DataContractUpdateTransitionV0 {
            data_contract: data_contract_serialization,
            identity_contract_nonce: self.identity_contract_nonce,
            user_fee_increase: self.user_fee_increase,
            signature_public_key_id,
            signature: Default::default(),
        });
        
        let state_transition = StateTransition::DataContractUpdate(transition);
        
        // Serialize the state transition
        let bytes = state_transition
            .serialize_to_bytes()
            .map_err(|e| JsError::new(&format!("Failed to serialize state transition: {}", e)))?;
        
        Ok(Uint8Array::from(&bytes[..]))
    }
}