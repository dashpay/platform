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
use wasm_bindgen::JsValue;
use crate::WasmSdkError;

#[wasm_bindgen]
pub async fn data_contract_fetch(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<DataContractWasm, WasmSdkError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!(
        "Invalid data contract ID: {}",
        e
    )))?;

    DataContract::fetch_by_identifier(sdk, id)
        .await?
        .ok_or_else(|| WasmSdkError::not_found("Data contract not found"))
        .map(Into::into)
}

#[wasm_bindgen]
pub async fn data_contract_fetch_with_proof_info(
    sdk: &WasmSdk,
    base58_id: &str,
) -> Result<JsValue, WasmSdkError> {
    let id = Identifier::from_string(
        base58_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!(
        "Invalid data contract ID: {}",
        e
    )))?;

    let (contract, metadata, proof) =
        DataContract::fetch_with_metadata_and_proof(sdk, id, None)
            .await?;

    match contract {
        Some(contract) => {
            let response = ProofMetadataResponse {
                data: contract
                    .to_json(&dash_sdk::dpp::version::PlatformVersion::get(sdk.version()).unwrap())
                    .map_err(|e| WasmSdkError::serialization(format!(
                        "Failed to convert contract to JSON: {}",
                        e
                    )))?,
                metadata: metadata.into(),
                proof: proof.into(),
            };

            // Use json_compatible serializer
            let serializer = serde_wasm_bindgen::Serializer::json_compatible();
            response
                .serialize(&serializer)
                .map_err(|e| WasmSdkError::serialization(format!(
                    "Failed to serialize response: {}",
                    e
                )))
        }
        None => Err(WasmSdkError::not_found("Data contract not found")),
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
) -> Result<JsValue, WasmSdkError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!(
        "Invalid data contract ID: {}",
        e
    )))?;

    // Create query with start timestamp
    let query = LimitQuery {
        query: (contract_id, start_at_ms.unwrap_or(0)),
        start_info: None,
        limit,
    };

    // Fetch contract history
    let history_result = DataContractHistory::fetch(sdk.as_ref(), query)
        .await?;

    // Convert to response format
    let mut versions = BTreeMap::new();
    let platform_version = sdk.as_ref().version();

    if let Some(history) = history_result {
        for (revision, contract) in history {
            versions.insert(
                revision.to_string(),
                contract
                    .to_json(platform_version)
                    .map_err(|e| WasmSdkError::serialization(format!(
                        "Failed to convert contract to JSON: {}",
                        e
                    )))?,
            );
        }
    }

    let response = DataContractHistoryResponse { versions };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| WasmSdkError::serialization(format!(
            "Failed to serialize response: {}",
            e
        )))
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct DataContractsResponse {
    data_contracts: BTreeMap<String, Option<serde_json::Value>>,
}

#[wasm_bindgen]
pub async fn get_data_contracts(sdk: &WasmSdk, ids: Vec<String>) -> Result<JsValue, WasmSdkError> {
    // Parse all contract IDs
    let identifiers: Result<Vec<Identifier>, WasmSdkError> = ids
        .iter()
        .map(|id| {
            Identifier::from_string(
                id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!(
                "Invalid data contract ID: {}",
                e
            )))
        })
        .collect();
    let identifiers = identifiers?;

    // Fetch all contracts
    let contracts_result: DataContracts = DataContract::fetch_many(sdk.as_ref(), identifiers)
        .await?;

    // Convert to response format
    let mut data_contracts = BTreeMap::new();
    let platform_version = sdk.as_ref().version();
    for (id, contract_opt) in contracts_result {
        let id_str = id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
        let contract_json = match contract_opt {
            Some(contract) => Some(
                contract
                    .to_json(platform_version)
                    .map_err(|e| WasmSdkError::serialization(format!(
                        "Failed to convert contract to JSON: {}",
                        e
                    )))?,
            ),
            None => None,
        };
        data_contracts.insert(id_str, contract_json);
    }

    let response = DataContractsResponse { data_contracts };

    // Use json_compatible serializer
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    response
        .serialize(&serializer)
        .map_err(|e| WasmSdkError::serialization(format!(
            "Failed to serialize response: {}",
            e
        )))
}

// Proof info versions for data contract queries

#[wasm_bindgen]
pub async fn get_data_contract_history_with_proof_info(
    sdk: &WasmSdk,
    id: &str,
    limit: Option<u32>,
    _offset: Option<u32>,
    start_at_ms: Option<u64>,
) -> Result<JsValue, WasmSdkError> {
    // Parse contract ID
    let contract_id = Identifier::from_string(
        id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!(
        "Invalid data contract ID: {}",
        e
    )))?;

    // Create query with start timestamp
    let query = LimitQuery {
        query: (contract_id, start_at_ms.unwrap_or(0)),
        start_info: None,
        limit,
    };

    // Fetch contract history with proof
    let (history_result, metadata, proof) =
        DataContractHistory::fetch_with_metadata_and_proof(sdk.as_ref(), query, None)
            .await?;

    // Convert to response format
    let mut versions = BTreeMap::new();
    let platform_version = sdk.as_ref().version();

    if let Some(history) = history_result {
        for (revision, contract) in history {
            versions.insert(
                revision.to_string(),
                contract
                    .to_json(platform_version)
                    .map_err(|e| WasmSdkError::serialization(format!(
                        "Failed to convert contract to JSON: {}",
                        e
                    )))?,
            );
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
        .map_err(|e| WasmSdkError::serialization(format!(
            "Failed to serialize response: {}",
            e
        )))
}

#[wasm_bindgen]
pub async fn get_data_contracts_with_proof_info(
    sdk: &WasmSdk,
    ids: Vec<String>,
) -> Result<JsValue, WasmSdkError> {
    // Parse all contract IDs
    let identifiers: Result<Vec<Identifier>, WasmSdkError> = ids
        .iter()
        .map(|id| {
            Identifier::from_string(
                id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!(
                "Invalid data contract ID: {}",
                e
            )))
        })
        .collect();
    let identifiers = identifiers?;

    // Fetch all contracts with proof
    let (contracts_result, metadata, proof) =
        DataContract::fetch_many_with_metadata_and_proof(sdk.as_ref(), identifiers, None)
            .await?;

    // Convert to response format
    let mut data_contracts = BTreeMap::new();
    let platform_version = sdk.as_ref().version();
    for (id, contract_opt) in contracts_result {
        let id_str = id.to_string(dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58);
        let contract_json = match contract_opt {
            Some(contract) => Some(
                contract
                    .to_json(platform_version)
                    .map_err(|e| WasmSdkError::serialization(format!(
                        "Failed to convert contract to JSON: {}",
                        e
                    )))?,
            ),
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
        .map_err(|e| WasmSdkError::serialization(format!(
            "Failed to serialize response: {}",
            e
        )))
}
