use crate::error::WasmSdkError;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::core_types::validator_set::v0::ValidatorSetV0Getters;
use js_sys::{Array, BigInt};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "StatusSoftware")]
#[derive(Clone)]
pub struct StatusSoftwareWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub dapi: String,
    #[wasm_bindgen(getter_with_clone)]
    pub drive: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub tenderdash: Option<String>,
}

impl StatusSoftwareWasm {
    fn new(dapi: String, drive: Option<String>, tenderdash: Option<String>) -> Self {
        Self {
            dapi,
            drive,
            tenderdash,
        }
    }
}

#[wasm_bindgen(js_name = "StatusTenderdashProtocol")]
#[derive(Clone)]
pub struct StatusTenderdashProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub p2p: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub block: u32,
}

impl StatusTenderdashProtocolWasm {
    fn new(p2p: u32, block: u32) -> Self {
        Self { p2p, block }
    }
}

#[wasm_bindgen(js_name = "StatusDriveProtocol")]
#[derive(Clone)]
pub struct StatusDriveProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub latest: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub current: u32,
}

impl StatusDriveProtocolWasm {
    fn new(latest: u32, current: u32) -> Self {
        Self { latest, current }
    }
}

#[wasm_bindgen(js_name = "StatusProtocol")]
#[derive(Clone)]
pub struct StatusProtocolWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub tenderdash: StatusTenderdashProtocolWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub drive: StatusDriveProtocolWasm,
}

impl StatusProtocolWasm {
    fn new(tenderdash: StatusTenderdashProtocolWasm, drive: StatusDriveProtocolWasm) -> Self {
        Self { tenderdash, drive }
    }
}

#[wasm_bindgen(js_name = "StatusVersion")]
#[derive(Clone)]
pub struct StatusVersionWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub software: StatusSoftwareWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub protocol: StatusProtocolWasm,
}

impl StatusVersionWasm {
    fn new(software: StatusSoftwareWasm, protocol: StatusProtocolWasm) -> Self {
        Self { software, protocol }
    }
}

#[wasm_bindgen(js_name = "StatusNode")]
#[derive(Clone)]
pub struct StatusNodeWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub id: String,
    #[wasm_bindgen(getter_with_clone)]
    pub pro_tx_hash: Option<String>,
}

impl StatusNodeWasm {
    fn new(id: String, pro_tx_hash: Option<String>) -> Self {
        Self { id, pro_tx_hash }
    }
}

#[wasm_bindgen(js_name = "StatusChain")]
#[derive(Clone)]
pub struct StatusChainWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub catching_up: bool,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_block_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_app_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub latest_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_block_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_app_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub earliest_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub max_peer_block_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub core_chain_locked_height: Option<u32>,
}

impl StatusChainWasm {
    #[allow(clippy::too_many_arguments)]
    fn new(
        catching_up: bool,
        latest_block_hash: String,
        latest_app_hash: String,
        latest_block_height: String,
        earliest_block_hash: String,
        earliest_app_hash: String,
        earliest_block_height: String,
        max_peer_block_height: String,
        core_chain_locked_height: Option<u32>,
    ) -> Self {
        Self {
            catching_up,
            latest_block_hash,
            latest_app_hash,
            latest_block_height,
            earliest_block_hash,
            earliest_app_hash,
            earliest_block_height,
            max_peer_block_height,
            core_chain_locked_height,
        }
    }
}

#[wasm_bindgen(js_name = "StatusNetwork")]
#[derive(Clone)]
pub struct StatusNetworkWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub chain_id: String,
    #[wasm_bindgen(getter_with_clone)]
    pub peers_count: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub listening: bool,
}

impl StatusNetworkWasm {
    fn new(chain_id: String, peers_count: u32, listening: bool) -> Self {
        Self {
            chain_id,
            peers_count,
            listening,
        }
    }
}

#[wasm_bindgen(js_name = "StatusStateSync")]
#[derive(Clone)]
pub struct StatusStateSyncWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub total_synced_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub remaining_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub total_snapshots: u32,
    #[wasm_bindgen(getter_with_clone)]
    pub chunk_process_avg_time: String,
    #[wasm_bindgen(getter_with_clone)]
    pub snapshot_height: String,
    #[wasm_bindgen(getter_with_clone)]
    pub snapshot_chunks_count: String,
    #[wasm_bindgen(getter_with_clone)]
    pub backfilled_blocks: String,
    #[wasm_bindgen(getter_with_clone)]
    pub backfill_blocks_total: String,
}

#[wasm_bindgen(js_name = "StatusTime")]
#[derive(Clone)]
pub struct StatusTimeWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub local: String,
    #[wasm_bindgen(getter_with_clone)]
    pub block: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub genesis: Option<String>,
    #[wasm_bindgen(getter_with_clone)]
    pub epoch: Option<u32>,
}

impl StatusTimeWasm {
    fn new(
        local: String,
        block: Option<String>,
        genesis: Option<String>,
        epoch: Option<u32>,
    ) -> Self {
        Self {
            local,
            block,
            genesis,
            epoch,
        }
    }
}

#[wasm_bindgen(js_name = "StatusResponse")]
#[derive(Clone)]
pub struct StatusResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub version: StatusVersionWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub node: StatusNodeWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub chain: StatusChainWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub network: StatusNetworkWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub state_sync: StatusStateSyncWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub time: StatusTimeWasm,
}

impl StatusResponseWasm {
    fn new(
        version: StatusVersionWasm,
        node: StatusNodeWasm,
        chain: StatusChainWasm,
        network: StatusNetworkWasm,
        state_sync: StatusStateSyncWasm,
        time: StatusTimeWasm,
    ) -> Self {
        Self {
            version,
            node,
            chain,
            network,
            state_sync,
            time,
        }
    }
}

#[wasm_bindgen(js_name = "QuorumInfo")]
#[derive(Clone)]
pub struct QuorumInfoWasm {
    quorum_hash: String,
    quorum_type: String,
    member_count: u32,
    threshold: u32,
    is_verified: bool,
}

impl QuorumInfoWasm {
    fn new(
        quorum_hash: String,
        quorum_type: String,
        member_count: u32,
        threshold: u32,
        is_verified: bool,
    ) -> Self {
        Self {
            quorum_hash,
            quorum_type,
            member_count,
            threshold,
            is_verified,
        }
    }
}

#[wasm_bindgen(js_class = QuorumInfo)]
impl QuorumInfoWasm {
    #[wasm_bindgen(getter = "quorumHash")]
    pub fn quorum_hash(&self) -> String {
        self.quorum_hash.clone()
    }

    #[wasm_bindgen(getter = "quorumType")]
    pub fn quorum_type(&self) -> String {
        self.quorum_type.clone()
    }

    #[wasm_bindgen(getter = "memberCount")]
    pub fn member_count(&self) -> u32 {
        self.member_count
    }

    #[wasm_bindgen(getter = "threshold")]
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    #[wasm_bindgen(getter = "isVerified")]
    pub fn is_verified(&self) -> bool {
        self.is_verified
    }
}

#[wasm_bindgen(js_name = "CurrentQuorumsInfo")]
#[derive(Clone)]
pub struct CurrentQuorumsInfoWasm {
    quorums: Vec<QuorumInfoWasm>,
    height: u64,
}

impl CurrentQuorumsInfoWasm {
    fn new(quorums: Vec<QuorumInfoWasm>, height: u64) -> Self {
        Self { quorums, height }
    }
}

#[wasm_bindgen(js_class = CurrentQuorumsInfo)]
impl CurrentQuorumsInfoWasm {
    #[wasm_bindgen(getter = "quorums")]
    pub fn quorums(&self) -> Array {
        let array = Array::new();
        for quorum in &self.quorums {
            array.push(&JsValue::from(quorum.clone()));
        }
        array
    }

    #[wasm_bindgen(getter = "height")]
    pub fn height(&self) -> u64 {
        self.height
    }
}

#[wasm_bindgen(js_name = "PrefundedSpecializedBalance")]
#[derive(Clone)]
pub struct PrefundedSpecializedBalanceWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub identity_id: String,
    balance: u64,
}

impl PrefundedSpecializedBalanceWasm {
    fn new(identity_id: String, balance: u64) -> Self {
        Self {
            identity_id,
            balance,
        }
    }
}

#[wasm_bindgen(js_class = PrefundedSpecializedBalance)]
impl PrefundedSpecializedBalanceWasm {
    #[wasm_bindgen(getter = "balance")]
    pub fn balance(&self) -> BigInt {
        BigInt::from(self.balance)
    }
}

#[wasm_bindgen(js_name = "PathElement")]
#[derive(Clone)]
pub struct PathElementWasm {
    path: Vec<String>,
    value: Option<String>,
}

impl PathElementWasm {
    fn new(path: Vec<String>, value: Option<String>) -> Self {
        Self { path, value }
    }
}

#[wasm_bindgen(js_class = PathElement)]
impl PathElementWasm {
    #[wasm_bindgen(getter = "path")]
    pub fn path(&self) -> Array {
        let array = Array::new();
        for segment in &self.path {
            array.push(&JsValue::from_str(segment));
        }
        array
    }

    #[wasm_bindgen(getter = "value")]
    pub fn value(&self) -> Option<String> {
        self.value.clone()
    }
}

#[wasm_bindgen(js_name = "StateTransitionResult")]
#[derive(Clone)]
pub struct StateTransitionResultWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub state_transition_hash: String,
    #[wasm_bindgen(getter_with_clone)]
    pub status: String,
    #[wasm_bindgen(getter_with_clone)]
    pub error: Option<String>,
}

impl StateTransitionResultWasm {
    fn new(state_transition_hash: String, status: String, error: Option<String>) -> Self {
        Self {
            state_transition_hash,
            status,
            error,
        }
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getStatus")]
    pub async fn get_status(&self) -> Result<StatusResponseWasm, WasmSdkError> {
        use dapi_grpc::platform::v0::get_status_request::{GetStatusRequestV0, Version};
        use dapi_grpc::platform::v0::GetStatusRequest;
        use dash_sdk::RequestSettings;
        use rs_dapi_client::DapiRequestExecutor;

        // Create the gRPC request
        let request = GetStatusRequest {
            version: Some(Version::V0(GetStatusRequestV0 {})),
        };

        // Execute the request
        let response = self
            .as_ref()
            .execute(request, RequestSettings::default())
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to get status: {}", e)))?;

        // Parse the response
        use dapi_grpc::platform::v0::get_status_response::Version as ResponseVersion;

        let v0_response = match response.inner.version {
            Some(ResponseVersion::V0(v0)) => v0,
            None => return Err(WasmSdkError::generic("No version in GetStatus response")),
        };

        // Map the response to our StatusResponse structure
        let software = StatusSoftwareWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .map(|s| s.dapi.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .and_then(|s| s.drive.clone()),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.software.as_ref())
                .and_then(|s| s.tenderdash.clone()),
        );

        let tenderdash_protocol = StatusTenderdashProtocolWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.tenderdash.as_ref())
                .map(|t| t.p2p)
                .unwrap_or(0),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.tenderdash.as_ref())
                .map(|t| t.block)
                .unwrap_or(0),
        );

        let drive_protocol = StatusDriveProtocolWasm::new(
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.drive.as_ref())
                .map(|d| d.latest)
                .unwrap_or(0),
            v0_response
                .version
                .as_ref()
                .and_then(|v| v.protocol.as_ref())
                .and_then(|p| p.drive.as_ref())
                .map(|d| d.current)
                .unwrap_or(0),
        );

        let protocol = StatusProtocolWasm::new(tenderdash_protocol, drive_protocol);
        let version = StatusVersionWasm::new(software, protocol);

        let node = StatusNodeWasm::new(
            v0_response
                .node
                .as_ref()
                .map(|n| hex::encode(&n.id))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .node
                .as_ref()
                .and_then(|n| n.pro_tx_hash.as_ref())
                .map(hex::encode),
        );

        let chain = StatusChainWasm::new(
            v0_response
                .chain
                .as_ref()
                .map(|c| c.catching_up)
                .unwrap_or(false),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.latest_block_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.latest_app_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.latest_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.earliest_block_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| hex::encode(&c.earliest_app_hash))
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.earliest_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .map(|c| c.max_peer_block_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .chain
                .as_ref()
                .and_then(|c| c.core_chain_locked_height),
        );

        let network = StatusNetworkWasm::new(
            v0_response
                .network
                .as_ref()
                .map(|n| n.chain_id.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            v0_response
                .network
                .as_ref()
                .map(|n| n.peers_count)
                .unwrap_or(0),
            v0_response
                .network
                .as_ref()
                .map(|n| n.listening)
                .unwrap_or(false),
        );

        let state_sync = StatusStateSyncWasm {
            total_synced_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.total_synced_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            remaining_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.remaining_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            total_snapshots: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.total_snapshots)
                .unwrap_or(0),
            chunk_process_avg_time: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.chunk_process_avg_time.to_string())
                .unwrap_or_else(|| "0".to_string()),
            snapshot_height: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.snapshot_height.to_string())
                .unwrap_or_else(|| "0".to_string()),
            snapshot_chunks_count: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.snapshot_chunks_count.to_string())
                .unwrap_or_else(|| "0".to_string()),
            backfilled_blocks: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.backfilled_blocks.to_string())
                .unwrap_or_else(|| "0".to_string()),
            backfill_blocks_total: v0_response
                .state_sync
                .as_ref()
                .map(|s| s.backfill_blocks_total.to_string())
                .unwrap_or_else(|| "0".to_string()),
        };

        let time = StatusTimeWasm::new(
            v0_response
                .time
                .as_ref()
                .map(|t| t.local.to_string())
                .unwrap_or_else(|| "0".to_string()),
            v0_response
                .time
                .as_ref()
                .and_then(|t| t.block)
                .map(|b| b.to_string()),
            v0_response
                .time
                .as_ref()
                .and_then(|t| t.genesis)
                .map(|g| g.to_string()),
            v0_response.time.as_ref().and_then(|t| t.epoch),
        );

        Ok(StatusResponseWasm::new(
            version, node, chain, network, state_sync, time,
        ))
    }

    #[wasm_bindgen(js_name = "getCurrentQuorumsInfo")]
    pub async fn get_current_quorums_info(&self) -> Result<CurrentQuorumsInfoWasm, WasmSdkError> {
        use dash_sdk::platform::FetchUnproved;
        use drive_proof_verifier::types::{
            CurrentQuorumsInfo as SdkCurrentQuorumsInfo, NoParamQuery,
        };

        let quorums_result =
            SdkCurrentQuorumsInfo::fetch_unproved(self.as_ref(), NoParamQuery {}).await?;

        // The result is Option<CurrentQuorumsInfo>
        if let Some(quorum_info) = quorums_result {
            // Convert the SDK response to our structure
            // Match quorum hashes with validator sets to get detailed information
            let quorums: Vec<QuorumInfoWasm> = quorum_info
                .quorum_hashes
                .into_iter()
                .map(|quorum_hash| {
                    // Try to find the corresponding validator set
                    let validator_set = quorum_info.validator_sets.iter().find(|vs| {
                        // Compare the quorum hash bytes directly

                        let vs_hash_bytes: &[u8] = vs.quorum_hash().as_ref();
                        vs_hash_bytes == &quorum_hash[..]
                    });

                    if let Some(vs) = validator_set {
                        let member_count = vs.members().len() as u32;

                        // Determine quorum type based on member count and quorum index
                        // This is an approximation based on common quorum sizes
                        // TODO: Get actual quorum type from the platform when available
                        let (quorum_type, threshold) = match member_count {
                            50..=70 => ("LLMQ_60_75".to_string(), (member_count * 75 / 100).max(1)),
                            90..=110 => {
                                ("LLMQ_100_67".to_string(), (member_count * 67 / 100).max(1))
                            }
                            350..=450 => {
                                ("LLMQ_400_60".to_string(), (member_count * 60 / 100).max(1))
                            }
                            _ => (
                                "LLMQ_TYPE_UNKNOWN".to_string(),
                                (member_count * 2 / 3).max(1),
                            ),
                        };

                        QuorumInfoWasm::new(
                            hex::encode(quorum_hash),
                            quorum_type,
                            member_count,
                            threshold,
                            true,
                        )
                    } else {
                        // No validator set found for this quorum hash
                        // TODO: This should not happen in normal circumstances. When the SDK
                        // provides complete quorum information, this fallback can be removed.
                        QuorumInfoWasm::new(
                            hex::encode(quorum_hash),
                            "LLMQ_TYPE_UNKNOWN".to_string(),
                            0,
                            0,
                            false,
                        )
                    }
                })
                .collect();

            Ok(CurrentQuorumsInfoWasm::new(
                quorums,
                quorum_info.last_platform_block_height,
            ))
        } else {
            // No quorum info available
            Ok(CurrentQuorumsInfoWasm::new(vec![], 0))
        }
    }

    #[wasm_bindgen(js_name = "getTotalCreditsInPlatform")]
    pub async fn get_total_credits_in_platform(&self) -> Result<BigInt, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{
            NoParamQuery, TotalCreditsInPlatform as TotalCreditsQuery,
        };

        let total_credits_result = TotalCreditsQuery::fetch(self.as_ref(), NoParamQuery {}).await?;

        // TotalCreditsInPlatform is likely a newtype wrapper around u64
        let credits_value = if let Some(credits) = total_credits_result {
            // Extract the inner value - assuming it has a field or can be dereferenced
            // We'll try to access it as a tuple struct
            credits.0
        } else {
            0
        };

        Ok(BigInt::from(credits_value))
    }

    #[wasm_bindgen(js_name = "getPrefundedSpecializedBalance")]
    pub async fn get_prefunded_specialized_balance(
        &self,
        identity_id: &str,
    ) -> Result<PrefundedSpecializedBalanceWasm, WasmSdkError> {
        use dash_sdk::platform::{Fetch, Identifier};
        use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;

        // Parse identity ID
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Fetch prefunded specialized balance
        let balance_result = PrefundedBalance::fetch(self.as_ref(), identity_identifier).await?;

        let balance_value = balance_result.map(|b| b.0).unwrap_or(0);

        Ok(PrefundedSpecializedBalanceWasm::new(
            identity_id.to_string(),
            balance_value,
        ))
    }

    #[wasm_bindgen(js_name = "waitForStateTransitionResult")]
    pub async fn wait_for_state_transition_result(
        &self,
        state_transition_hash: &str,
    ) -> Result<StateTransitionResultWasm, WasmSdkError> {
        use dapi_grpc::platform::v0::wait_for_state_transition_result_request::{
            Version, WaitForStateTransitionResultRequestV0,
        };
        use dapi_grpc::platform::v0::WaitForStateTransitionResultRequest;

        use dash_sdk::RequestSettings;
        use rs_dapi_client::DapiRequestExecutor;

        // Parse the hash from hex string to bytes
        let hash_bytes = hex::decode(state_transition_hash).map_err(|e| {
            WasmSdkError::invalid_argument(format!("Invalid state transition hash: {}", e))
        })?;

        // Create the gRPC request
        let request = WaitForStateTransitionResultRequest {
            version: Some(Version::V0(WaitForStateTransitionResultRequestV0 {
                state_transition_hash: hash_bytes,
                prove: self.prove(),
            })),
        };

        // Execute the request
        let response = self
            .as_ref()
            .execute(request, RequestSettings::default())
            .await
            .map_err(|e| {
                WasmSdkError::generic(format!("Failed to wait for state transition result: {}", e))
            })?;

        // Parse the response
        use dapi_grpc::platform::v0::wait_for_state_transition_result_response::{
            wait_for_state_transition_result_response_v0::Result as V0Result,
            Version as ResponseVersion,
        };

        let (status, error) = match response.inner.version {
            Some(ResponseVersion::V0(v0)) => match v0.result {
                Some(V0Result::Error(e)) => {
                    let error_message = format!("Code: {}, Message: {}", e.code, e.message);
                    ("ERROR".to_string(), Some(error_message))
                }
                Some(V0Result::Proof(_)) => {
                    // State transition was successful
                    ("SUCCESS".to_string(), None)
                }
                None => (
                    "UNKNOWN".to_string(),
                    Some("No result returned".to_string()),
                ),
            },
            None => (
                "UNKNOWN".to_string(),
                Some("No version in response".to_string()),
            ),
        };

        Ok(StateTransitionResultWasm::new(
            state_transition_hash.to_string(),
            status,
            error,
        ))
    }

    #[wasm_bindgen(js_name = "getPathElements")]
    pub async fn get_path_elements(
        &self,
        path: Vec<String>,
        keys: Vec<String>,
    ) -> Result<Array, WasmSdkError> {
        use dash_sdk::drive::grovedb::Element;
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::{Elements, KeysInPath};

        // Convert string path to byte vectors
        // Path elements can be either numeric values (like "96" for Balances) or string keys
        let path_bytes: Vec<Vec<u8>> = path
            .iter()
            .map(|p| {
                // Try to parse as a u8 number first (for root tree paths)
                if let Ok(num) = p.parse::<u8>() {
                    vec![num]
                } else {
                    // Otherwise treat as a string key
                    p.as_bytes().to_vec()
                }
            })
            .collect();

        // Convert string keys to byte vectors
        let key_bytes: Vec<Vec<u8>> = keys.iter().map(|k| k.as_bytes().to_vec()).collect();

        // Create the query
        let query = KeysInPath {
            path: path_bytes,
            keys: key_bytes,
        };

        // Fetch path elements
        let path_elements_result: Elements = Element::fetch_many(self.as_ref(), query).await?;

        // Convert the result to our response format
        let elements_array = Array::new();
        for key in keys {
            let value = path_elements_result
                .get(key.as_bytes())
                .and_then(|element_opt| element_opt.as_ref())
                .and_then(|element| {
                    element.as_item_bytes().ok().map(|bytes| {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    })
                });

            elements_array.push(&JsValue::from(PathElementWasm::new(vec![key], value)));
        }

        Ok(elements_array)
    }

    // Proof versions for system queries

    #[wasm_bindgen(js_name = "getTotalCreditsInPlatformWithProofInfo")]
    pub async fn get_total_credits_in_platform_with_proof_info(
        &self,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{
            NoParamQuery, TotalCreditsInPlatform as TotalCreditsQuery,
        };

        let (total_credits_result, metadata, proof) =
            TotalCreditsQuery::fetch_with_metadata_and_proof(self.as_ref(), NoParamQuery {}, None)
                .await?;

        let data = total_credits_result
            .map(|credits| JsValue::from(BigInt::from(credits.0)))
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getPrefundedSpecializedBalanceWithProofInfo")]
    pub async fn get_prefunded_specialized_balance_with_proof_info(
        &self,
        identity_id: &str,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::platform::{Fetch, Identifier};
        use drive_proof_verifier::types::PrefundedSpecializedBalance as PrefundedBalance;

        // Parse identity ID
        let identity_identifier = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        // Fetch prefunded specialized balance with proof
        let (balance_result, metadata, proof) = PrefundedBalance::fetch_with_metadata_and_proof(
            self.as_ref(),
            identity_identifier,
            None,
        )
        .await?;

        let data = balance_result
            .map(|balance| {
                JsValue::from(PrefundedSpecializedBalanceWasm::new(
                    identity_id.to_string(),
                    balance.0,
                ))
            })
            .unwrap_or(JsValue::UNDEFINED);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }

    #[wasm_bindgen(js_name = "getPathElementsWithProofInfo")]
    pub async fn get_path_elements_with_proof_info(
        &self,
        path: Vec<String>,
        keys: Vec<String>,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        use dash_sdk::drive::grovedb::Element;
        use dash_sdk::platform::FetchMany;
        use drive_proof_verifier::types::KeysInPath;

        // Convert string path to byte vectors
        // Path elements can be either numeric values (like "96" for Balances) or string keys
        let path_bytes: Vec<Vec<u8>> = path
            .iter()
            .map(|p| {
                // Try to parse as a u8 number first (for root tree paths)
                if let Ok(num) = p.parse::<u8>() {
                    vec![num]
                } else {
                    // Otherwise treat as a string key
                    p.as_bytes().to_vec()
                }
            })
            .collect();

        // Convert string keys to byte vectors
        let key_bytes: Vec<Vec<u8>> = keys.iter().map(|k| k.as_bytes().to_vec()).collect();

        // Create the query
        let query = KeysInPath {
            path: path_bytes,
            keys: key_bytes,
        };

        // Fetch path elements with proof
        let (path_elements_result, metadata, proof) =
            Element::fetch_many_with_metadata_and_proof(self.as_ref(), query, None).await?;

        let elements_array = Array::new();
        for key in keys {
            let value = path_elements_result
                .get(key.as_bytes())
                .and_then(|element_opt| element_opt.as_ref())
                .and_then(|element| {
                    element.as_item_bytes().ok().map(|bytes| {
                        use base64::Engine;
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    })
                });

            elements_array.push(&JsValue::from(PathElementWasm::new(vec![key], value)));
        }

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            elements_array,
            metadata,
            proof,
        ))
    }
}
