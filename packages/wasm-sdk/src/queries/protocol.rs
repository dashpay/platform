use crate::error::WasmSdkError;
use crate::impl_wasm_serde_conversions;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::dashcore::ProTxHash;
use js_sys::Map;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::{ProTxHashLikeNullableJs, ProTxHashWasm};

#[dpp_json_convertible_derive::json_safe_fields(crate = "dash_sdk::dpp")]
#[wasm_bindgen(js_name = "ProtocolVersionUpgradeState")]
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolVersionUpgradeStateWasm {
    current_protocol_version: u32,
    next_protocol_version: Option<u32>,
    activation_height: Option<u64>,
    vote_count: Option<u32>,
    #[serde(rename = "thresholdReached")]
    is_threshold_reached: bool,
}

impl ProtocolVersionUpgradeStateWasm {
    fn new(
        current_protocol_version: u32,
        next_protocol_version: Option<u32>,
        activation_height: Option<u64>,
        vote_count: Option<u32>,
        is_threshold_reached: bool,
    ) -> Self {
        Self {
            current_protocol_version,
            next_protocol_version,
            activation_height,
            vote_count,
            is_threshold_reached,
        }
    }
}

#[wasm_bindgen(js_class = ProtocolVersionUpgradeState)]
impl ProtocolVersionUpgradeStateWasm {
    #[wasm_bindgen(getter = "currentProtocolVersion")]
    pub fn current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }

    #[wasm_bindgen(getter = "nextProtocolVersion")]
    pub fn next_protocol_version(&self) -> Option<u32> {
        self.next_protocol_version
    }

    #[wasm_bindgen(getter = "activationHeight")]
    pub fn activation_height(&self) -> Option<u64> {
        self.activation_height
    }

    #[wasm_bindgen(getter = "voteCount")]
    pub fn vote_count(&self) -> Option<u32> {
        self.vote_count
    }

    #[wasm_bindgen(getter = "isThresholdReached")]
    pub fn is_threshold_reached(&self) -> bool {
        self.is_threshold_reached
    }
}

#[wasm_bindgen(js_name = "ProtocolVersionUpgradeVoteStatus")]
#[derive(Clone)]
pub struct ProtocolVersionUpgradeVoteStatusWasm {
    pro_tx_hash: ProTxHashWasm,
    version: u32,
}

impl ProtocolVersionUpgradeVoteStatusWasm {
    pub(crate) fn new(pro_tx_hash: ProTxHash, version: u32) -> Self {
        Self {
            pro_tx_hash: ProTxHashWasm::from(pro_tx_hash),
            version,
        }
    }
}

#[wasm_bindgen(js_class = ProtocolVersionUpgradeVoteStatus)]
impl ProtocolVersionUpgradeVoteStatusWasm {
    #[wasm_bindgen(getter = "proTxHash")]
    pub fn pro_tx_hash(&self) -> ProTxHashWasm {
        self.pro_tx_hash
    }

    #[wasm_bindgen(getter)]
    pub fn version(&self) -> u32 {
        self.version
    }
}

impl_wasm_serde_conversions!(ProtocolVersionUpgradeStateWasm, ProtocolVersionUpgradeState);

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getProtocolVersionUpgradeState")]
    pub async fn get_protocol_version_upgrade_state(
        &self,
    ) -> Result<ProtocolVersionUpgradeStateWasm, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::ProtocolVersionVoteCount;

        let upgrade_result: drive_proof_verifier::types::ProtocolVersionUpgrades =
            ProtocolVersionVoteCount::fetch_many(self.as_ref(), ()).await?;

        // Get the current protocol version from the SDK
        let current_version = self.version();

        // Find the next version with votes
        let mut next_version = None;
        let mut activation_height = None;
        let mut vote_count = None;
        let mut threshold_reached = false;

        // The result is an IndexMap<u32, Option<u64>> where u32 is version and Option<u64> is activation height
        for (version, height_opt) in upgrade_result.iter() {
            if *version > current_version {
                next_version = Some(*version);
                activation_height = *height_opt;
                // TODO: Get actual vote count and threshold from platform
                vote_count = None;
                threshold_reached = height_opt.is_some();
                break;
            }
        }

        Ok(ProtocolVersionUpgradeStateWasm::new(
            current_version,
            next_version,
            activation_height,
            vote_count,
            threshold_reached,
        ))
    }

    #[wasm_bindgen(
        js_name = "getProtocolVersionUpgradeVoteStatus",
        unchecked_return_type = "Map<string, ProtocolVersionUpgradeVoteStatus>"
    )]
    pub async fn get_protocol_version_upgrade_vote_status(
        &self,
        #[wasm_bindgen(js_name = "startProTxHash")] start_pro_tx_hash: ProTxHashLikeNullableJs,
        count: u32,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::types::version_votes::MasternodeProtocolVoteEx;
        use drive_proof_verifier::types::MasternodeProtocolVote;

        // Parse the ProTxHash using extern type
        let start_hash: Option<ProTxHash> = start_pro_tx_hash.try_into()?;

        let votes_result =
            MasternodeProtocolVote::fetch_votes(self.as_ref(), start_hash, Some(count)).await?;

        // Convert to our response format
        let votes_map = Map::new();
        for (pro_tx_hash, vote_opt) in votes_result {
            if let Some(vote) = vote_opt {
                let key = JsValue::from_str(&pro_tx_hash.to_string());
                let value = JsValue::from(ProtocolVersionUpgradeVoteStatusWasm::new(
                    pro_tx_hash,
                    vote.voted_version,
                ));
                votes_map.set(&key, &value);
            }
        }

        Ok(votes_map)
    }

    // Proof versions for protocol queries

    #[wasm_bindgen(
        js_name = "getProtocolVersionUpgradeStateWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<ProtocolVersionUpgradeState>"
    )]
    pub async fn get_protocol_version_upgrade_state_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::ProtocolVersionVoteCount;

        let (upgrade_result, metadata, proof): (
            drive_proof_verifier::types::ProtocolVersionUpgrades,
            _,
            _,
        ) = ProtocolVersionVoteCount::fetch_many_with_metadata_and_proof(self.as_ref(), (), None)
            .await?;

        // Get the current protocol version from the SDK
        let current_version = self.version();

        // Find the next version with votes
        let mut next_version = None;
        let mut activation_height = None;
        let mut vote_count = None;
        let mut threshold_reached = false;

        for (version, height_opt) in upgrade_result.iter() {
            if *version > current_version {
                next_version = Some(*version);
                activation_height = *height_opt;
                vote_count = None;
                threshold_reached = height_opt.is_some();
                break;
            }
        }

        let state = ProtocolVersionUpgradeStateWasm::new(
            current_version,
            next_version,
            activation_height,
            vote_count,
            threshold_reached,
        );

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            state, metadata, proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getProtocolVersionUpgradeVoteStatusWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<string, ProtocolVersionUpgradeVoteStatus>>"
    )]
    pub async fn get_protocol_version_upgrade_vote_status_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "startProTxHash")] start_pro_tx_hash: ProTxHashLikeNullableJs,
        count: u32,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::{FetchMany, LimitQuery};
        use drive_proof_verifier::types::MasternodeProtocolVote;

        // Parse the ProTxHash using extern type
        let start_hash: Option<ProTxHash> = start_pro_tx_hash.try_into()?;

        // Create a LimitQuery with the start hash and count
        let query = LimitQuery {
            query: start_hash,
            limit: Some(count),
            start_info: None,
        };

        let (votes_result, metadata, proof) =
            MasternodeProtocolVote::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        // Convert to our response format
        let votes_map = Map::new();
        for (pro_tx_hash, vote_opt) in votes_result {
            if let Some(vote) = vote_opt {
                let key = JsValue::from_str(&pro_tx_hash.to_string());
                let value = JsValue::from(ProtocolVersionUpgradeVoteStatusWasm::new(
                    pro_tx_hash,
                    vote.voted_version,
                ));
                votes_map.set(&key, &value);
            }
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            votes_map, metadata, proof,
        ))
    }
}
