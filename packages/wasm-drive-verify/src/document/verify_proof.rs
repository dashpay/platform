use crate::utils::bounds::{check_array_bounds, check_object_bounds};
use crate::utils::error::{
    format_error, format_error_with_context, format_result_error, format_result_error_with_context,
    ErrorCategory,
};
use crate::utils::getters::VecU8ToUint8Array;
use crate::utils::platform_version::get_platform_version_with_validation;
use crate::utils::serialization::document_to_js_value;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use drive::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use indexmap::IndexMap;
use js_sys::{Array, Object, Reflect, Uint8Array};
use serde_wasm_bindgen::from_value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyDocumentProofResult {
    root_hash: Vec<u8>,
    documents: JsValue,
}

#[wasm_bindgen]
impl VerifyDocumentProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn documents(&self) -> JsValue {
        self.documents.clone()
    }
}

#[wasm_bindgen(js_name = "verifyDocumentProof")]
pub fn verify_document_proof(
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
) -> Result<VerifyDocumentProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = get_platform_version_with_validation(platform_version_number)?;

    // For now, we need the contract to be provided as CBOR bytes through contract_js
    // This is a limitation until we have proper JS serialization for DataContract
    let contract_bytes: Vec<u8> = if contract_js.is_instance_of::<Uint8Array>() {
        let array: Uint8Array = contract_js.clone().dyn_into().map_err(|_| {
            format_error(
                ErrorCategory::ConversionError,
                "contract must be Uint8Array",
            )
        })?;
        array.to_vec()
    } else {
        return Err(format_error(
            ErrorCategory::InvalidInput,
            "contract must be provided as Uint8Array (CBOR bytes)",
        ));
    };

    let contract = DataContract::versioned_deserialize(&contract_bytes, true, platform_version)
        .map_err(|e| format_result_error(ErrorCategory::DeserializationError, e))?;

    // Get document type
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| {
            format_result_error_with_context(ErrorCategory::NotFoundError, document_type_name, e)
        })?;

    // Parse where clauses
    let internal_clauses = parse_internal_clauses(where_clauses)?;

    // Parse order by
    let order_by_map = parse_order_by(order_by)?;

    // Parse start_at
    let start_at_bytes = start_at.map(|arr| {
        let vec = arr.to_vec();
        let bytes: [u8; 32] = vec
            .try_into()
            .map_err(|_| format_error(ErrorCategory::InvalidInput, "start_at must be 32 bytes"))
            .unwrap();
        bytes
    });

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

    let (root_hash, documents) = query
        .verify_proof(&proof_vec, platform_version)
        .map_err(|e| format_result_error(ErrorCategory::VerificationError, e))?;

    // Convert documents to JS array
    let js_array = Array::new();
    for doc in documents {
        // Convert document to JS value
        let doc_js = document_to_js_value(doc)?;
        js_array.push(&doc_js);
    }

    Ok(VerifyDocumentProofResult {
        root_hash: root_hash.to_vec(),
        documents: js_array.into(),
    })
}

fn parse_internal_clauses(where_clauses: &JsValue) -> Result<InternalClauses, JsValue> {
    if where_clauses.is_null() || where_clauses.is_undefined() {
        return Ok(InternalClauses::default());
    }

    let obj: Object = where_clauses.clone().dyn_into().map_err(|_| {
        format_error(
            ErrorCategory::InvalidInput,
            "where_clauses must be an object",
        )
    })?;

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
            let clauses_obj: Object = clauses.dyn_into().map_err(|_| {
                format_error(
                    ErrorCategory::InvalidInput,
                    "equal_clauses must be an object",
                )
            })?;

            let keys = Object::keys(&clauses_obj);
            check_object_bounds(keys.length() as usize, "equal_clauses")?;
            let mut equal_clauses = BTreeMap::new();

            for i in 0..keys.length() {
                let key = keys.get(i);
                let key_str = key.as_string().ok_or_else(|| {
                    format_error(ErrorCategory::InvalidInput, "object key must be a string")
                })?;

                let clause = Reflect::get(&clauses_obj, &key).map_err(|_| {
                    format_error(
                        ErrorCategory::InvalidInput,
                        "failed to get clause from object",
                    )
                })?;

                equal_clauses.insert(key_str, parse_where_clause(&clause)?);
            }

            internal_clauses.equal_clauses = equal_clauses;
        }
    }

    Ok(internal_clauses)
}

fn parse_where_clause(clause_js: &JsValue) -> Result<WhereClause, JsValue> {
    let obj: Object = clause_js.clone().dyn_into().map_err(|_| {
        format_error(
            ErrorCategory::InvalidInput,
            "where clause must be an object",
        )
    })?;

    let field = Reflect::get(&obj, &JsValue::from_str("field"))
        .map_err(|_| {
            format_error(
                ErrorCategory::InvalidInput,
                "where clause missing field property",
            )
        })?
        .as_string()
        .ok_or_else(|| format_error(ErrorCategory::InvalidInput, "field must be a string"))?;

    let operator_str = Reflect::get(&obj, &JsValue::from_str("operator"))
        .map_err(|_| {
            format_error(
                ErrorCategory::InvalidInput,
                "where clause missing operator property",
            )
        })?
        .as_string()
        .ok_or_else(|| format_error(ErrorCategory::InvalidInput, "operator must be a string"))?;

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
            return Err(format_error_with_context(
                ErrorCategory::InvalidInput,
                "operator",
                &operator_str,
            ))
        }
    };

    let value_js = Reflect::get(&obj, &JsValue::from_str("value")).map_err(|_| {
        format_error(
            ErrorCategory::InvalidInput,
            "where clause missing value property",
        )
    })?;

    // Check bounds if value is an array
    if let Some(array) = value_js.dyn_ref::<js_sys::Array>() {
        check_array_bounds(array.length() as usize, "where clause value")?;
    }

    let value: Value = from_value(value_js)
        .map_err(|e| format_result_error(ErrorCategory::DeserializationError, e))?;

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
        .map_err(|_| format_error(ErrorCategory::InvalidInput, "order_by must be an object"))?;

    let keys = Object::keys(&obj);

    for i in 0..keys.length() {
        let key = keys.get(i);
        let key_str = key.as_string().ok_or_else(|| {
            format_error(ErrorCategory::InvalidInput, "object key must be a string")
        })?;

        let clause_js = Reflect::get(&obj, &key)
            .map_err(|_| format_error(ErrorCategory::InvalidInput, "failed to get order clause"))?;

        let clause_obj: Object = clause_js.dyn_into().map_err(|_| {
            format_error(
                ErrorCategory::InvalidInput,
                "order clause must be an object",
            )
        })?;

        let field = Reflect::get(&clause_obj, &JsValue::from_str("field"))
            .map_err(|_| {
                format_error(
                    ErrorCategory::InvalidInput,
                    "order clause missing field property",
                )
            })?
            .as_string()
            .ok_or_else(|| format_error(ErrorCategory::InvalidInput, "field must be a string"))?;

        let ascending = Reflect::get(&clause_obj, &JsValue::from_str("ascending"))
            .map_err(|_| {
                format_error(
                    ErrorCategory::InvalidInput,
                    "order clause missing ascending property",
                )
            })?
            .as_bool()
            .ok_or_else(|| {
                format_error(ErrorCategory::InvalidInput, "ascending must be a boolean")
            })?;

        order_by_map.insert(key_str, OrderClause { field, ascending });
    }

    Ok(order_by_map)
}
