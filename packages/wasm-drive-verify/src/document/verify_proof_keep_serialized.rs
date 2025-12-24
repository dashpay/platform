use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use drive::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use indexmap::IndexMap;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_wasm_bindgen::from_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyDocumentProofKeepSerializedResult {
    root_hash: Vec<u8>,
    serialized_documents: JsValue,
}

#[wasm_bindgen]
impl VerifyDocumentProofKeepSerializedResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn serialized_documents(&self) -> JsValue {
        self.serialized_documents.clone()
    }
}

#[wasm_bindgen(js_name = "verifyDocumentProofKeepSerialized")]
pub fn verify_document_proof_keep_serialized(
    proof: &Uint8Array,
    contract_js: &JsValue,
    document_type_name: &str,
    where_clauses: &JsValue,
    order_by: &JsValue,
    limit: Option<u16>,
    offset: Option<u16>,
    start_at: Option<Uint8Array>,
    start_at_included: bool,
    block_time_ms: Option<u64>,
    platform_version_number: u32,
) -> Result<VerifyDocumentProofKeepSerializedResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    // For now, we need the contract to be provided as CBOR bytes through contract_js
    // This is a limitation until we have proper JS serialization for DataContract
    let contract_bytes: Vec<u8> = if contract_js.is_instance_of::<Uint8Array>() {
        let array: Uint8Array = contract_js
            .clone()
            .dyn_into()
            .map_err(|_| JsValue::from_str("Failed to convert contract to Uint8Array"))?;
        array.to_vec()
    } else {
        return Err(JsValue::from_str(
            "Contract must be provided as Uint8Array (CBOR bytes)",
        ));
    };

    let contract = DataContract::versioned_deserialize(&contract_bytes, true, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;

    // Get document type
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| JsValue::from_str(&format!("Document type not found: {:?}", e)))?;

    // Parse where clauses
    let internal_clauses = parse_internal_clauses(where_clauses)?;

    // Parse order by
    let order_by_map = parse_order_by(order_by)?;

    // Parse start_at
    let start_at_bytes = if let Some(arr) = start_at {
        let vec = arr.to_vec();
        let bytes: [u8; 32] = vec
            .try_into()
            .map_err(|_| JsValue::from_str("Invalid start_at length. Expected 32 bytes."))?;
        Some(bytes)
    } else {
        None
    };

    // Create the query
    let query = DriveDocumentQuery {
        contract: &contract,
        document_type,
        internal_clauses,
        offset,
        limit,
        order_by: order_by_map,
        start_at: start_at_bytes,
        start_at_included,
        block_time_ms,
    };

    let (root_hash, serialized_docs) = query
        .verify_proof_keep_serialized(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert serialized documents to JS array of Uint8Arrays
    let js_array = Array::new();
    for doc_bytes in serialized_docs {
        let uint8_array = Uint8Array::from(&doc_bytes[..]);
        js_array.push(&uint8_array);
    }

    Ok(VerifyDocumentProofKeepSerializedResult {
        root_hash: root_hash.to_vec(),
        serialized_documents: js_array.into(),
    })
}

// Reuse the same parsing functions from verify_proof.rs
fn parse_internal_clauses(where_clauses: &JsValue) -> Result<InternalClauses, JsValue> {
    if where_clauses.is_null() || where_clauses.is_undefined() {
        return Ok(InternalClauses::default());
    }

    let obj: Object = where_clauses
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("where_clauses must be an object"))?;

    let mut internal_clauses = InternalClauses::default();

    // Parse primary_key_in_clause
    if let Ok(clause) = Reflect::get(&obj, &JsValue::from_str("primary_key_in_clause")) {
        if !clause.is_null() && !clause.is_undefined() {
            internal_clauses.primary_key_in_clause = Some(parse_where_clause(&clause)?);
        }
    }

    // Parse primary_key_equal_clause
    if let Ok(clause) = Reflect::get(&obj, &JsValue::from_str("primary_key_equal_clause")) {
        if !clause.is_null() && !clause.is_undefined() {
            internal_clauses.primary_key_equal_clause = Some(parse_where_clause(&clause)?);
        }
    }

    // Parse in_clause
    if let Ok(clause) = Reflect::get(&obj, &JsValue::from_str("in_clause")) {
        if !clause.is_null() && !clause.is_undefined() {
            internal_clauses.in_clause = Some(parse_where_clause(&clause)?);
        }
    }

    // Parse range_clause
    if let Ok(clause) = Reflect::get(&obj, &JsValue::from_str("range_clause")) {
        if !clause.is_null() && !clause.is_undefined() {
            internal_clauses.range_clause = Some(parse_where_clause(&clause)?);
        }
    }

    // Parse equal_clauses
    if let Ok(clauses) = Reflect::get(&obj, &JsValue::from_str("equal_clauses")) {
        if !clauses.is_null() && !clauses.is_undefined() {
            let clauses_obj: Object = clauses
                .dyn_into()
                .map_err(|_| JsValue::from_str("equal_clauses must be an object"))?;

            let keys = Object::keys(&clauses_obj);
            let mut equal_clauses = BTreeMap::new();

            for i in 0..keys.length() {
                let key = keys.get(i);
                let key_str = key
                    .as_string()
                    .ok_or_else(|| JsValue::from_str("Key must be a string"))?;

                let clause = Reflect::get(&clauses_obj, &key)
                    .map_err(|_| JsValue::from_str("Failed to get clause"))?;

                equal_clauses.insert(key_str, parse_where_clause(&clause)?);
            }

            internal_clauses.equal_clauses = equal_clauses;
        }
    }

    Ok(internal_clauses)
}

fn parse_where_clause(clause_js: &JsValue) -> Result<WhereClause, JsValue> {
    let obj: Object = clause_js
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("where clause must be an object"))?;

    let field = Reflect::get(&obj, &JsValue::from_str("field"))
        .map_err(|_| JsValue::from_str("Failed to get field"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("field must be a string"))?;

    let operator_str = Reflect::get(&obj, &JsValue::from_str("operator"))
        .map_err(|_| JsValue::from_str("Failed to get operator"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("operator must be a string"))?;

    let operator = match operator_str.as_str() {
        "Equal" => WhereOperator::Equal,
        "GreaterThan" => WhereOperator::GreaterThan,
        "GreaterThanOrEquals" => WhereOperator::GreaterThanOrEquals,
        "LessThan" => WhereOperator::LessThan,
        "LessThanOrEquals" => WhereOperator::LessThanOrEquals,
        "Between" => WhereOperator::Between,
        "BetweenExcludeBounds" => WhereOperator::BetweenExcludeBounds,
        "BetweenExcludeLeft" => WhereOperator::BetweenExcludeLeft,
        "BetweenExcludeRight" => WhereOperator::BetweenExcludeRight,
        "In" => WhereOperator::In,
        "StartsWith" => WhereOperator::StartsWith,
        _ => {
            return Err(JsValue::from_str(&format!(
                "Unknown operator: {}",
                operator_str
            )))
        }
    };

    let value_js = Reflect::get(&obj, &JsValue::from_str("value"))
        .map_err(|_| JsValue::from_str("Failed to get value"))?;

    let value: Value = from_value(value_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse value: {:?}", e)))?;

    Ok(WhereClause {
        field,
        operator,
        value,
    })
}

fn parse_order_by(order_by_js: &JsValue) -> Result<IndexMap<String, OrderClause>, JsValue> {
    let mut order_by_map = IndexMap::new();

    if order_by_js.is_null() || order_by_js.is_undefined() {
        return Ok(order_by_map);
    }

    let obj: Object = order_by_js
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("order_by must be an object"))?;

    let keys = Object::keys(&obj);

    for i in 0..keys.length() {
        let key = keys.get(i);
        let key_str = key
            .as_string()
            .ok_or_else(|| JsValue::from_str("Key must be a string"))?;

        let clause_js = Reflect::get(&obj, &key)
            .map_err(|_| JsValue::from_str("Failed to get order clause"))?;

        let clause_obj: Object = clause_js
            .dyn_into()
            .map_err(|_| JsValue::from_str("order clause must be an object"))?;

        let field = Reflect::get(&clause_obj, &JsValue::from_str("field"))
            .map_err(|_| JsValue::from_str("Failed to get field"))?
            .as_string()
            .ok_or_else(|| JsValue::from_str("field must be a string"))?;

        let ascending = Reflect::get(&clause_obj, &JsValue::from_str("ascending"))
            .map_err(|_| JsValue::from_str("Failed to get ascending"))?
            .as_bool()
            .ok_or_else(|| JsValue::from_str("ascending must be a boolean"))?;

        order_by_map.insert(key_str, OrderClause { field, ascending });
    }

    Ok(order_by_map)
}
