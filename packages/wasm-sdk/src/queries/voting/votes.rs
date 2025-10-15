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
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

#[wasm_bindgen(js_name = "ContestedResourceIdentityVotesQuery")]
pub struct ContestedResourceIdentityVotesQueryWasm(ContestedResourceVotesGivenByIdentityQuery);

impl ContestedResourceIdentityVotesQueryWasm {
    pub(crate) fn into_inner(self) -> ContestedResourceVotesGivenByIdentityQuery {
        self.0
    }
}

#[wasm_bindgen(js_name = "ContestedResourceIdentityVotesQueryBuilder")]
pub struct ContestedResourceIdentityVotesQueryBuilder {
    identity_id: Identifier,
    limit: Option<u16>,
    start_at_vote: Option<([u8; 32], bool)>,
    order_ascending: bool,
}

#[wasm_bindgen(js_class = ContestedResourceIdentityVotesQueryBuilder)]
impl ContestedResourceIdentityVotesQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        identity_id: &str,
    ) -> Result<ContestedResourceIdentityVotesQueryBuilder, WasmSdkError> {
        let identity_id = Identifier::from_string(
            identity_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid identity ID: {}", e)))?;

        Ok(Self {
            identity_id,
            limit: None,
            start_at_vote: None,
            order_ascending: true,
        })
    }

    #[wasm_bindgen(js_name = "withLimit")]
    pub fn with_limit(
        mut self,
        limit: Option<u32>,
    ) -> Result<ContestedResourceIdentityVotesQueryBuilder, WasmSdkError> {
        self.limit = match limit {
            Some(0) => None,
            Some(count) => {
                if count > u16::MAX as u32 {
                    return Err(WasmSdkError::invalid_argument(format!(
                        "limit {} exceeds maximum of {}",
                        count,
                        u16::MAX
                    )));
                }
                Some(count as u16)
            }
            None => None,
        };
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withOrderAscending")]
    pub fn with_order_ascending(
        mut self,
        ascending: bool,
    ) -> ContestedResourceIdentityVotesQueryBuilder {
        self.order_ascending = ascending;
        self
    }

    #[wasm_bindgen(js_name = "withStartAtVote")]
    pub fn with_start_at_vote(
        mut self,
        vote_id: &str,
        included: bool,
    ) -> Result<ContestedResourceIdentityVotesQueryBuilder, WasmSdkError> {
        let identifier = Identifier::from_string(
            vote_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid vote ID: {}", e)))?;

        self.start_at_vote = Some((identifier.to_buffer(), included));
        Ok(self)
    }

    #[wasm_bindgen(js_name = "build")]
    pub fn build(self) -> ContestedResourceIdentityVotesQueryWasm {
        let ContestedResourceIdentityVotesQueryBuilder {
            identity_id,
            limit,
            start_at_vote,
            order_ascending,
        } = self;

        ContestedResourceIdentityVotesQueryWasm(ContestedResourceVotesGivenByIdentityQuery {
            identity_id,
            offset: None,
            limit,
            start_at: start_at_vote,
            order_ascending,
        })
    }
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
        query: ContestedResourceIdentityVotesQueryWasm,
    ) -> Result<Array, WasmSdkError> {
        let votes = ResourceVote::fetch_many(self.as_ref(), query.into_inner()).await?;

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
        query: ContestedResourceIdentityVotesQueryWasm,
    ) -> Result<JsValue, WasmSdkError> {
        let (votes, metadata, proof) = ResourceVote::fetch_many_with_metadata_and_proof(
            self.as_ref(),
            query.into_inner(),
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
