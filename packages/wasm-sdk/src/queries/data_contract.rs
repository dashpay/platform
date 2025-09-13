use crate::dpp::DataContractWasm;
use crate::queries::ProofMetadataResponse;
use crate::sdk::WasmSdk;
use dash_sdk::dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dash_sdk::platform::query::LimitQuery;
use dash_sdk::platform::{DataContract, Fetch, FetchMany, Identifier};
use drive_proof_verifier::types::{DataContractHistory, DataContracts};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsError, JsValue};
use crate::error::new_structured_error;
use serde_json::json;

#[wasm_bindgen]
pub async fn data_contract_fetch(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<DataContractWasm, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    DataContract::fetch_by_identifier(sdk, id)
        .await
        .map_err(crate::error::map_sdk_error)?
        .ok_or_else(|| new_structured_error("Data contract not found", "E_NOT_FOUND", "not_found", Some(json!({"resource":"data_contract","id": base58_id })), Some(false)))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn data_contract_fetch_with_proof_info(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<JsValue, JsError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    let (contract, metadata, proof) =
        DataContract::fetch_with_metadata_and_proof(sdk, id, None)
            .await
            .map_err(crate::error::map_sdk_error)?;

    match contract {
        Some(contract) => {
            let response = ProofMetadataResponse {
                data: contract.to_json(
                    &dash_sdk::dpp::version::PlatformVersion::get(sdk.version()).unwrap(),
                )?,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response
                .serialize(&serializer)
                .map_err(|e| new_structured_error(&format!("Failed to serialize response: {}", e), "E_INTERNAL", "internal", None, Some(false)))
        }
        None => Err(new_structured_error("Data contract not found", "E_NOT_FOUND", "not_found", Some(json!({"resource":"data_contract","id": base58_id })), Some(false))),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DataContractHistoryResponse {
    versions: BTreeMap<String, serde_json::Value>,
}

#[wasm_bindgen]
pub async fn get_data_contract_history(
    sdk: &WasmSdk,
    id: &str,
    limit: Option<u32>,
    _offset: Option<u32>,
    start_at_ms: Option<u64>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create query with start timestamp
    let query = LimitQuery {
        query: (contract_id, start_at_ms.unwrap_or(0)),
        start_info: None,
        limit,
    };

    // Fetch contract history
    let history_result = DataContractHistory::fetch(sdk.as_ref(), query)
        .await
        .map_err(crate::error::map_sdk_error)?;

    // Convert to response format
    let mut versions = BTreeMap::new();
    let platform_version = sdk.as_ref().version();

    if let Some(history) = history_result {
        for (revision, contract) in history {
            versions.insert(revision.to_string(), contract.to_json(platform_version)?);
        }
    }

    let response = DataContractHistoryResponse { versions };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| new_structured_error(&format!("Failed to serialize response: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DataContractsResponse {
    data_contracts: BTreeMap<String, Option<serde_json::Value>>,
}

#[wasm_bindgen]
pub async fn get_data_contracts(sdk: &WasmSdk, ids: Vec<String>) -> Result<JsValue, JsError> {
    // Parse all contract IDs
    let identifiers: Result<Vec<Identifier>, _> = ids
        .iter()
        .map(|id| {
            Identifier::from_string(
                id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
        })
        .collect();
    let identifiers = identifiers?;

    // Fetch all contracts
    let contracts_result: DataContracts = DataContract::fetch_many(sdk.as_ref(), identifiers)
        .await
        .map_err(|e| JsError::new(&format!("Failed to fetch data contracts: {}", e)))?;

    // Convert to response format
    let mut data_contracts = BTreeMap::new();
    let platform_version = sdk.as_ref().version();
    for (id, contract_opt) in contracts_result {
        let id_str = id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
        let contract_json = match contract_opt {
            Some(contract) => Some(contract.to_json(platform_version)?),
            None => None,
        };
        data_contracts.insert(id_str, contract_json);
    }

    let response = DataContractsResponse { data_contracts };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| new_structured_error(&format!("Failed to serialize response: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}

// Proof info versions for data contract queries

#[wasm_bindgen]
pub async fn get_data_contract_history_with_proof_info(
    sdk: &WasmSdk,
    id: &str,
    limit: Option<u32>,
    _offset: Option<u32>,
    start_at_ms: Option<u64>,
) -> Result<JsValue, JsError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )?;

    // Create query with start timestamp
    let query = LimitQuery {
        query: (contract_id, start_at_ms.unwrap_or(0)),
        start_info: None,
        limit,
    };

    // Fetch contract history with proof
    let (history_result, metadata, proof) =
        DataContractHistory::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await
            .map_err(crate::error::map_sdk_error)?;

    // Convert to response format
    let mut versions = BTreeMap::new();
    let platform_version = sdk.as_ref().version();

    if let Some(history) = history_result {
        for (revision, contract) in history {
            versions.insert(revision.to_string(), contract.to_json(platform_version)?);
        }
    }

    let data = DataContractHistoryResponse { versions };

    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| new_structured_error(&format!("Failed to serialize response: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}

#[wasm_bindgen]
pub async fn get_data_contracts_with_proof_info(
    sdk: &WasmSdk,
    ids: Vec<String>,
) -> Result<JsValue, JsError> {
    // Parse all contract IDs
    let identifiers: Result<Vec<Identifier>, _> = ids
        .iter()
        .map(|id| {
            Identifier::from_string(
                id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
        })
        .collect();
    let identifiers = identifiers?;

    // Fetch all contracts with proof
    let (contracts_result, metadata, proof) =
        DataContract::fetch_many_with_metadata_and_proof(sdk.as_ref(), identifiers, None)
            .await
            .map_err(crate::error::map_sdk_error)?;

    // Convert to response format
    let mut data_contracts = BTreeMap::new();
    let platform_version = sdk.as_ref().version();
    for (id, contract_opt) in contracts_result {
        let id_str = id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
        let contract_json = match contract_opt {
            Some(contract) => Some(contract.to_json(platform_version)?),
            None => None,
        };
        data_contracts.insert(id_str, contract_json);
    }

    let data = DataContractsResponse { data_contracts };

    let response = ProofMetadataResponse {
        data,
        metadata: metadata.into(),
        proof: proof.into(),
    };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| new_structured_error(&format!("Failed to serialize response: {}", e), "E_INTERNAL", "internal", None, Some(false)))
}
