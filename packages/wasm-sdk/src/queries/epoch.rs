use crate::error::WasmSdkError;
use crate::queries::{ProofInfoWasm, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use dash_sdk::dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dash_sdk::dpp::dashcore::hashes::Hash;
use dash_sdk::dpp::dashcore::ProTxHash;
use dash_sdk::platform::fetch_current_no_parameters::FetchCurrent;
use dash_sdk::platform::types::proposed_blocks::ProposedBlockCountEx;
use dash_sdk::platform::{FetchMany, LimitQuery};
use js_sys::{BigInt, Map, Number};
use std::str::FromStr;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::epoch::{ExtendedEpochInfoWasm, FinalizedEpochInfoWasm};
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(js_name = "EpochsProofResponse")]
#[derive(Clone)]
pub struct EpochsProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub epochs: Map,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "EpochProofResponse")]
#[derive(Clone)]
pub struct EpochProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub epoch: ExtendedEpochInfoWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "FinalizedEpochsProofResponse")]
#[derive(Clone)]
pub struct FinalizedEpochsProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub epochs: Map,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getEpochsInfo")]
    pub async fn get_epochs_info(
        &self,
        start_epoch: Option<u16>,
        count: Option<u32>,
        ascending: Option<bool>,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::types::epoch::EpochQuery;

        let query = LimitQuery {
            query: EpochQuery {
                start: start_epoch,
                ascending: ascending.unwrap_or(true),
            },
            limit: count,
            start_info: None,
        };

        let epochs_result: drive_proof_verifier::types::ExtendedEpochInfos =
            ExtendedEpochInfo::fetch_many(self.as_ref(), query).await?;

        let epochs_map = Map::new();

        for (epoch_index, epoch_info) in epochs_result {
            let key = Number::from(epoch_index as u32);
            let value = epoch_info.map(ExtendedEpochInfoWasm::from);
            epochs_map.set(&key.into(), &JsValue::from(value));
        }

        Ok(epochs_map)
    }

    #[wasm_bindgen(js_name = "getFinalizedEpochInfos")]
    pub async fn get_finalized_epoch_infos(
        &self,
        start_epoch: Option<u16>,
        count: Option<u32>,
        ascending: Option<bool>,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::types::finalized_epoch::FinalizedEpochQuery;

        if start_epoch.is_none() {
            return Err(WasmSdkError::invalid_argument(
                "start_epoch is required for finalized epoch queries",
            ));
        }

        let start = start_epoch.unwrap();
        let is_ascending = ascending.unwrap_or(true);
        let limit = count.unwrap_or(100);

        // Ensure limit is at least 1 to avoid underflow
        let limit = limit.max(1);

        // Calculate end epoch based on direction and limit
        let end_epoch = if is_ascending {
            start.saturating_add((limit - 1) as u16)
        } else {
            start.saturating_sub((limit - 1) as u16)
        };

        let query = if is_ascending {
            FinalizedEpochQuery {
                start_epoch_index: start,
                start_epoch_index_included: true,
                end_epoch_index: end_epoch,
                end_epoch_index_included: true,
            }
        } else {
            FinalizedEpochQuery {
                start_epoch_index: end_epoch,
                start_epoch_index_included: true,
                end_epoch_index: start,
                end_epoch_index_included: true,
            }
        };

        let epochs_result: drive_proof_verifier::types::FinalizedEpochInfos =
            dash_sdk::dpp::block::finalized_epoch_info::FinalizedEpochInfo::fetch_many(
                self.as_ref(),
                query,
            )
            .await?;

        let epochs_map = Map::new();
        for (epoch_index, epoch_info) in epochs_result {
            let key = Number::from(epoch_index as u32);
            let value = epoch_info.map(FinalizedEpochInfoWasm::from);
            epochs_map.set(&key.into(), &JsValue::from(value));
        }

        Ok(epochs_map)
    }

    #[wasm_bindgen(js_name = "getEvonodesProposedEpochBlocksByIds")]
    pub async fn get_evonodes_proposed_epoch_blocks_by_ids(
        &self,
        epoch: u16,
        ids: Vec<String>,
    ) -> Result<Map, WasmSdkError> {
        use drive_proof_verifier::types::ProposerBlockCountById;

        // Parse the ProTxHash strings
        let pro_tx_hashes: Vec<ProTxHash> = ids
            .into_iter()
            .map(|hash_str| {
                ProTxHash::from_str(&hash_str).map_err(|e| {
                    WasmSdkError::invalid_argument(format!(
                        "Invalid ProTxHash '{}': {}",
                        hash_str, e
                    ))
                })
            })
            .collect::<Result<Vec<_>, WasmSdkError>>()?;

        // Use FetchMany to get block counts for specific IDs
        let counts =
            ProposerBlockCountById::fetch_many(self.as_ref(), (epoch, pro_tx_hashes)).await?;

        let map = Map::new();

        for (identifier, count) in counts.0 {
            let key = JsValue::from(IdentifierWasm::from(identifier));
            map.set(&key, &JsValue::from(BigInt::from(count)));
        }

        Ok(map)
    }

    #[wasm_bindgen(js_name = "getEvonodesProposedEpochBlocksByRange")]
    pub async fn get_evonodes_proposed_epoch_blocks_by_range(
        &self,
        epoch: u16,
        limit: Option<u32>,
        start_after: Option<String>,
        order_ascending: Option<bool>,
    ) -> Result<Map, WasmSdkError> {
        use dash_sdk::platform::QueryStartInfo;
        use drive_proof_verifier::types::ProposerBlockCounts;

        // Parse start_after if provided
        let start_info = if let Some(start) = start_after {
            let pro_tx_hash = ProTxHash::from_str(&start).map_err(|e| {
                WasmSdkError::invalid_argument(format!("Invalid start_after ProTxHash: {}", e))
            })?;
            Some(QueryStartInfo {
                start_key: pro_tx_hash.to_byte_array().to_vec(),
                start_included: false,
            })
        } else {
            None
        };

        let counts_result = ProposerBlockCounts::fetch_proposed_blocks_by_range(
            self.as_ref(),
            Some(epoch),
            limit,
            start_info,
        )
        .await?;

        let map = Map::new();
        for (identifier, count) in counts_result.0 {
            let key = JsValue::from(IdentifierWasm::from(identifier));
            map.set(&key, &JsValue::from(BigInt::from(count)));
        }

        Ok(map)
    }

    #[wasm_bindgen(js_name = "getCurrentEpoch")]
    pub async fn get_current_epoch(&self) -> Result<ExtendedEpochInfoWasm, WasmSdkError> {
        let epoch = ExtendedEpochInfo::fetch_current(self.as_ref()).await?;

        Ok(ExtendedEpochInfoWasm::from(epoch))
    }

    #[wasm_bindgen(js_name = "getEpochsInfoWithProofInfo")]
    pub async fn get_epochs_info_with_proof_info(
        &self,
        start_epoch: Option<u16>,
        count: Option<u32>,
        ascending: Option<bool>,
    ) -> Result<EpochsProofResponseWasm, WasmSdkError> {
        use dash_sdk::platform::types::epoch::EpochQuery;

        let query = LimitQuery {
            query: EpochQuery {
                start: start_epoch,
                ascending: ascending.unwrap_or(true),
            },
            limit: count,
            start_info: None,
        };

        let (epochs_result, metadata, proof) =
            ExtendedEpochInfo::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let epochs_map = Map::new();
        for (epoch_index, epoch_info) in epochs_result {
            let key = Number::from(epoch_index as u32);
            let value = epoch_info.map(ExtendedEpochInfoWasm::from);
            epochs_map.set(&key.into(), &JsValue::from(value));
        }

        Ok(EpochsProofResponseWasm {
            epochs: epochs_map,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getCurrentEpochWithProofInfo")]
    pub async fn get_current_epoch_with_proof_info(
        &self,
    ) -> Result<EpochProofResponseWasm, WasmSdkError> {
        let (epoch, metadata, proof) =
            ExtendedEpochInfo::fetch_current_with_metadata_and_proof(self.as_ref()).await?;

        Ok(EpochProofResponseWasm {
            epoch: ExtendedEpochInfoWasm::from(epoch),
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    // Additional proof info versions for epoch queries

    #[wasm_bindgen(js_name = "getFinalizedEpochInfosWithProofInfo")]
    pub async fn get_finalized_epoch_infos_with_proof_info(
        &self,
        start_epoch: Option<u16>,
        count: Option<u32>,
        ascending: Option<bool>,
    ) -> Result<FinalizedEpochsProofResponseWasm, WasmSdkError> {
        use dash_sdk::platform::types::finalized_epoch::FinalizedEpochQuery;

        if start_epoch.is_none() {
            return Err(WasmSdkError::invalid_argument(
                "start_epoch is required for finalized epoch queries",
            ));
        }

        let start = start_epoch.unwrap();
        let is_ascending = ascending.unwrap_or(true);
        let limit = count.unwrap_or(100);

        // Ensure limit is at least 1 to avoid underflow
        let limit = limit.max(1);

        // Calculate end epoch based on direction and limit
        let end_epoch = if is_ascending {
            start.saturating_add((limit - 1) as u16)
        } else {
            start.saturating_sub((limit - 1) as u16)
        };

        let query = if is_ascending {
            FinalizedEpochQuery {
                start_epoch_index: start,
                start_epoch_index_included: true,
                end_epoch_index: end_epoch,
                end_epoch_index_included: true,
            }
        } else {
            FinalizedEpochQuery {
                start_epoch_index: end_epoch,
                start_epoch_index_included: true,
                end_epoch_index: start,
                end_epoch_index_included: true,
            }
        };

        let (epochs_result, metadata, proof) = dash_sdk::dpp::block::finalized_epoch_info::FinalizedEpochInfo::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
            .await?;

        let epochs_map = Map::new();
        for (index, epoch) in epochs_result {
            let key = Number::from(index as u32);
            let value = epoch.map(FinalizedEpochInfoWasm::from);
            epochs_map.set(&key.into(), &JsValue::from(value));
        }

        Ok(FinalizedEpochsProofResponseWasm {
            epochs: epochs_map,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getEvonodesProposedEpochBlocksByIdsWithProofInfo")]
    pub async fn get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info(
        &self,
        epoch: u16,
        pro_tx_hashes: Vec<String>,
    ) -> Result<JsValue, WasmSdkError> {
        // TODO: Implement once SDK Query trait is implemented for ProposerBlockCountById
        // Currently not supported due to query format issues
        let _ = (self, epoch, pro_tx_hashes); // Parameters will be used when implemented
        Err(WasmSdkError::generic(
            "get_evonodes_proposed_epoch_blocks_by_ids_with_proof_info is not yet implemented",
        ))
    }

    #[wasm_bindgen(js_name = "getEvonodesProposedEpochBlocksByRangeWithProofInfo")]
    pub async fn get_evonodes_proposed_epoch_blocks_by_range_with_proof_info(
        &self,
        epoch: u16,
        limit: Option<u32>,
        start_after: Option<String>,
        order_ascending: Option<bool>,
    ) -> Result<JsValue, WasmSdkError> {
        // TODO: Implement once SDK Query trait is implemented for ProposerBlockCountByRange
        // Currently not supported due to query format issues
        let _ = (self, epoch, limit, start_after, order_ascending); // Parameters will be used when implemented
        Err(WasmSdkError::generic(
            "get_evonodes_proposed_epoch_blocks_by_range_with_proof_info is not yet implemented",
        ))
    }
}
