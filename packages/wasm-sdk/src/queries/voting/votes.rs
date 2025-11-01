use crate::queries::utils::{
    convert_optional_limit, deserialize_required_query, identifier_from_base58,
};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::platform::FetchMany;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive_proof_verifier::types::ResourceVotesByIdentity;
use js_sys::Array;
use platform_value::string_encoding::Encoding;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen(typescript_custom_section)]
const CONTESTED_RESOURCE_IDENTITY_VOTES_QUERY_TS: &'static str = r#"
/**
 * Query parameters for fetching contested resource votes cast by an identity.
 */
export interface ContestedResourceIdentityVotesQuery {
  /**
   * Identity identifier (base58 string).
   */
  identityId: string;

  /**
   * Maximum number of votes to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Vote identifier to resume from (exclusive by default).
   * @default undefined
   */
  startAtVoteId?: string;

  /**
   * Include the `startAtVoteId` when true.
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
    #[wasm_bindgen(typescript_type = "ContestedResourceIdentityVotesQuery")]
    pub type ContestedResourceIdentityVotesQueryJs;
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceIdentityVotesQueryFields {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    start_at_vote_id: Option<String>,
    #[serde(default)]
    start_at_included: Option<bool>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceIdentityVotesQueryInput {
    identity_id: String,
    #[serde(flatten)]
    fields: ContestedResourceIdentityVotesQueryFields,
}

fn build_contested_resource_identity_votes_query(
    input: ContestedResourceIdentityVotesQueryInput,
) -> Result<ContestedResourceVotesGivenByIdentityQuery, WasmSdkError> {
    let ContestedResourceIdentityVotesQueryInput {
        identity_id,
        fields:
            ContestedResourceIdentityVotesQueryFields {
                limit,
                start_at_vote_id,
                start_at_included,
                order_ascending,
            },
    } = input;

    let identity_id = identifier_from_base58(&identity_id, "identity ID")?;

    let limit = convert_optional_limit(limit, "limit")?;

    let start_at = match start_at_vote_id {
        Some(vote_id) => {
            let identifier = identifier_from_base58(&vote_id, "vote ID")?;

            Some((identifier.to_buffer(), start_at_included.unwrap_or(true)))
        }
        None => None,
    };

    Ok(ContestedResourceVotesGivenByIdentityQuery {
        identity_id,
        offset: None,
        limit,
        start_at,
        order_ascending: order_ascending.unwrap_or(true),
    })
}

fn parse_contested_resource_identity_votes_query(
    query: ContestedResourceIdentityVotesQueryJs,
) -> Result<ContestedResourceVotesGivenByIdentityQuery, WasmSdkError> {
    let input: ContestedResourceIdentityVotesQueryInput = deserialize_required_query(
        query,
        "Query object is required",
        "contested resource identity votes query",
    )?;

    build_contested_resource_identity_votes_query(input)
}

fn resource_votes_to_json(
    votes: ResourceVotesByIdentity,
) -> Result<Vec<serde_json::Value>, WasmSdkError> {
    let mut results = Vec::new();

    for (vote_id, vote_opt) in votes.into_iter() {
        let Some(vote) = vote_opt else {
            continue;
        };

        let ResourceVote::V0(ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        }) = vote;

        let VotePoll::ContestedDocumentResourceVotePoll(vote_poll) = vote_poll;

        let poll_unique_id = vote_poll.unique_id().map_err(|e| {
            WasmSdkError::serialization(format!("Failed to derive vote poll unique id: {}", e))
        })?;

        let index_values_json = vote_poll
            .index_values
            .into_iter()
            .map(|value| {
                serde_json::to_value(value).map_err(|e| {
                    WasmSdkError::serialization(format!(
                        "Failed to serialize vote poll index value: {}",
                        e
                    ))
                })
            })
            .collect::<Result<Vec<_>, WasmSdkError>>()?;

        let poll_json = serde_json::json!({
            "type": "contestedDocumentResource",
            "uniqueId": poll_unique_id.to_string(Encoding::Base58),
            "contractId": vote_poll.contract_id.to_string(Encoding::Base58),
            "documentTypeName": vote_poll.document_type_name,
            "indexName": vote_poll.index_name,
            "indexValues": index_values_json,
        });

        let choice_json = match resource_vote_choice {
            ResourceVoteChoice::TowardsIdentity(identifier) => serde_json::json!({
                "type": "towardsIdentity",
                "identityId": identifier.to_string(Encoding::Base58),
            }),
            ResourceVoteChoice::Abstain => serde_json::json!({ "type": "abstain" }),
            ResourceVoteChoice::Lock => serde_json::json!({ "type": "lock" }),
        };

        results.push(serde_json::json!({
            "voteId": vote_id.to_string(Encoding::Base58),
            "votePoll": poll_json,
            "choice": choice_json,
        }));
    }

    Ok(results)
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResourceIdentityVotes")]
    pub async fn get_contested_resource_identity_votes(
        &self,
        query: ContestedResourceIdentityVotesQueryJs,
    ) -> Result<Array, WasmSdkError> {
        let drive_query = parse_contested_resource_identity_votes_query(query)?;

        let votes = ResourceVote::fetch_many(self.as_ref(), drive_query).await?;

        let votes_json = resource_votes_to_json(votes)?;
        let array = Array::new();
        for vote in votes_json {
            let js_value = serde_wasm_bindgen::to_value(&vote).map_err(|e| {
                WasmSdkError::serialization(format!(
                    "Failed to serialize contested resource identity vote: {}",
                    e
                ))
            })?;
            array.push(&js_value);
        }

        Ok(array)
    }

    #[wasm_bindgen(js_name = "getContestedResourceIdentityVotesWithProofInfo")]
    pub async fn get_contested_resource_identity_votes_with_proof_info(
        &self,
        query: ContestedResourceIdentityVotesQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let drive_query = parse_contested_resource_identity_votes_query(query)?;
        let (votes, metadata, proof) =
            ResourceVote::fetch_many_with_metadata_and_proof(self.as_ref(), drive_query, None)
                .await?;

        let votes_json = resource_votes_to_json(votes)?;

        let data = serde_wasm_bindgen::to_value(&serde_json::json!({
            "votes": votes_json
        }))
        .map_err(|e| {
            WasmSdkError::serialization(format!(
                "Failed to serialize contested resource identity votes response: {}",
                e
            ))
        })?;

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            data, metadata, proof,
        ))
    }
}
