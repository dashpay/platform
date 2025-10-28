use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::platform::FetchMany;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive_proof_verifier::types::ResourceVotesByIdentity;
use js_sys::Array;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

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

fn convert_limit(limit: Option<u32>) -> Result<Option<u16>, WasmSdkError> {
    match limit {
        Some(0) => Ok(None),
        Some(value) => {
            if value > u16::MAX as u32 {
                return Err(WasmSdkError::invalid_argument(format!(
                    "limit {} exceeds maximum of {}",
                    value,
                    u16::MAX
                )));
            }
            Ok(Some(value as u16))
        }
        None => Ok(None),
    }
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

    let identity_id = Identifier::from_string(
        &identity_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

    let limit = convert_limit(limit)?;

    let start_at = match start_at_vote_id {
        Some(vote_id) => {
            let identifier = Identifier::from_string(
                &vote_id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid vote ID: {}", e)))?;

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
    let value: JsValue = query.into();
    let input: ContestedResourceIdentityVotesQueryInput =
        serde_wasm_bindgen::from_value(value).map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "Invalid contested resource identity votes query: {}",
                err
            ))
        })?;

    build_contested_resource_identity_votes_query(input)
}

fn resource_votes_to_json(
    votes: ResourceVotesByIdentity,
) -> Result<Vec<serde_json::Value>, WasmSdkError> {
    let mut results = Vec::new();

    for (vote_id, vote_opt) in votes.into_iter() {
        let vote = match vote_opt {
            Some(vote) => vote,
            None => continue,
        };

        let inner = match vote {
            ResourceVote::V0(inner) => inner,
        };

        let vote_poll = match inner.vote_poll {
            VotePoll::ContestedDocumentResourceVotePoll(poll) => poll,
        };

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

        let choice_json = match inner.resource_vote_choice {
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
    ) -> Result<JsValue, WasmSdkError> {
        let drive_query = parse_contested_resource_identity_votes_query(query)?;
        let (votes, metadata, proof) = ResourceVote::fetch_many_with_metadata_and_proof(
            self.as_ref(),
            drive_query,
            None,
        )
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

        let response = ProofMetadataResponseWasm::from_parts(data, metadata.into(), proof.into());

        Ok(JsValue::from(response))
    }
}
