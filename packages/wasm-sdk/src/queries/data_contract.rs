use crate::queries::utils::deserialize_required_query;
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::platform::query::LimitQuery;
use dash_sdk::platform::{DataContract, Fetch, FetchMany, Identifier};
use drive_proof_verifier::types::{DataContractHistory, DataContracts};
use js_sys::{BigInt, Map};
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::{IdentifierLikeArrayJs, IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::utils::try_to_vec;
use wasm_dpp2::DataContractWasm;

#[wasm_bindgen(typescript_custom_section)]
const DATA_CONTRACT_HISTORY_QUERY_TS: &'static str = r#"
/**
 * Query parameters for retrieving data contract history.
 */
export interface DataContractHistoryQuery {
  /**
   * Data contract identifier.
   */
  dataContractId: IdentifierLike

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
    data_contract_id: IdentifierWasm,
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
    let DataContractHistoryQueryInput {
        data_contract_id,
        limit,
        start_at_ms,
    } = input;

    let contract_id: Identifier = data_contract_id.into();

    Ok(DataContractHistoryQueryParsed {
        contract_id,
        limit,
        start_at_ms,
    })
}

fn build_limit_query(params: &DataContractHistoryQueryParsed) -> LimitQuery<(Identifier, u64)> {
    LimitQuery {
        query: (params.contract_id, params.start_at_ms.unwrap_or(0)),
        start_info: None,
        limit: params.limit,
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getDataContract")]
    pub async fn get_data_contract(
        &self,
        #[wasm_bindgen(js_name = "contractId")]
        contract_id: IdentifierLikeJs,
    ) -> Result<Option<DataContractWasm>, WasmSdkError> {
        let id: Identifier = contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err)))?;

        let data_contract = DataContract::fetch_by_identifier(self.as_ref(), id)
            .await?
            .map(DataContractWasm::from);

        Ok(data_contract)
    }

    #[wasm_bindgen(
        js_name = "getDataContractWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<DataContract>"
    )]
    pub async fn get_data_contract_with_proof_info(
        &self,
        #[wasm_bindgen(js_name = "contractId")]
        contract_id: IdentifierLikeJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let id: Identifier = contract_id.try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid data contract ID: {}", err)))?;

        let (contract, metadata, proof) =
            DataContract::fetch_with_metadata_and_proof(self.as_ref(), id, None).await?;

        contract
            .map(|contract| {
                ProofMetadataResponseWasm::from_sdk_parts(
                    DataContractWasm::from(contract),
                    metadata,
                    proof,
                )
            })
            .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))
    }

    #[wasm_bindgen(
        js_name = "getDataContractHistory",
        unchecked_return_type = "Map<bigint, DataContract>"
    )]
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

    #[wasm_bindgen(
        js_name = "getDataContracts",
        unchecked_return_type = "Map<Identifier, DataContract | undefined>"
    )]
    pub async fn get_data_contracts(
        &self,
        ids: IdentifierLikeArrayJs,
    ) -> Result<Map, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(ids, "ids", "identifier")?;

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

    #[wasm_bindgen(
        js_name = "getDataContractHistoryWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<bigint, DataContract>>"
    )]
    pub async fn get_data_contract_history_with_proof_info(
        &self,
        query: DataContractHistoryQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
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

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            history_map,
            metadata,
            proof,
        ))
    }

    #[wasm_bindgen(
        js_name = "getDataContractsWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, DataContract | undefined>>"
    )]
    pub async fn get_data_contracts_with_proof_info(
        &self,
        ids: IdentifierLikeArrayJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        // Parse all contract IDs
        let identifiers: Vec<Identifier> =
            try_to_vec::<IdentifierWasm, _, _>(ids, "ids", "identifier")?;

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

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            contracts_map,
            metadata,
            proof,
        ))
    }
}
