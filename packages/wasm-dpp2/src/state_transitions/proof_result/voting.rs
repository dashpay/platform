//! Voting-related `StateTransitionProofResult` wrappers.

use crate::VoteWasm;
use crate::impl_wasm_conversions_serde;
use crate::impl_wasm_type_info;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// --- VerifiedMasternodeVote ---

#[wasm_bindgen(js_name = "VerifiedMasternodeVote")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedMasternodeVoteWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub vote: VoteWasm,
}

impl_wasm_type_info!(VerifiedMasternodeVoteWasm, VerifiedMasternodeVote);
impl_wasm_conversions_serde!(VerifiedMasternodeVoteWasm, VerifiedMasternodeVote);

// --- VerifiedNextDistribution ---

#[wasm_bindgen(js_name = "VerifiedNextDistribution")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedNextDistributionWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub vote: VoteWasm,
}

impl_wasm_type_info!(VerifiedNextDistributionWasm, VerifiedNextDistribution);
impl_wasm_conversions_serde!(VerifiedNextDistributionWasm, VerifiedNextDistribution);
