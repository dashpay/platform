use crate::queries::utils::{
    convert_json_values_to_platform_values, convert_optional_limit, deserialize_required_query,
    identifier_from_base58,
};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::platform::FetchMany;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive_proof_verifier::types::Voter;
use js_sys::Array;
use platform_value::string_encoding::Encoding;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(typescript_custom_section)]
const CONTESTED_RESOURCE_VOTERS_QUERY_TS: &'static str = r#"
/**
 * Query parameters for fetching voters of a contested resource.
 */
export interface ContestedResourceVotersForIdentityQuery {
  /**
   * Data contract identifier (base58 string).
   */
  dataContractId: string;

  /**
   * Contested document type name.
   */
  documentTypeName: string;

  /**
   * Index name used to locate the contested resource.
   */
  indexName: string;

  /**
   * Optional index values used as query arguments.
   * @default undefined
   */
  indexValues?: unknown[];

  /**
   * Contested identity identifier (base58 string).
   */
  contestantId: string;

  /**
   * Maximum number of voters to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Voter identifier to resume from (exclusive by default).
   * @default undefined
   */
  startAtVoterId?: string;

  /**
   * Include the `startAtVoterId` when true.
   * @default true
   */
  startAtIncluded?: boolean;

  /**
   * Sort order. When omitted, defaults to ascending.
   * @default true
   */
  orderAscending?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContestedResourceVotersForIdentityQuery")]
    pub type ContestedResourceVotersQueryJs;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceVotersQueryInput {
    data_contract_id: String,
    document_type_name: String,
    index_name: String,
    contestant_id: String,
    #[serde(default)]
    index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    start_at_voter_id: Option<String>,
    #[serde(default)]
    start_at_included: Option<bool>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

fn build_contested_resource_voters_query(
    query: ContestedResourceVotersQueryInput,
) -> Result<ContestedDocumentVotePollVotesDriveQuery, WasmSdkError> {
    let ContestedResourceVotersQueryInput {
        data_contract_id,
        document_type_name,
        index_name,
        contestant_id,
        index_values,
        limit,
        start_at_voter_id,
        start_at_included,
        order_ascending,
    } = query;

    let contract_id = identifier_from_base58(&data_contract_id, "contract ID")?;

    let contestant_id = identifier_from_base58(&contestant_id, "contestant ID")?;

    let index_values = convert_json_values_to_platform_values(index_values, "indexValues")?;

    let start_at = match start_at_voter_id {
        Some(voter_id) => {
            let identifier = identifier_from_base58(&voter_id, "voter ID")?;

            Some((identifier.to_buffer(), start_at_included.unwrap_or(true)))
        }
        None => None,
    };

    let limit = convert_optional_limit(limit, "limit")?;
    let order_ascending = order_ascending.unwrap_or(true);

    Ok(ContestedDocumentVotePollVotesDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        },
        contestant_id,
        offset: None,
        limit,
        start_at,
        order_ascending,
    })
}

fn parse_contested_resource_voters_query(
    query: ContestedResourceVotersQueryJs,
) -> Result<ContestedDocumentVotePollVotesDriveQuery, WasmSdkError> {
    let input: ContestedResourceVotersQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contested resource voters query",
    )?;

    build_contested_resource_voters_query(input)
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResourceVotersForIdentity")]
    pub async fn get_contested_resource_voters_for_identity(
        &self,
        query: ContestedResourceVotersQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let drive_query = parse_contested_resource_voters_query(query)?;

        let voters = Voter::fetch_many(self.as_ref(), drive_query)
            .await
            .map_err(WasmSdkError::from)?;

        let array = Array::new();
        for voter in voters.0.into_iter() {
            let identifier_js = IdentifierWasm::from(voter.0);
            array.push(&JsValue::from(identifier_js));
        }

        Ok(array)
    }

    #[wasm_bindgen(js_name = "getContestedResourceVotersForIdentityWithProofInfo")]
    pub async fn get_contested_resource_voters_for_identity_with_proof_info(
        &self,
        query: ContestedResourceVotersQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let drive_query = parse_contested_resource_voters_query(query)?;

        let (voters, metadata, proof) =
            Voter::fetch_many_with_metadata_and_proof(self.as_ref(), drive_query, None).await?;

        let voters_list: Vec<String> = voters
            .0
            .into_iter()
            .map(|voter| voter.0.to_string(Encoding::Base58))
            .collect();

        let data = serde_wasm_bindgen::to_value(&voters_list).map_err(|e| {
            WasmSdkError::serialization(format!(
                "Failed to serialize contested resource voters: {}",
                e
            ))
        })?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }
}
