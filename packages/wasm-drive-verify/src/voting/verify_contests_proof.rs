use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::DataContract;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use drive::query::vote_polls_by_document_type_query::ResolvedVotePollsByDocumentTypeQuery;
use drive::util::object_size_info::DataContractResolvedInfo;
use js_sys::{Array, Uint8Array};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyContestsProofResult {
    root_hash: Vec<u8>,
    contests: Array,
}

#[wasm_bindgen]
impl VerifyContestsProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn contests(&self) -> Array {
        self.contests.clone()
    }
}

#[wasm_bindgen(js_name = "verifyContestsProof")]
pub fn verify_contests_proof(
    proof: &Uint8Array,
    contract_cbor: &Uint8Array,
    document_type_name: &str,
    index_name: &str,
    start_at_value: Option<Uint8Array>,
    start_index_values: Option<Array>,
    end_index_values: Option<Array>,
    limit: Option<u16>,
    order_ascending: bool,
    platform_version_number: u32,
) -> Result<VerifyContestsProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Deserialize the data contract
    let contract: DataContract = ciborium::de::from_reader(&contract_cbor.to_vec()[..])
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;
    let contract_arc = Arc::new(contract);

    // Parse start_at_value
    let start_at_value_parsed = start_at_value
        .map(|v| {
            let bytes = v.to_vec();
            let value = ciborium::de::from_reader::<Value, _>(&bytes[..]).map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize start_at_value: {:?}", e))
            })?;
            Ok::<(Value, bool), JsValue>((value, true)) // true means inclusive
        })
        .transpose()?;

    // Parse start_index_values
    let start_index_values_parsed = parse_index_values(start_index_values)?;

    // Parse end_index_values
    let end_index_values_parsed = parse_index_values(end_index_values)?;

    // Create the resolved query
    let query = ResolvedVotePollsByDocumentTypeQuery {
        contract: DataContractResolvedInfo::ArcDataContract(contract_arc.clone()),
        document_type_name: &document_type_name.to_string(),
        index_name: &index_name.to_string(),
        start_at_value: &start_at_value_parsed,
        start_index_values: &start_index_values_parsed.unwrap_or_default(),
        end_index_values: &end_index_values_parsed.unwrap_or_default(),
        limit,
        order_ascending,
    };

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, contests_vec) = query
        .verify_contests_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert Values to JS array
    let js_array = Array::new();
    for value in contests_vec {
        let mut value_bytes = Vec::new();
        ciborium::into_writer(&value, &mut value_bytes)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize value: {:?}", e)))?;
        let value_uint8 = Uint8Array::from(&value_bytes[..]);
        js_array.push(&value_uint8);
    }

    Ok(VerifyContestsProofResult {
        root_hash: root_hash.to_vec(),
        contests: js_array,
    })
}

fn parse_index_values(values: Option<Array>) -> Result<Option<Vec<Value>>, JsValue> {
    values
        .map(|arr| {
            let mut result = Vec::new();
            for i in 0..arr.length() {
                let value_js = arr.get(i);
                let value_uint8: Uint8Array = value_js
                    .dyn_into()
                    .map_err(|_| JsValue::from_str("Index value must be a Uint8Array"))?;

                let value_bytes = value_uint8.to_vec();
                let value: Value = ciborium::de::from_reader(&value_bytes[..]).map_err(|e| {
                    JsValue::from_str(&format!("Failed to deserialize index value: {:?}", e))
                })?;

                result.push(value);
            }
            Ok(result)
        })
        .transpose()
}
