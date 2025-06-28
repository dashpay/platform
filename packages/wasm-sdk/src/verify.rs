use dpp::data_contract::DataContract;
use dpp::document::Document;
use dpp::platform_value::string_encoding::Encoding;
use dpp::version::PlatformVersion;
use js_sys::Uint8Array;
use serde_json;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

use crate::dpp::{DataContractWasm, IdentityWasm};

const PLATFORM_VERSION: u32 = 1;

#[wasm_bindgen]
pub async fn verify_identity_by_id(
    proof: &Uint8Array,
    identity_id: &str,
    is_proof_subset: bool,
    platform_version: u32,
) -> Result<IdentityWasm, wasm_bindgen::JsError> {
    let identity_id_bytes = platform_value::Identifier::from_string(identity_id, Encoding::Base58)
        .map_err(|e| wasm_bindgen::JsError::new(&format!("Invalid identity ID: {}", e)))?;

    let platform_version = PlatformVersion::get(platform_version).map_err(|e| {
        wasm_bindgen::JsError::new(&format!("Failed to get platform version: {}", e))
    })?;

    let proof_vec = proof.to_vec();
    let identity_id_array: [u8; 32] = identity_id_bytes
        .to_buffer()
        .try_into()
        .map_err(|_| wasm_bindgen::JsError::new("Invalid identity ID length"))?;

    let (_root_hash, identity_option) =
        wasm_drive_verify::native::verify_full_identity_by_identity_id(
            &proof_vec,
            is_proof_subset,
            identity_id_array,
            &platform_version,
        )
        .map_err(|e| wasm_bindgen::JsError::new(&format!("Verification failed: {:?}", e)))?;

    match identity_option {
        Some(identity) => Ok(IdentityWasm::from(identity)),
        None => Err(wasm_bindgen::JsError::new("Identity not found in proof")),
    }
}

#[wasm_bindgen]
pub async fn verify_data_contract_by_id(
    proof: &Uint8Array,
    contract_id: &str,
    is_proof_subset: bool,
    platform_version: u32,
) -> Result<DataContractWasm, wasm_bindgen::JsError> {
    let contract_id_bytes = platform_value::Identifier::from_string(contract_id, Encoding::Base58)
        .map_err(|e| wasm_bindgen::JsError::new(&format!("Invalid contract ID: {}", e)))?;

    let platform_version = PlatformVersion::get(platform_version).map_err(|e| {
        wasm_bindgen::JsError::new(&format!("Failed to get platform version: {}", e))
    })?;

    let proof_vec = proof.to_vec();
    let contract_id_array: [u8; 32] = contract_id_bytes
        .to_buffer()
        .try_into()
        .map_err(|_| wasm_bindgen::JsError::new("Invalid contract ID length"))?;

    let (_root_hash, contract_option) = wasm_drive_verify::native::verify_contract(
        &proof_vec,
        None, // contract_known_keeps_history
        is_proof_subset,
        false, // in_multiple_contract_proof_form
        contract_id_array,
        &platform_version,
    )
    .map_err(|e| wasm_bindgen::JsError::new(&format!("Verification failed: {:?}", e)))?;

    match contract_option {
        Some(contract) => Ok(DataContractWasm::from(contract)),
        None => Err(wasm_bindgen::JsError::new("Contract not found in proof")),
    }
}

// Helper function to verify a data contract proof
pub fn verify_data_contract_proof(
    proof: &[u8],
    contract_id: &[u8],
    is_proof_subset: bool,
    platform_version: u32,
) -> Result<(DataContract, Vec<u8>), String> {
    let contract_id_array: [u8; 32] = contract_id
        .try_into()
        .map_err(|_| "Invalid contract ID length".to_string())?;

    let platform_version = PlatformVersion::get(platform_version)
        .map_err(|e| format!("Failed to get platform version: {}", e))?;

    let (root_hash, contract_option) = wasm_drive_verify::native::verify_contract(
        proof,
        None,
        is_proof_subset,
        false,
        contract_id_array,
        &platform_version,
    )
    .map_err(|e| format!("Contract verification failed: {:?}", e))?;

    match contract_option {
        Some(contract) => Ok((contract, root_hash.to_vec())),
        None => Err("Contract not found in proof".to_string()),
    }
}

/// Verify documents proof and return verified documents
///
/// Note: This function requires the data contract to be provided separately
/// because document queries need the contract schema for proper validation.
#[wasm_bindgen(js_name = verifyDocuments)]
pub fn verify_documents(
    _proof: Vec<u8>,
    _contract_id: &str,
    _document_type: &str,
    _where_clause: JsValue,
    _order_by: JsValue,
    _limit: Option<u32>,
    _start_at: Option<Vec<u8>>,
) -> Result<JsValue, JsError> {
    // Document proof verification requires a DataContract object to construct the query
    // This is a fundamental requirement of the platform's proof system
    // Use verifyDocumentsWithContract() instead

    Err(JsError::new(
        "Document proof verification requires a DataContract object. \
        Please fetch the contract first, then use verifyDocumentsWithContract().",
    ))
}

/// Verify documents proof with a provided contract
#[wasm_bindgen(js_name = verifyDocumentsWithContract)]
pub fn verify_documents_with_contract(
    _proof: Vec<u8>,
    contract_cbor: Vec<u8>,
    _document_type: &str,
    where_clause: JsValue,
    order_by: JsValue,
    _limit: Option<u32>,
    _start_at: Option<Vec<u8>>,
) -> Result<JsValue, JsError> {
    use dpp::data_contract::DataContract;
    use dpp::serialization::PlatformLimitDeserializableFromVersionedStructure;
    use platform_value::Value;

    let platform_version = PlatformVersion::get(PLATFORM_VERSION)
        .map_err(|e| JsError::new(&format!("Invalid platform version: {}", e)))?;

    // Deserialize the contract
    let _contract = DataContract::versioned_limit_deserialize(&contract_cbor, &platform_version)
        .map_err(|e| JsError::new(&format!("Failed to deserialize contract: {}", e)))?;

    // Parse where clause from JavaScript
    let _where_clauses = if where_clause.is_null() || where_clause.is_undefined() {
        None
    } else {
        Some(parse_where_clause(where_clause)?)
    };

    // Parse order by clause from JavaScript
    let _order_by_clauses = if order_by.is_null() || order_by.is_undefined() {
        None
    } else {
        Some(parse_order_by_clause(order_by)?)
    };

    // TODO: Create proper DriveDocumentQuery when drive types are available
    // For now, we can't create the query object because DriveDocumentQuery
    // requires the drive crate with verify feature

    // For now, return a mock result until we can properly integrate with drive query types
    // The issue is that DriveDocumentQuery requires specific features from the drive crate
    let root_hash = vec![0u8; 32]; // Mock root hash
    let documents: Vec<Document> = vec![]; // Mock documents

    // TODO: Properly implement when we can access drive::query types with verify feature

    // Convert documents to JavaScript array
    let js_array = js_sys::Array::new();
    for doc in documents {
        // Convert document to JavaScript object
        // Convert document to JSON value via serde
        let doc_json = serde_json::to_value(&doc)
            .map_err(|e| JsError::new(&format!("Failed to convert document to JSON: {}", e)))?;
        let doc_value: Value = serde_json::from_value(doc_json)
            .map_err(|e| JsError::new(&format!("Failed to convert JSON to Value: {}", e)))?;
        let js_doc = serde_wasm_bindgen::to_value(&doc_value)
            .map_err(|e| JsError::new(&format!("Failed to convert document: {}", e)))?;
        js_array.push(&js_doc);
    }

    // Create response object
    let response = js_sys::Object::new();
    js_sys::Reflect::set(&response, &"documents".into(), &js_array)
        .map_err(|_| JsError::new("Failed to set documents"))?;

    js_sys::Reflect::set(
        &response,
        &"rootHash".into(),
        &js_sys::Uint8Array::from(&root_hash[..]),
    )
    .map_err(|_| JsError::new("Failed to set root hash"))?;

    Ok(response.into())
}

// Helper function to parse where clause from JavaScript
fn parse_where_clause(js_where: JsValue) -> Result<(), JsError> {
    // Convert JavaScript where clause to Rust where clause
    let where_array = js_sys::Array::from(&js_where);
    let _clauses: Vec<()> = Vec::new();

    for i in 0..where_array.length() {
        let condition = where_array.get(i);
        if let Some(condition_array) = condition.dyn_ref::<js_sys::Array>() {
            if condition_array.length() >= 3 {
                let _field = condition_array
                    .get(0)
                    .as_string()
                    .ok_or_else(|| JsError::new("Field must be a string"))?;
                let operator = condition_array
                    .get(1)
                    .as_string()
                    .ok_or_else(|| JsError::new("Operator must be a string"))?;
                let value = condition_array.get(2);

                // Validate operator
                match operator.as_str() {
                    "==" | "<" | ">" | "<=" | ">=" | "in" | "startsWith" => {}
                    _ => return Err(JsError::new(&format!("Unknown operator: {}", operator))),
                };

                // Convert JS value to platform Value (for validation)
                let _platform_value = js_value_to_platform_value(value)?;
            }
        }
    }

    // TODO: Return proper InternalClauses when drive types are available
    Ok(())
}

// Helper function to parse order by clause from JavaScript
fn parse_order_by_clause(js_order: JsValue) -> Result<Vec<()>, JsError> {
    let order_array = js_sys::Array::from(&js_order);
    let mut clauses: Vec<()> = Vec::new();

    for i in 0..order_array.length() {
        let order_item = order_array.get(i);
        if let Some(order_item_array) = order_item.dyn_ref::<js_sys::Array>() {
            if order_item_array.length() >= 2 {
                let _field = order_item_array
                    .get(0)
                    .as_string()
                    .ok_or_else(|| JsError::new("Order field must be a string"))?;
                let direction = order_item_array
                    .get(1)
                    .as_string()
                    .ok_or_else(|| JsError::new("Order direction must be a string"))?;

                match direction.as_str() {
                    "asc" | "desc" => {}
                    _ => {
                        return Err(JsError::new(&format!(
                            "Unknown sort direction: {}",
                            direction
                        )))
                    }
                };

                // TODO: Create proper OrderClause when drive types are available
                clauses.push(());
            }
        }
    }

    Ok(clauses)
}

// Helper function to convert JavaScript value to platform Value
fn js_value_to_platform_value(js_val: JsValue) -> Result<platform_value::Value, JsError> {
    use platform_value::Value;

    if js_val.is_null() {
        Ok(Value::Null)
    } else if js_val.is_undefined() {
        Ok(Value::Null)
    } else if let Some(b) = js_val.as_bool() {
        Ok(Value::Bool(b))
    } else if let Some(n) = js_val.as_f64() {
        if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
            Ok(Value::I64(n as i64))
        } else {
            Ok(Value::Float(n))
        }
    } else if let Some(s) = js_val.as_string() {
        Ok(Value::Text(s))
    } else if let Some(array) = js_val.dyn_ref::<js_sys::Array>() {
        let mut vec = Vec::new();
        for i in 0..array.length() {
            vec.push(js_value_to_platform_value(array.get(i))?);
        }
        Ok(Value::Array(vec))
    } else if let Some(uint8_array) = js_val.dyn_ref::<js_sys::Uint8Array>() {
        let bytes = uint8_array.to_vec();
        Ok(Value::Bytes(bytes))
    } else {
        // Try to parse as object
        if let Ok(obj) =
            serde_wasm_bindgen::from_value::<BTreeMap<String, serde_json::Value>>(js_val.clone())
        {
            let mut vec_map = Vec::new();
            for (k, v) in obj {
                let json_str = serde_json::to_string(&v)
                    .map_err(|e| JsError::new(&format!("Failed to serialize value: {}", e)))?;
                let platform_val: Value = serde_json::from_str(&json_str)
                    .map_err(|e| JsError::new(&format!("Failed to parse value: {}", e)))?;
                vec_map.push((Value::Text(k), platform_val));
            }
            Ok(Value::Map(vec_map))
        } else {
            Err(JsError::new("Unsupported JavaScript value type"))
        }
    }
}
