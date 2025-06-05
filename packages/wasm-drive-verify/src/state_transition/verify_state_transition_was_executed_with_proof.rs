use drive::drive::Drive;
use drive::verify::RootHash;
use drive::query::ContractLookupFn;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, Object, Reflect};
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;

#[wasm_bindgen]
pub struct VerifyStateTransitionWasExecutedWithProofResult {
    root_hash: Vec<u8>,
    proof_result: JsValue,
}

#[wasm_bindgen]
impl VerifyStateTransitionWasExecutedWithProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
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
    let contract_lookup_fn: ContractLookupFn = Box::new(move |identifier: &Identifier| {
        known_contracts.get(identifier).cloned()
    });

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

fn parse_known_contracts(known_contracts_js: &JsValue) -> Result<HashMap<Identifier, DataContract>, JsValue> {
    let mut contracts = HashMap::new();

    if known_contracts_js.is_null() || known_contracts_js.is_undefined() {
        return Ok(contracts);
    }

    let obj: Object = known_contracts_js.clone().dyn_into()
        .map_err(|_| JsValue::from_str("known_contracts must be an object"))?;

    let keys = Object::keys(&obj);
    
    for i in 0..keys.length() {
        let key = keys.get(i);
        let key_str = key.as_string()
            .ok_or_else(|| JsValue::from_str("Contract ID must be a string"))?;
        
        // Parse identifier from hex string
        let identifier = Identifier::from_string(&key_str, Default::default())
            .map_err(|e| JsValue::from_str(&format!("Invalid contract ID: {:?}", e)))?;
        
        let contract_js = Reflect::get(&obj, &key)
            .map_err(|_| JsValue::from_str("Failed to get contract"))?;
        
        let contract: DataContract = from_value(contract_js)
            .map_err(|e| JsValue::from_str(&format!("Failed to parse contract: {:?}", e)))?;
        
        contracts.insert(identifier, contract);
    }

    Ok(contracts)
}

fn convert_proof_result_to_js(proof_result: &StateTransitionProofResult) -> Result<JsValue, JsValue> {
    // Convert the proof result to a JS object
    // This will need to handle the various StateTransitionProofResult variants
    let obj = Object::new();

    match proof_result {
        StateTransitionProofResult::VerifiedBalanceTransition(balance_transition) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("VerifiedBalanceTransition"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            
            Reflect::set(&obj, &JsValue::from_str("feeResult"), &JsValue::from_f64(balance_transition.fee_result as f64))
                .map_err(|_| JsValue::from_str("Failed to set feeResult"))?;
            
            Reflect::set(&obj, &JsValue::from_str("feesUpdatedAfterBlock"), &JsValue::from_bool(balance_transition.fees_updated_after_block))
                .map_err(|_| JsValue::from_str("Failed to set feesUpdatedAfterBlock"))?;
        }
        StateTransitionProofResult::VerifiedIdentityBalanceAndRevisionTransition(identity_transition) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("VerifiedIdentityBalanceAndRevisionTransition"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            
            Reflect::set(&obj, &JsValue::from_str("feeResult"), &JsValue::from_f64(identity_transition.fee_result as f64))
                .map_err(|_| JsValue::from_str("Failed to set feeResult"))?;
            
            Reflect::set(&obj, &JsValue::from_str("feesUpdatedAfterBlock"), &JsValue::from_bool(identity_transition.fees_updated_after_block))
                .map_err(|_| JsValue::from_str("Failed to set feesUpdatedAfterBlock"))?;
            
            if let Some(balance) = identity_transition.balance {
                Reflect::set(&obj, &JsValue::from_str("balance"), &JsValue::from_f64(balance as f64))
                    .map_err(|_| JsValue::from_str("Failed to set balance"))?;
            }
            
            if let Some(revision) = identity_transition.revision {
                Reflect::set(&obj, &JsValue::from_str("revision"), &JsValue::from_f64(revision as f64))
                    .map_err(|_| JsValue::from_str("Failed to set revision"))?;
            }
        }
        StateTransitionProofResult::VerifiedPartialIdentityContractInfos(partial_info) => {
            Reflect::set(&obj, &JsValue::from_str("type"), &JsValue::from_str("VerifiedPartialIdentityContractInfos"))
                .map_err(|_| JsValue::from_str("Failed to set type"))?;
            
            // Convert contract infos to JS
            // This would need more detailed implementation based on the actual structure
        }
    }

    Ok(obj.into())
}