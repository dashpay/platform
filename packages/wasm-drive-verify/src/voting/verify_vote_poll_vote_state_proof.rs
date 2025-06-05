use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryExecutionResult, ResolvedContestedDocumentVotePollDriveQuery,
};
use drive::verify::RootHash;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use wasm_bindgen::prelude::*;
use js_sys::{Uint8Array, Array, Object, Reflect};
use std::sync::Arc;

#[wasm_bindgen]
pub struct VerifyVotePollVoteStateProofResult {
    root_hash: Vec<u8>,
    result: JsValue,
}

#[wasm_bindgen]
impl VerifyVotePollVoteStateProofResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn result(&self) -> JsValue {
        self.result.clone()
    }
}

#[wasm_bindgen(js_name = "verifyVotePollVoteStateProof")]
pub fn verify_vote_poll_vote_state_proof(
    proof: &Uint8Array,
    contract_cbor: &Uint8Array,
    document_type_name: &str,
    index_name: &str,
    contested_document_resource_vote_poll_bytes: &Uint8Array,
    result_type: &str, // "documents" or "values"
    allow_include_locked_and_abstaining_vote_tally: bool,
    platform_version_number: u32,
) -> Result<VerifyVotePollVoteStateProofResult, JsValue> {
    let proof_vec = proof.to_vec();
    
    // Deserialize the data contract
    let contract: DataContract = ciborium::de::from_reader(&contract_cbor.to_vec()[..])
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;
    let contract_arc = Arc::new(contract);
    
    // Parse the contested document resource vote poll identifier
    let contested_document_resource_vote_poll: Identifier = Identifier::from_bytes(
        &contested_document_resource_vote_poll_bytes.to_vec()
    ).map_err(|e| JsValue::from_str(&format!("Invalid vote poll identifier: {:?}", e)))?;
    
    // Create the resolved query
    let query = ResolvedContestedDocumentVotePollDriveQuery {
        contract: &contract_arc,
        document_type_name,
        index_name,
        contested_document_resource_vote_poll,
        result_type: if result_type == "documents" {
            drive::query::vote_poll_vote_state_query::ResultType::Documents
        } else {
            drive::query::vote_poll_vote_state_query::ResultType::DocumentsAndVoteTally
        },
        allow_include_locked_and_abstaining_vote_tally,
        start_at: None,
        limit: None,
        order_ascending: true,
    };
    
    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let (root_hash, execution_result) = query.verify_vote_poll_vote_state_proof(
        &proof_vec,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert execution result to JS object
    let js_result = match execution_result {
        ContestedDocumentVotePollDriveQueryExecutionResult::DocumentsAndVoteTally {
            contenders,
            additional,
        } => {
            let result_obj = Object::new();
            
            // Add contenders array
            let contenders_array = Array::new();
            for doc in contenders {
                let doc_bytes = ciborium::ser::into_vec(&doc)
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize document: {:?}", e)))?;
                let doc_uint8 = Uint8Array::from(&doc_bytes[..]);
                contenders_array.push(&doc_uint8);
            }
            Reflect::set(&result_obj, &JsValue::from_str("contenders"), &contenders_array)
                .map_err(|_| JsValue::from_str("Failed to set contenders"))?;
            
            // Add additional fields
            if let Some(abstain_vote_tally) = additional.abstain_vote_tally {
                Reflect::set(&result_obj, &JsValue::from_str("abstainVoteTally"), &JsValue::from_f64(abstain_vote_tally as f64))
                    .map_err(|_| JsValue::from_str("Failed to set abstainVoteTally"))?;
            }
            
            if let Some(locked_vote_tally) = additional.locked_vote_tally {
                Reflect::set(&result_obj, &JsValue::from_str("lockedVoteTally"), &JsValue::from_f64(locked_vote_tally as f64))
                    .map_err(|_| JsValue::from_str("Failed to set lockedVoteTally"))?;
            }
            
            if let Some(winner) = additional.winner {
                let winner_bytes = winner.to_vec();
                let winner_uint8 = Uint8Array::from(&winner_bytes[..]);
                Reflect::set(&result_obj, &JsValue::from_str("winner"), &winner_uint8)
                    .map_err(|_| JsValue::from_str("Failed to set winner"))?;
            }
            
            if additional.skipped_identity_ids.len() > 0 {
                let skipped_array = Array::new();
                for id in additional.skipped_identity_ids {
                    let id_uint8 = Uint8Array::from(id.as_bytes());
                    skipped_array.push(&id_uint8);
                }
                Reflect::set(&result_obj, &JsValue::from_str("skippedIdentityIds"), &skipped_array)
                    .map_err(|_| JsValue::from_str("Failed to set skippedIdentityIds"))?;
            }
            
            Reflect::set(&result_obj, &JsValue::from_str("finishedResults"), &JsValue::from_bool(additional.finished_results))
                .map_err(|_| JsValue::from_str("Failed to set finishedResults"))?;
            
            result_obj.into()
        }
        ContestedDocumentVotePollDriveQueryExecutionResult::Documents(docs) => {
            // Just return the documents array
            let docs_array = Array::new();
            for doc in docs {
                let doc_bytes = ciborium::ser::into_vec(&doc)
                    .map_err(|e| JsValue::from_str(&format!("Failed to serialize document: {:?}", e)))?;
                let doc_uint8 = Uint8Array::from(&doc_bytes[..]);
                docs_array.push(&doc_uint8);
            }
            docs_array.into()
        }
    };

    Ok(VerifyVotePollVoteStateProofResult {
        root_hash: root_hash.to_vec(),
        result: js_result,
    })
}