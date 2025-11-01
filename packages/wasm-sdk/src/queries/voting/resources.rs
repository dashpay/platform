use crate::queries::utils::{
    convert_json_values_to_platform_values, convert_optional_limit, deserialize_required_query,
    identifier_from_base58,
};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::utils::js_value_to_platform_value;
use crate::WasmSdkError;
use dash_sdk::platform::FetchMany;
use drive::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
use drive_proof_verifier::types::{ContestedResource, ContestedResources};
use js_sys::{Array, Reflect};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::{prelude::wasm_bindgen, JsValue};

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VotePollsByDocumentTypeQueryInput {
    data_contract_id: String,
    document_type_name: String,
    index_name: String,
    #[serde(default)]
    start_index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    end_index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    start_at_value_included: Option<bool>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

fn create_vote_polls_by_document_type_query(
    query: VotePollsByDocumentTypeQueryInput,
    start_at_value: Option<JsValue>,
) -> Result<VotePollsByDocumentTypeQuery, WasmSdkError> {
    let VotePollsByDocumentTypeQueryInput {
        data_contract_id,
        document_type_name,
        index_name,
        start_index_values,
        end_index_values,
        start_at_value_included,
        limit,
        order_ascending,
    } = query;

    let start_index_values =
        convert_json_values_to_platform_values(start_index_values, "startIndexValues")?;

    let end_index_values =
        convert_json_values_to_platform_values(end_index_values, "endIndexValues")?;

    let start_at_value = match start_at_value {
        Some(value) => {
            let platform_value = js_value_to_platform_value(value).map_err(|err| {
                WasmSdkError::invalid_argument(format!("Invalid startAtValue: {}", err))
            })?;
            Some((platform_value, start_at_value_included.unwrap_or(true)))
        }
        None => None,
    };

    let limit = convert_optional_limit(limit, "limit")?;

    let contract_id = identifier_from_base58(&data_contract_id, "contract ID")?;

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
    query: VotePollsByDocumentTypeQueryJs,
) -> Result<(VotePollsByDocumentTypeQueryInput, Option<JsValue>), WasmSdkError> {
    let value: JsValue = query.into();

    if value.is_null() || value.is_undefined() {
        return Err(WasmSdkError::invalid_argument(
            "Query object is required".to_string(),
        ));
    }

    let start_at_value =
        Reflect::get(&value, &JsValue::from_str("startAtValue")).map_err(|err| {
            let message = err
                .as_string()
                .unwrap_or_else(|| "unable to access property".to_string());

            WasmSdkError::invalid_argument(format!(
                "Invalid vote polls by document type options: {}",
                message
            ))
        })?;

    let start_at_value = if start_at_value.is_null() || start_at_value.is_undefined() {
        None
    } else {
        Some(start_at_value)
    };

    let query_input = deserialize_required_query(
        value,
        "Query object is required",
        "vote polls by document type options",
    )?;

    Ok((query_input, start_at_value))
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResources")]
    pub async fn get_contested_resources(
        &self,
        query: VotePollsByDocumentTypeQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let (query, start_at_value) = parse_vote_polls_by_document_type_query(query)?;
        let drive_query = create_vote_polls_by_document_type_query(query, start_at_value)?;

        let contested_resources = ContestedResource::fetch_many(self.as_ref(), drive_query).await?;

        let array = contested_resources_into_wasm(contested_resources)?;

        Ok(array)
    }

    // Proof info versions for voting queries
    #[wasm_bindgen(js_name = "getContestedResourcesWithProofInfo")]
    pub async fn get_contested_resources_with_proof_info(
        &self,
        query: VotePollsByDocumentTypeQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let (query, start_at_value) = parse_vote_polls_by_document_type_query(query)?;
        let drive_query = create_vote_polls_by_document_type_query(query, start_at_value)?;

        let (contested_resources, metadata, proof) =
            ContestedResource::fetch_many_with_metadata_and_proof(self.as_ref(), drive_query, None)
                .await?;

        let array = contested_resources_into_wasm(contested_resources)?;

        let response = ProofMetadataResponseWasm::from_sdk_parts(array, metadata, proof);

        Ok(response)
    }
}
