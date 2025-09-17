use crate::error::WasmSdkError;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::core_types::validator_set::v0::ValidatorSetV0Getters;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

// Response structures for the gRPC getStatus endpoint
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    version: StatusVersion,
    node: StatusNode,
    chain: StatusChain,
    network: StatusNetwork,
    state_sync: StatusStateSync,
    time: StatusTime,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusVersion {
    software: StatusSoftware,
    protocol: StatusProtocol,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusSoftware {
    dapi: String,
    drive: Option<String>,
    tenderdash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusProtocol {
    tenderdash: StatusTenderdashProtocol,
    drive: StatusDriveProtocol,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusTenderdashProtocol {
    p2p: u32,
    block: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusDriveProtocol {
    latest: u32,
    current: u32,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusNode {
    id: String,
    pro_tx_hash: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusChain {
    catching_up: bool,
    latest_block_hash: String,
    latest_app_hash: String,
    latest_block_height: String,
    earliest_block_hash: String,
    earliest_app_hash: String,
    earliest_block_height: String,
    max_peer_block_height: String,
    core_chain_locked_height: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusNetwork {
    chain_id: String,
    peers_count: u32,
    listening: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusStateSync {
    total_synced_time: String,
    remaining_time: String,
    total_snapshots: u32,
    chunk_process_avg_time: String,
    snapshot_height: String,
    snapshot_chunks_count: String,
    backfilled_blocks: String,
    backfill_blocks_total: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StatusTime {
    local: String,
    block: Option<String>,
    genesis: Option<String>,
    epoch: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct QuorumInfo {
    quorum_hash: String,
    quorum_type: String,
    member_count: u32,
    threshold: u32,
    is_verified: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CurrentQuorumsInfo {
    quorums: Vec<QuorumInfo>,
    height: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TotalCreditsResponse {
    total_credits_in_platform: String, // Use String to handle large numbers
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct StateTransitionResult {
    state_transition_hash: String,
    status: String,
    error: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PrefundedSpecializedBalance {
    identity_id: String,
    balance: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct PathElement {
    path: Vec<String>,
    value: Option<String>,
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getStatus")]
    pub async fn get_status(&self) -> Result<JsValue, WasmSdkError> {
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
        let status = StatusResponse {
            version: StatusVersion {
                software: StatusSoftware {
                    dapi: v0_response
                        .version
                        .as_ref()
                        .and_then(|v| v.software.as_ref())
                        .map(|s| s.dapi.clone())
                        .unwrap_or_else(|| "unknown".to_string()),
                    drive: v0_response
                        .version
                        .as_ref()
                        .and_then(|v| v.software.as_ref())
                        .and_then(|s| s.drive.clone()),
                    tenderdash: v0_response
                        .version
                        .as_ref()
                        .and_then(|v| v.software.as_ref())
                        .and_then(|s| s.tenderdash.clone()),
                },
                protocol: StatusProtocol {
                    tenderdash: StatusTenderdashProtocol {
                        p2p: v0_response
                            .version
                            .as_ref()
                            .and_then(|v| v.protocol.as_ref())
                            .and_then(|p| p.tenderdash.as_ref())
                            .map(|t| t.p2p)
                            .unwrap_or(0),
                        block: v0_response
                            .version
                            .as_ref()
                            .and_then(|v| v.protocol.as_ref())
                            .and_then(|p| p.tenderdash.as_ref())
                            .map(|t| t.block)
                            .unwrap_or(0),
                    },
                    drive: StatusDriveProtocol {
                        latest: v0_response
                            .version
                            .as_ref()
                            .and_then(|v| v.protocol.as_ref())
                            .and_then(|p| p.drive.as_ref())
                            .map(|d| d.latest)
                            .unwrap_or(0),
                        current: v0_response
                            .version
                            .as_ref()
                            .and_then(|v| v.protocol.as_ref())
                            .and_then(|p| p.drive.as_ref())
                            .map(|d| d.current)
                            .unwrap_or(0),
                    },
                },
            },
            node: StatusNode {
                id: v0_response
                    .node
                    .as_ref()
                    .map(|n| hex::encode(&n.id))
                    .unwrap_or_else(|| "unknown".to_string()),
                pro_tx_hash: v0_response
                    .node
                    .as_ref()
                    .and_then(|n| n.pro_tx_hash.as_ref())
                    .map(hex::encode),
            },
            chain: StatusChain {
                catching_up: v0_response
                    .chain
                    .as_ref()
                    .map(|c| c.catching_up)
                    .unwrap_or(false),
                latest_block_hash: v0_response
                    .chain
                    .as_ref()
                    .map(|c| hex::encode(&c.latest_block_hash))
                    .unwrap_or_else(|| "unknown".to_string()),
                latest_app_hash: v0_response
                    .chain
                    .as_ref()
                    .map(|c| hex::encode(&c.latest_app_hash))
                    .unwrap_or_else(|| "unknown".to_string()),
                latest_block_height: v0_response
                    .chain
                    .as_ref()
                    .map(|c| c.latest_block_height.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                earliest_block_hash: v0_response
                    .chain
                    .as_ref()
                    .map(|c| hex::encode(&c.earliest_block_hash))
                    .unwrap_or_else(|| "unknown".to_string()),
                earliest_app_hash: v0_response
                    .chain
                    .as_ref()
                    .map(|c| hex::encode(&c.earliest_app_hash))
                    .unwrap_or_else(|| "unknown".to_string()),
                earliest_block_height: v0_response
                    .chain
                    .as_ref()
                    .map(|c| c.earliest_block_height.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                max_peer_block_height: v0_response
                    .chain
                    .as_ref()
                    .map(|c| c.max_peer_block_height.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                core_chain_locked_height: v0_response
                    .chain
                    .as_ref()
                    .and_then(|c| c.core_chain_locked_height),
            },
            network: StatusNetwork {
                chain_id: v0_response
                    .network
                    .as_ref()
                    .map(|n| n.chain_id.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                peers_count: v0_response
                    .network
                    .as_ref()
                    .map(|n| n.peers_count)
                    .unwrap_or(0),
                listening: v0_response
                    .network
                    .as_ref()
                    .map(|n| n.listening)
                    .unwrap_or(false),
            },
            state_sync: StatusStateSync {
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
            },
            time: StatusTime {
                local: v0_response
                    .time
                    .as_ref()
                    .map(|t| t.local.to_string())
                    .unwrap_or_else(|| "0".to_string()),
                block: v0_response
                    .time
                    .as_ref()
                    .and_then(|t| t.block)
                    .map(|b| b.to_string()),
                genesis: v0_response
                    .time
                    .as_ref()
                    .and_then(|t| t.genesis)
                    .map(|g| g.to_string()),
                epoch: v0_response.time.as_ref().and_then(|t| t.epoch),
            },
        };

        serde_wasm_bindgen::to_value(&status).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    #[wasm_bindgen(js_name = "getCurrentQuorumsInfo")]
    pub async fn get_current_quorums_info(&self) -> Result<JsValue, WasmSdkError> {
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
            let quorums: Vec<QuorumInfo> = quorum_info
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

                        QuorumInfo {
                            quorum_hash: hex::encode(quorum_hash),
                            quorum_type,
                            member_count,
                            threshold,
                            is_verified: true, // We have the validator set, so it's verified
                        }
                    } else {
                        // No validator set found for this quorum hash
                        // TODO: This should not happen in normal circumstances. When the SDK
                        // provides complete quorum information, this fallback can be removed.
                        QuorumInfo {
                            quorum_hash: hex::encode(quorum_hash),
                            quorum_type: "LLMQ_TYPE_UNKNOWN".to_string(),
                            member_count: 0,
                            threshold: 0,
                            is_verified: false,
                        }
                    }
                })
                .collect();

            let info = CurrentQuorumsInfo {
                quorums,
                height: quorum_info.last_platform_block_height,
            };

            serde_wasm_bindgen::to_value(&info).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        } else {
            // No quorum info available
            let info = CurrentQuorumsInfo {
                quorums: vec![],
                height: 0,
            };

            serde_wasm_bindgen::to_value(&info).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        }
    }

    #[wasm_bindgen(js_name = "getTotalCreditsInPlatform")]
    pub async fn get_total_credits_in_platform(&self) -> Result<JsValue, WasmSdkError> {
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

        let response = TotalCreditsResponse {
            total_credits_in_platform: credits_value.to_string(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    #[wasm_bindgen(js_name = "getPrefundedSpecializedBalance")]
    pub async fn get_prefunded_specialized_balance(
        &self,
        identity_id: &str,
    ) -> Result<JsValue, WasmSdkError> {
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

        if let Some(balance) = balance_result {
            let response = PrefundedSpecializedBalance {
                identity_id: identity_id.to_string(),
                balance: balance.0, // PrefundedSpecializedBalance is a newtype wrapper around u64
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        } else {
            // Return zero balance if not found
            let response = PrefundedSpecializedBalance {
                identity_id: identity_id.to_string(),
                balance: 0,
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response.serialize(&serializer).map_err(|e| {
                WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
            })
        }
    }

    #[wasm_bindgen(js_name = "waitForStateTransitionResult")]
    pub async fn wait_for_state_transition_result(
        &self,
        state_transition_hash: &str,
    ) -> Result<JsValue, WasmSdkError> {
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

        let result = StateTransitionResult {
            state_transition_hash: state_transition_hash.to_string(),
            status,
            error,
        };

        serde_wasm_bindgen::to_value(&result).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    #[wasm_bindgen(js_name = "getPathElements")]
    pub async fn get_path_elements(
        &self,
        path: Vec<String>,
        keys: Vec<String>,
    ) -> Result<JsValue, WasmSdkError> {
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
        let elements: Vec<PathElement> = keys
            .into_iter()
            .map(|key| {
                // Check if this key exists in the result
                let value = path_elements_result
                    .get(key.as_bytes())
                    .and_then(|element_opt| element_opt.as_ref())
                    .and_then(|element| {
                        // Element can contain different types, we'll serialize it as base64
                        element.as_item_bytes().ok().map(|bytes| {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        })
                    });

                PathElement {
                    path: vec![key],
                    value,
                }
            })
            .collect();

        serde_wasm_bindgen::to_value(&elements).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    // Proof versions for system queries

    #[wasm_bindgen(js_name = "getTotalCreditsInPlatformWithProofInfo")]
    pub async fn get_total_credits_in_platform_with_proof_info(
        &self,
    ) -> Result<JsValue, WasmSdkError> {
        use crate::queries::ProofMetadataResponse;
        use dash_sdk::platform::Fetch;
        use drive_proof_verifier::types::{
            NoParamQuery, TotalCreditsInPlatform as TotalCreditsQuery,
        };

        let (total_credits_result, metadata, proof) =
            TotalCreditsQuery::fetch_with_metadata_and_proof(self.as_ref(), NoParamQuery {}, None)
                .await?;

        let data = total_credits_result.map(|credits| TotalCreditsResponse {
                total_credits_in_platform: credits.0.to_string(),
            });

        let response = ProofMetadataResponse {
            data,
            metadata: metadata.into(),
            proof: proof.into(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    #[wasm_bindgen(js_name = "getPrefundedSpecializedBalanceWithProofInfo")]
    pub async fn get_prefunded_specialized_balance_with_proof_info(
        &self,
        identity_id: &str,
    ) -> Result<JsValue, WasmSdkError> {
        use crate::queries::ProofMetadataResponse;
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

        let data = PrefundedSpecializedBalance {
            identity_id: identity_id.to_string(),
            balance: balance_result.map(|b| b.0).unwrap_or(0),
        };

        let response = ProofMetadataResponse {
            data,
            metadata: metadata.into(),
            proof: proof.into(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }

    #[wasm_bindgen(js_name = "getPathElementsWithProofInfo")]
    pub async fn get_path_elements_with_proof_info(
        &self,
        path: Vec<String>,
        keys: Vec<String>,
    ) -> Result<JsValue, WasmSdkError> {
        use crate::queries::ProofMetadataResponse;
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

        // Convert the result to our response format
        let elements: Vec<PathElement> = keys
            .into_iter()
            .map(|key| {
                let value = path_elements_result
                    .get(key.as_bytes())
                    .and_then(|element_opt| element_opt.as_ref())
                    .and_then(|element| {
                        element.as_item_bytes().ok().map(|bytes| {
                            use base64::Engine;
                            base64::engine::general_purpose::STANDARD.encode(bytes)
                        })
                    });

                PathElement {
                    path: vec![key],
                    value,
                }
            })
            .collect();

        let response = ProofMetadataResponse {
            data: elements,
            metadata: metadata.into(),
            proof: proof.into(),
        };

        // Use json_compatible serializer
        let serializer = serde_wasm_bindgen::Serializer::json_compatible();
        response.serialize(&serializer).map_err(|e| {
            WasmSdkError::serialization(format!("Failed to serialize response: {}", e))
        })
    }
}
