use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::utils::{js_value_to_platform_value, js_values_to_platform_values};
use crate::WasmSdkError;
use dash_sdk::dpp::platform_value::{string_encoding::Encoding, Identifier};
use dash_sdk::platform::FetchMany;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::{ContestedResource, ContestedResources};
use js_sys::Array;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(typescript_custom_section)]
const VOTE_POLLS_BY_DOCUMENT_TYPE_QUERY_TS: &'static str = r#"
/**
 * Query configuration for contested resource listings.
 */
export interface VotePollsByDocumentTypeQuery {
  /**
   * Data contract identifier (base58 string).
   */
  dataContractId: string;

  /**
   * Document type to query.
   */
  documentTypeName: string;

  /**
   * Index name to query.
   */
  indexName: string;

  /**
   * Optional lower bound for index range, commonly an array of composite values.
   * @default undefined
   */
  startIndexValues?: unknown[];

  /**
   * Optional upper bound for index range, commonly an array of composite values.
   * @default undefined
   */
  endIndexValues?: unknown[];

  /**
   * Cursor value to resume iteration from.
   * Provide a JS value matching the index schema (e.g., string, number, array).
   * @default undefined
   */
  startAtValue?: unknown;

  /**
   * Whether to include `startAtValue` in the result set.
   * @default true
   */
  startAtValueIncluded?: boolean;

  /**
   * Maximum number of records to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Sort order. When omitted, the query defaults to ascending order.
   * @default true
   */
  orderAscending?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "VotePollsByDocumentTypeQuery")]
    pub type VotePollsByDocumentTypeQueryJs;
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByDocumentTypeFilters {
    #[serde(default)]
    start_index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    end_index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    start_at_value: Option<JsonValue>,
    #[serde(default)]
    start_at_value_included: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByDocumentTypeQueryInput {
    data_contract_id: String,
    document_type_name: String,
    index_name: String,
    #[serde(flatten)]
    filters: VotePollsByDocumentTypeFilters,
}

fn create_vote_polls_by_document_type_query(
    query: VotePollsByDocumentTypeQueryInput,
) -> Result<VotePollsByDocumentTypeQuery, WasmSdkError> {
    let VotePollsByDocumentTypeQueryInput {
        data_contract_id,
        document_type_name,
        index_name,
        filters:
            VotePollsByDocumentTypeFilters {
        start_index_values,
        end_index_values,
        start_at_value,
        start_at_value_included,
        limit,
                order_ascending,
            },
    } = query;

    let start_index_values_js: Vec<JsValue> = start_index_values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            serde_wasm_bindgen::to_value(&value).map_err(|err| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid startIndexValues entry: {}",
                    err
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let start_index_values = js_values_to_platform_values(start_index_values_js)?;

    let end_index_values_js: Vec<JsValue> = end_index_values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            serde_wasm_bindgen::to_value(&value).map_err(|err| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid endIndexValues entry: {}",
                    err
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let end_index_values = js_values_to_platform_values(end_index_values_js)?;

    let start_at_value = match start_at_value {
        Some(value) => {
            let value = serde_wasm_bindgen::to_value(&value).map_err(|err| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid startAtValue entry: {}",
                    err
                ))
            })?;
            let platform_value = js_value_to_platform_value(value)?;
            Some((platform_value, start_at_value_included.unwrap_or(true)))
        }
        None => None,
    };

    let limit = match limit {
        Some(0) => None,
        Some(value) => {
            if value > u16::MAX as u32 {
                return Err(WasmSdkError::invalid_argument(format!(
                    "limit {} exceeds maximum of {}",
                    value,
                    u16::MAX
                )));
            }
            Some(value as u16)
        }
        None => None,
    };

    let contract_id = Identifier::from_string(
        &data_contract_id,
        Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

    Ok(VotePollsByDocumentTypeQuery {
        contract_id,
        document_type_name,
        index_name,
        start_index_values,
        end_index_values,
        start_at_value,
        limit,
        order_ascending: order_ascending.unwrap_or(true),
    })
}

fn contested_resources_into_wasm(
    contested_resources: ContestedResources,
) -> Result<Array, WasmSdkError> {
    let array = Array::new();

    for resource in contested_resources.0 {
        let js_value = serde_wasm_bindgen::to_value(&resource.0).map_err(|e| {
            WasmSdkError::serialization(format!(
                "Failed to serialize contested resource value: {}",
                e
            ))
        })?;
        array.push(&js_value);
    }

    Ok(array)
}

fn parse_vote_polls_by_document_type_query(
    query: JsValue,
) -> Result<VotePollsByDocumentTypeQueryInput, WasmSdkError> {
    if query.is_null() || query.is_undefined() {
        return Err(WasmSdkError::invalid_argument(
            "Query object is required".to_string(),
        ));
    } else {
        serde_wasm_bindgen::from_value(query).map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "Invalid vote polls by document type options: {}",
                err
            ))
        })
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResources")]
    pub async fn get_contested_resources(
        &self,
        query: VotePollsByDocumentTypeQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let query_value: JsValue = query.into();
        let query = parse_vote_polls_by_document_type_query(query_value)
            .and_then(create_vote_polls_by_document_type_query)?;

        let contested_resources = ContestedResource::fetch_many(self.as_ref(), query).await?;

        let array = contested_resources_into_wasm(contested_resources)?;

        Ok(array)
    }

    // Proof info versions for voting queries
    #[wasm_bindgen(js_name = "getContestedResourcesWithProofInfo")]
    pub async fn get_contested_resources_with_proof_info(
        &self,
        query: VotePollsByDocumentTypeQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query_value: JsValue = query.into();
        let query = parse_vote_polls_by_document_type_query(query_value)
            .and_then(create_vote_polls_by_document_type_query)?;

        let (contested_resources, metadata, proof) =
            ContestedResource::fetch_many_with_metadata_and_proof(self.as_ref(), query, None)
                .await?;

        let array = contested_resources_into_wasm(contested_resources)?;

        let response = ProofMetadataResponseWasm::from_sdk_parts(array, metadata, proof);

        Ok(response)
    }
}
