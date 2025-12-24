use crate::utils::getters::VecU8ToUint8Array;
use dpp::data_contract::DataContract;
use dpp::serialization::PlatformDeserializableWithPotentialValidationFromVersionedStructure;
use dpp::version::PlatformVersion;
use dpp::voting::votes::Vote;
use drive::drive::Drive;
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct VerifyMasternodeVoteResult {
    root_hash: Vec<u8>,
    vote: Option<Vec<u8>>,
}

#[wasm_bindgen]
impl VerifyMasternodeVoteResult {
    #[wasm_bindgen(getter)]
    pub fn root_hash(&self) -> Uint8Array {
        self.root_hash.to_uint8array()
    }

    #[wasm_bindgen(getter)]
    pub fn vote(&self) -> Option<Vec<u8>> {
        self.vote.clone()
    }
}

#[wasm_bindgen(js_name = "verifyMasternodeVote")]
pub fn verify_masternode_vote(
    proof: &Uint8Array,
    masternode_pro_tx_hash: &Uint8Array,
    vote_cbor: &Uint8Array,
    data_contract_cbor: &Uint8Array,
    verify_subset_of_proof: bool,
    platform_version_number: u32,
) -> Result<VerifyMasternodeVoteResult, JsValue> {
    let proof_vec = proof.to_vec();

    let platform_version = PlatformVersion::get(platform_version_number)
        .map_err(|e| JsValue::from_str(&format!("Invalid platform version: {:?}", e)))?;

    let masternode_pro_tx_hash_bytes: [u8; 32] =
        masternode_pro_tx_hash.to_vec().try_into().map_err(|_| {
            JsValue::from_str("Invalid masternode_pro_tx_hash length. Expected 32 bytes.")
        })?;

    // Deserialize the vote
    let vote: Vote = ciborium::de::from_reader(&vote_cbor.to_vec()[..])
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize vote: {:?}", e)))?;

    // Deserialize the data contract
    let data_contract = DataContract::versioned_deserialize(
        &data_contract_cbor.to_vec(),
        true,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Failed to deserialize data contract: {:?}", e)))?;

    let (root_hash, vote_option) = Drive::verify_masternode_vote(
        &proof_vec,
        masternode_pro_tx_hash_bytes,
        &vote,
        &data_contract,
        verify_subset_of_proof,
        platform_version,
    )
    .map_err(|e| JsValue::from_str(&format!("Verification failed: {:?}", e)))?;

    // Serialize the optional vote if it exists
    let vote_bytes = vote_option
        .map(|v| {
            let mut bytes = Vec::new();
            ciborium::into_writer(&v, &mut bytes)
                .map_err(|e| JsValue::from_str(&format!("Failed to serialize vote: {:?}", e)))?;
            Ok::<Vec<u8>, JsValue>(bytes)
        })
        .transpose()?;

    Ok(VerifyMasternodeVoteResult {
        root_hash: root_hash.to_vec(),
        vote: vote_bytes,
    })
}
