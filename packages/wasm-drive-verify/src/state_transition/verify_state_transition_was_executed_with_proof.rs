use crate::utils::getters::VecU8ToUint8Array;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::query::ContractLookupFn;
use js_sys::{Object, Reflect, Uint8Array};
use serde_wasm_bindgen::{from_value, to_value};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

// Import the partial identity serialization function from the identity module
use crate::identity::verify_identity_keys_by_identity_id::partial_identity_to_js;

#[wasm_bindgen]
pub struct VerifyStateTransitionWasExecutedWithProofResult {
    root_hash: Vec<u8>,
    proof_result: JsValue,
}

#[wasm_bindgen]
impl VerifyStateTransitionWasExecutedWithProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn proof_result(&self) -> JsValue {
        self.proof_result.clone()
    }
}

#[wasm_bindgen(js_name = "verifyStateTransitionWasExecutedWithProof")]
pub fn verify_state_transition_was_executed_with_proof(
    state_transition_js: &JsValue,
    block_height: u64,
    block_time_ms: u64,
    block_core_height: u32,
    proof: &Uint8Array,
    known_contracts_js: &JsValue,
    platform_version_number: u32,
) -> Result<VerifyStateTransitionWasExecutedWithProofResult, JsValue> {
    let proof_vec = proof.to_vec();

    // Parse state transition from JS
    let state_transition = parse_state_transition(state_transition_js)?;

    // Create block info
    let block_info = BlockInfo {
        time_ms: block_time_ms,
        height: block_height,
        core_height: block_core_height,
        epoch: Default::default(),
    };

    // Parse known contracts from JS object
    let known_contracts = parse_known_contracts(known_contracts_js)?;

    // Create contract lookup function
    let contract_lookup_fn: Box<ContractLookupFn> =
        Box::new(move |identifier: &Identifier| Ok(known_contracts.get(identifier).cloned()));

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, proof_result) = Drive::verify_state_transition_was_executed_with_proof(
        &state_transition,
        &block_info,
        &proof_vec,
        &contract_lookup_fn,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert proof result to JS value
    let proof_result_js = convert_proof_result_to_js(&proof_result)?;

    Ok(VerifyStateTransitionWasExecutedWithProofResult {
        root_hash: root_hash.to_vec(),
        proof_result: proof_result_js,
    })
}

fn parse_state_transition(state_transition_js: &JsValue) -> Result<StateTransition, JsValue> {
    // Parse the state transition from JS value
    // The JS side should provide the state transition as a JS object
    from_value::<StateTransition>(state_transition_js.clone())
        .map_err(|e| JsValue::from_str(&format!("Failed to parse state transition: {:?}", e)))
}

fn parse_known_contracts(
    known_contracts_js: &JsValue,
) -> Result<HashMap<Identifier, Arc<DataContract>>, JsValue> {
    let mut contracts = HashMap::new();

    if known_contracts_js.is_null() || known_contracts_js.is_undefined() {
        return Ok(contracts);
    }

    let obj: Object = known_contracts_js
        .clone()
        .dyn_into()
        .map_err(|_| JsValue::from_str("known_contracts must be an object"))?;

    let keys = Object::keys(&obj);

    for i in 0..keys.length() {
        let key = keys.get(i);
        let key_str = key
            .as_string()
            .ok_or_else(|| JsValue::from_str("Contract ID must be a string"))?;

        // Parse identifier from hex string
        use dpp::platform_value::string_encoding::Encoding;
        let identifier = Identifier::from_string(&key_str, Encoding::Hex)
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {:?}", e)))?;

        let contract_js =
            Reflect::get(&obj, &key).map_err(|_| JsValue::from_str("Failed to get contract"))?;

        let contract: DataContract = from_value(contract_js)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse contract: {:?}", e)))?;

        contracts.insert(identifier, Arc::new(contract));
    }

    Ok(contracts)
}

fn convert_proof_result_to_js(
    proof_result: &StateTransitionProofResult,
) -> Result<JsValue, JsValue> {
    // Convert the proof result to a JS object
    // This will need to handle the various StateTransitionProofResult variants
    let obj = Object::new();

    match proof_result {
        StateTransitionProofResult::VerifiedDataContract(_contract) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("VerifiedDataContract"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let contract_js = to_value(_contract).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize data contract: {:?}", e))
            })?;
            Reflect::set(&obj, &JsValue::from_str("dataContract"), &contract_js)
                .map_err(|_| JsValue::from_str("Failed to set dataContract"))?;
        }
        StateTransitionProofResult::VerifiedIdentity(_identity) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("VerifiedIdentity"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let identity_js = to_value(_identity).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize identity: {:?}", e))
            })?;
            Reflect::set(&obj, &JsValue::from_str("identity"), &identity_js)
                .map_err(|_| JsValue::from_str("Failed to set identity"))?;
        }
        StateTransitionProofResult::VerifiedDocuments(_documents) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("VerifiedDocuments"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let documents_js = to_value(_documents).map_err(|e| {
                JsValue::from_str(&format!("Failed to serialize documents: {:?}", e))
            })?;
            Reflect::set(&obj, &JsValue::from_str("documents"), &documents_js)
                .map_err(|_| JsValue::from_str("Failed to set documents"))?;
        }
        StateTransitionProofResult::VerifiedPartialIdentity(_partial_identity) => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("VerifiedPartialIdentity"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            let partial_identity_js = partial_identity_to_js(_partial_identity)?;
            Reflect::set(
                &obj,
                &JsValue::from_str("partialIdentity"),
                &partial_identity_js,
            )
            .map_err(|_| JsValue::from_str("Failed to set partialIdentity"))?;
        }
        _ => {
            Reflect::set(
                &obj,
                &JsValue::from_str("type"),
                &JsValue::from_str("Unknown"),
            )
            .map_err(|_| JsValue::from_str("Failed to set type"))?;

            Reflect::set(
                &obj,
                &JsValue::from_str("message"),
                &JsValue::from_str("This proof result type is not yet implemented"),
            )
            .map_err(|_| JsValue::from_str("Failed to set message"))?;
        }
    }

    Ok(obj.into())
}
