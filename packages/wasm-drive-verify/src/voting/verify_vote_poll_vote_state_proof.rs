use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use js_sys::{Array, Object, Reflect, Uint8Array};
use std::sync::Arc;
use wasm_bindgen::prelude::*;

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
    let contract_bytes = contract_cbor.to_vec();
    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;
        
    let contract = DataContract::versioned_deserialize(&contract_bytes, true, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize contract: {:?}", e)))?;
    let contract_arc = Arc::new(contract);

    // Parse the contested document resource vote poll identifier
    let _contested_document_resource_vote_poll: Identifier =
        Identifier::from_bytes(&contested_document_resource_vote_poll_bytes.to_vec())
            .map_err(|e| JsValue::from_str(&format!("Invalid vote poll identifier: {:?}", e)))?;

    // Create the query
    let query = ContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id: contract_arc.id(),
            document_type_name: document_type_name.to_string(),
            index_name: index_name.to_string(),
            index_values: vec![],
        },
        result_type: if result_type == "documents" {
            ContestedDocumentVotePollDriveQueryResultType::Documents
        } else {
            ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
        },
        offset: None,
        limit: None,
        start_at: None,
        allow_include_locked_and_abstaining_vote_tally,
    };

    let resolved_query = query
        .resolve_with_provided_borrowed_contract(&contract_arc)
        .map_err(|e| JsValue::from_str(&format!("Failed to resolve query: {:?}", e)))?;

    let (root_hash, execution_result) = resolved_query
        .verify_vote_poll_vote_state_proof(&proof_vec, platform_version)
        .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Convert execution result to JS object
    let js_result = {
        let result_obj = Object::new();

        // Add contenders array
        let contenders_array = Array::new();
        for contender in execution_result.contenders {
            let doc_bytes = contender.serialized_document().as_ref().map(|doc| doc.to_vec()).unwrap_or_default();
            let doc_uint8 = Uint8Array::from(&doc_bytes[..]);
            contenders_array.push(&doc_uint8);
        }
        Reflect::set(
            &result_obj,
            &JsValue::from_str("contenders"),
            &contenders_array,
        )
        .map_err(|_| JsValue::from_str("Failed to set contenders"))?;

        // Add vote tallies if present
        if let Some(abstaining_vote_tally) = execution_result.abstaining_vote_tally {
            Reflect::set(
                &result_obj,
                &JsValue::from_str("abstainVoteTally"),
                &JsValue::from_f64(abstaining_vote_tally as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set abstainVoteTally"))?;
        }

        if let Some(locked_vote_tally) = execution_result.locked_vote_tally {
            Reflect::set(
                &result_obj,
                &JsValue::from_str("lockedVoteTally"),
                &JsValue::from_f64(locked_vote_tally as f64),
            )
            .map_err(|_| JsValue::from_str("Failed to set lockedVoteTally"))?;
        }

        if let Some((_winner_info, _block_info)) = execution_result.winner {
            // For now, just set the winner identifier if available
            let winner_obj = Object::new();
            // TODO: Add proper serialization for ContestedDocumentVotePollWinnerInfo
            // when it implements Serialize
            Reflect::set(&result_obj, &JsValue::from_str("winner"), &winner_obj)
                .map_err(|_| JsValue::from_str("Failed to set winner"))?;
        }

        Reflect::set(
            &result_obj,
            &JsValue::from_str("skipped"),
            &JsValue::from_f64(execution_result.skipped as f64),
        )
        .map_err(|_| JsValue::from_str("Failed to set skipped"))?;

        result_obj.into()
    };

    Ok(VerifyVotePollVoteStateProofResult {
        root_hash: root_hash.to_vec(),
        result: js_result,
    })
}
