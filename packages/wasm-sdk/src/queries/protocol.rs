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
    vote_count: Option<u64>,
}

impl ProtocolVersionUpgradeStateWasm {
    fn new(
        current_protocol_version: u32,
        next_protocol_version: Option<u32>,
        vote_count: Option<u64>,
    ) -> Self {
        Self {
            current_protocol_version,
            next_protocol_version,
            vote_count,
        }
    }
}

#[wasm_bindgen(js_class = ProtocolVersionUpgradeState)]
impl ProtocolVersionUpgradeStateWasm {
    /// Protocol version the chain was running when this response was produced.
    #[wasm_bindgen(getter = "currentProtocolVersion")]
    pub fn current_protocol_version(&self) -> u32 {
        self.current_protocol_version
    }

    /// Candidate upgrade version: the version above the current one with the
    /// most evonode votes, if any votes exist.
    #[wasm_bindgen(getter = "nextProtocolVersion")]
    pub fn next_protocol_version(&self) -> Option<u32> {
        self.next_protocol_version
    }

    /// Number of evonode votes cast for `nextProtocolVersion`.
    #[wasm_bindgen(getter = "voteCount")]
    pub fn vote_count(&self) -> Option<u64> {
        self.vote_count
    }
}

/// Pick the candidate upgrade version from the vote counts: among versions
/// newer than `current_version`, the one with the most votes; equal vote
/// counts resolve to the highest version.
fn next_version_upgrade(
    upgrades: &drive_proof_verifier::types::ProtocolVersionUpgrades,
    current_version: u32,
) -> (Option<u32>, Option<u64>) {
    upgrades
        .iter()
        .filter_map(|(version, votes)| votes.map(|votes| (*version, votes)))
        .filter(|(version, _)| *version > current_version)
        .max_by_key(|&(version, votes)| (votes, version))
        .map_or((None, None), |(version, votes)| {
            (Some(version), Some(votes))
        })
}

#[cfg(test)]
mod tests {
    use super::next_version_upgrade;
    use drive_proof_verifier::types::ProtocolVersionUpgrades;

    #[test]
    fn empty_upgrades_yield_no_candidate() {
        let upgrades = ProtocolVersionUpgrades::new();

        assert_eq!(next_version_upgrade(&upgrades, 12), (None, None));
    }

    #[test]
    fn none_vote_counts_are_skipped() {
        let upgrades = ProtocolVersionUpgrades::from_iter([(13, None), (14, None)]);

        assert_eq!(next_version_upgrade(&upgrades, 12), (None, None));
    }

    #[test]
    fn versions_at_or_below_current_are_excluded() {
        let upgrades = ProtocolVersionUpgrades::from_iter([(11, Some(50)), (12, Some(80))]);

        assert_eq!(next_version_upgrade(&upgrades, 12), (None, None));
    }

    #[test]
    fn picks_future_version_with_most_votes() {
        let upgrades = ProtocolVersionUpgrades::from_iter([
            (11, Some(200)),
            (13, Some(7)),
            (14, Some(132)),
            (15, Some(3)),
        ]);

        assert_eq!(next_version_upgrade(&upgrades, 12), (Some(14), Some(132)));
    }

    #[test]
    fn equal_votes_resolve_to_highest_version() {
        let upgrades = ProtocolVersionUpgrades::from_iter([(13, Some(9)), (14, Some(9))]);

        assert_eq!(next_version_upgrade(&upgrades, 12), (Some(14), Some(9)));
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

        let (upgrade_result, metadata): (drive_proof_verifier::types::ProtocolVersionUpgrades, _) =
            ProtocolVersionVoteCount::fetch_many_with_metadata(self.as_ref(), (), None).await?;

        // The chain's protocol version at the time of the response
        let current_version = metadata.protocol_version;

        let (next_version, vote_count) = next_version_upgrade(&upgrade_result, current_version);

        Ok(ProtocolVersionUpgradeStateWasm::new(
            current_version,
            next_version,
            vote_count,
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

        // The chain's protocol version at the time of the response
        let current_version = metadata.protocol_version;

        let (next_version, vote_count) = next_version_upgrade(&upgrade_result, current_version);

        let state = ProtocolVersionUpgradeStateWasm::new(current_version, next_version, vote_count);

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
