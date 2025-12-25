use crate::queries::utils::{convert_optional_limit, deserialize_required_query};
use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::WasmSdkError;
use dash_sdk::dpp::platform_value::Identifier;
use dash_sdk::dpp::voting::votes::resource_vote::ResourceVote;
use dash_sdk::platform::FetchMany;
use drive::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
use drive_proof_verifier::types::ResourceVotesByIdentity;
use js_sys::Map;
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::voting::resource_vote::ResourceVoteWasm;

#[wasm_bindgen(typescript_custom_section)]
const CONTESTED_RESOURCE_IDENTITY_VOTES_QUERY_TS: &'static str = r#"
/**
 * Query parameters for fetching contested resource votes cast by an identity.
 */
export interface ContestedResourceIdentityVotesQuery {
  /**
   * Identity identifier.
   */
  identityId: IdentifierLike

  /**
   * Maximum number of votes to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Vote identifier to resume from (exclusive by default).
   * @default undefined
   */
  startAtVoteId?: IdentifierLike

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
    start_at_vote_id: Option<IdentifierWasm>,
    #[serde(default)]
    start_at_included: Option<bool>,
    #[serde(default)]
    order_ascending: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceIdentityVotesQueryInput {
    identity_id: IdentifierWasm,
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

    let identity_id: Identifier = identity_id.into();

    let limit = convert_optional_limit(limit, "limit")?;

    let start_at = match start_at_vote_id {
        Some(vote_id) => {
            let identifier: Identifier = vote_id.into();

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

/// Convert ResourceVotesByIdentity to a Map<Identifier, ResourceVote>.
fn resource_votes_to_map(votes: ResourceVotesByIdentity) -> Map {
    let map = Map::new();

    for (vote_id, vote_opt) in votes.into_iter() {
        let Some(vote) = vote_opt else {
            continue;
        };

        let key = JsValue::from(IdentifierWasm::from(vote_id));
        let value = JsValue::from(ResourceVoteWasm::from(vote));

        map.set(&key, &value);
    }

    map
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(
        js_name = "getContestedResourceIdentityVotes",
        unchecked_return_type = "Map<Identifier, ResourceVote>"
    )]
    pub async fn get_contested_resource_identity_votes(
        &self,
        query: ContestedResourceIdentityVotesQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let drive_query = parse_contested_resource_identity_votes_query(query)?;

        let votes = ResourceVote::fetch_many(self.as_ref(), drive_query).await?;

        Ok(resource_votes_to_map(votes))
    }

    #[wasm_bindgen(
        js_name = "getContestedResourceIdentityVotesWithProofInfo",
        unchecked_return_type = "ProofMetadataResponseTyped<Map<Identifier, ResourceVote>>"
    )]
    pub async fn get_contested_resource_identity_votes_with_proof_info(
        &self,
        query: ContestedResourceIdentityVotesQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let drive_query = parse_contested_resource_identity_votes_query(query)?;
        let (votes, metadata, proof) =
            ResourceVote::fetch_many_with_metadata_and_proof(self.as_ref(), drive_query, None)
                .await?;

        let votes_map = resource_votes_to_map(votes);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            votes_map, metadata, proof,
        ))
    }
}
