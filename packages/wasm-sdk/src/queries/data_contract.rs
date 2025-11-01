use crate::queries::utils::deserialize_required_query;
use crate::queries::{ProofInfoWasm, ResponseMetadataWasm};
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::platform::query::LimitQuery;
use dash_sdk::platform::{DataContract, Fetch, FetchMany, Identifier};
use drive_proof_verifier::types::{DataContractHistory, DataContracts};
use js_sys::{BigInt, Map};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::DataContractWasm;

#[wasm_bindgen(js_name = "DataContractProofResponse")]
#[derive(Clone)]
pub struct DataContractProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub contract: DataContractWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "DataContractHistoryProofResponse")]
#[derive(Clone)]
pub struct DataContractHistoryProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub history: Map,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(js_name = "DataContractsProofResponse")]
#[derive(Clone)]
pub struct DataContractsProofResponseWasm {
    #[wasm_bindgen(getter_with_clone)]
    pub contracts: Map,
    #[wasm_bindgen(getter_with_clone)]
    pub metadata: ResponseMetadataWasm,
    #[wasm_bindgen(getter_with_clone)]
    pub proof: ProofInfoWasm,
}

#[wasm_bindgen(typescript_custom_section)]
const DATA_CONTRACT_HISTORY_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving data contract history.
 */
export interface DataContractHistoryQuery {
  /**
   * Data contract identifier (base58 string).
   */
  dataContractId: string;

  /**
   * Maximum number of entries to return.
   * @default undefined
   */
  limit?: number;

  /**
   * Millisecond timestamp (inclusive) to start from.
   * @default 0
   */
  startAtMs?: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DataContractHistoryQuery")]
    pub type DataContractHistoryQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DataContractHistoryQueryInput {
    data_contract_id: String,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    start_at_ms: Option<u64>,
}

struct DataContractHistoryQueryParsed {
    contract_id: Identifier,
    limit: Option<u32>,
    start_at_ms: Option<u64>,
}

fn parse_data_contract_history_query(
    query: DataContractHistoryQueryJs,
) -> Result<DataContractHistoryQueryParsed, WasmSdkError> {
    let input: DataContractHistoryQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "data contract history query",
    )?;

    let contract_id = Identifier::from_string(
        &input.data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e)))?;

    Ok(DataContractHistoryQueryParsed {
        contract_id,
        limit: input.limit,
        start_at_ms: input.start_at_ms,
    })
}

fn build_limit_query(params: &DataContractHistoryQueryParsed) -> LimitQuery<(Identifier, u64)> {
    LimitQuery {
        query: (params.contract_id.clone(), params.start_at_ms.unwrap_or(0)),
        start_info: None,
        limit: params.limit,
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getDataContract")]
    pub async fn get_data_contract(
        &self,
        base58_id: &str,
    ) -> Result<Option<DataContractWasm>, WasmSdkError> {
        let id = Identifier::from_string(
            base58_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e)))?;

        let data_contract = DataContract::fetch_by_identifier(self.as_ref(), id)
            .await?
            .map(DataContractWasm::from);

        Ok(data_contract)
    }

    #[wasm_bindgen(js_name = "getDataContractWithProofInfo")]
    pub async fn get_data_contract_with_proof_info(
        &self,
        base58_id: &str,
    ) -> Result<DataContractProofResponseWasm, WasmSdkError> {
        let id = Identifier::from_string(
            base58_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e)))?;

        let (contract, metadata, proof) =
            DataContract::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        match contract {
            Some(contract) => Ok(DataContractProofResponseWasm {
                contract: DataContractWasm::from(contract),
                metadata: metadata.into(),
                proof: proof.into(),
            }),
            None => Err(WasmSdkError::not_found("Data contract not found")),
        }
    }

    #[wasm_bindgen(js_name = "getDataContractHistory")]
    pub async fn get_data_contract_history(
        &self,
        query: DataContractHistoryQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let params = parse_data_contract_history_query(query)?;
        let limit_query = build_limit_query(&params);

        let history_result = DataContractHistory::fetch(self.as_ref(), limit_query).await?;

        let history_map = Map::new();

        if let Some(history) = history_result {
            for (block_time_ms, contract) in history {
                let contract_js = JsValue::from(DataContractWasm::from(contract));
                let key = JsValue::from(BigInt::from(block_time_ms));

                history_map.set(&key, &contract_js);
            }
        }

        Ok(history_map)
    }

    #[wasm_bindgen(js_name = "getDataContracts")]
    pub async fn get_data_contracts(&self, ids: Vec<String>) -> Result<Map, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Result<Vec<Identifier>, WasmSdkError> = ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e))
                })
            })
            .collect();
        let identifiers = identifiers?;

        // Fetch all contracts
        let contracts_result: DataContracts =
            DataContract::fetch_many(self.as_ref(), identifiers).await?;

        let contracts_map = Map::new();

        for (id, contract) in contracts_result {
            let key = JsValue::from(IdentifierWasm::from(id));
            let value = contract.map(DataContractWasm::from);
            contracts_map.set(&key, &JsValue::from(value));
        }

        Ok(contracts_map)
    }

    // Proof info versions for data contract queries

    #[wasm_bindgen(js_name = "getDataContractHistoryWithProofInfo")]
    pub async fn get_data_contract_history_with_proof_info(
        &self,
        query: DataContractHistoryQueryJs,
    ) -> Result<DataContractHistoryProofResponseWasm, WasmSdkError> {
        let params = parse_data_contract_history_query(query)?;
        let limit_query = build_limit_query(&params);

        let (history_result, metadata, proof) =
            DataContractHistory::fetch_with_metadata_and_proof(self.as_ref(), limit_query, None)
                .await?;

        let history_map = Map::new();

        if let Some(history) = history_result {
            for (block_time_ms, contract) in history {
                let contract_js = JsValue::from(DataContractWasm::from(contract));
                let key = JsValue::from(BigInt::from(block_time_ms));

                history_map.set(&key, &contract_js);
            }
        }

        Ok(DataContractHistoryProofResponseWasm {
            history: history_map,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }

    #[wasm_bindgen(js_name = "getDataContractsWithProofInfo")]
    pub async fn get_data_contracts_with_proof_info(
        &self,
        ids: Vec<String>,
    ) -> Result<DataContractsProofResponseWasm, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Result<Vec<Identifier>, WasmSdkError> = ids
            .iter()
            .map(|id| {
                Identifier::from_string(
                    id,
                    dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
                )
                .map_err(|e| {
                    WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", e))
                })
            })
            .collect();
        let identifiers = identifiers?;

        // Fetch all contracts with proof
        let (contracts_result, metadata, proof) =
            DataContract::fetch_many_with_metadata_and_proof(self.as_ref(), identifiers, None)
                .await?;

        let contracts_map = Map::new();

        for (id, contract_opt) in contracts_result {
            let key = JsValue::from(IdentifierWasm::from(id));
            let value = contract_opt.map(DataContractWasm::from);

            contracts_map.set(&key, &JsValue::from(value));
        }

        Ok(DataContractsProofResponseWasm {
            contracts: contracts_map,
            metadata: metadata.into(),
            proof: proof.into(),
        })
    }
}
