use crate::queries::ProofMetadataResponseWasm;
use crate::sdk::WasmSdk;
use crate::utils::js_values_to_platform_values;
use crate::WasmSdkError;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::platform::FetchMany;
use drive::query::vote_poll_contestant_votes_query::ContestedDocumentVotePollVotesDriveQuery;
use drive_proof_verifier::types::Voter;
use js_sys::Array;
use platform_value::string_encoding::Encoding;
use platform_value::Identifier;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::identifier::IdentifierWasm;

#[wasm_bindgen(js_name = "ContestedResourceVotersQuery")]
pub struct ContestedResourceVotersQueryWasm(ContestedDocumentVotePollVotesDriveQuery);

impl ContestedResourceVotersQueryWasm {
    pub(crate) fn into_inner(self) -> ContestedDocumentVotePollVotesDriveQuery {
        self.0
    }
}

#[wasm_bindgen(js_name = "ContestedResourceVotersQueryBuilder")]
pub struct ContestedResourceVotersQueryBuilder {
    vote_poll: ContestedDocumentResourceVotePoll,
    contestant_id: Identifier,
    limit: Option<u16>,
    start_at: Option<([u8; 32], bool)>,
    order_ascending: bool,
}

#[wasm_bindgen(js_class = ContestedResourceVotersQueryBuilder)]
impl ContestedResourceVotersQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        data_contract_id: &str,
        document_type_name: &str,
        index_name: &str,
        contestant_id: &str,
    ) -> Result<ContestedResourceVotersQueryBuilder, WasmSdkError> {
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        let contestant_id = Identifier::from_string(
            contestant_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contestant ID: {}", e)))?;

        Ok(Self {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: Vec::new(),
            },
            contestant_id,
            limit: None,
            start_at: None,
            order_ascending: true,
        })
    }

    #[wasm_bindgen(js_name = "withIndexValues")]
    pub fn with_index_values(
        mut self,
        values: Vec<JsValue>,
    ) -> Result<ContestedResourceVotersQueryBuilder, WasmSdkError> {
        self.vote_poll.index_values = js_values_to_platform_values(values)?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withLimit")]
    pub fn with_limit(
        mut self,
        limit: Option<u32>,
    ) -> Result<ContestedResourceVotersQueryBuilder, WasmSdkError> {
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
        order_ascending: bool,
    ) -> ContestedResourceVotersQueryBuilder {
        self.order_ascending = order_ascending;
        self
    }

    #[wasm_bindgen(js_name = "withStartAtVoter")]
    pub fn with_start_at_voter(
        mut self,
        voter_id: &str,
        included: bool,
    ) -> Result<ContestedResourceVotersQueryBuilder, WasmSdkError> {
        let identifier = Identifier::from_string(
            voter_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid voter ID: {}", e)))?;

        self.start_at = Some((identifier.to_buffer(), included));
        Ok(self)
    }

    #[wasm_bindgen(js_name = "build")]
    pub fn build(self) -> ContestedResourceVotersQueryWasm {
        let ContestedResourceVotersQueryBuilder {
            vote_poll,
            contestant_id,
            limit,
            start_at,
            order_ascending,
        } = self;

        ContestedResourceVotersQueryWasm(ContestedDocumentVotePollVotesDriveQuery {
            vote_poll,
            contestant_id,
            offset: None,
            limit,
            start_at,
            order_ascending,
        })
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResourceVotersForIdentity")]
    pub async fn get_contested_resource_voters_for_identity(
        &self,
        query: ContestedResourceVotersQueryWasm,
    ) -> Result<Array, WasmSdkError> {
        let voters = Voter::fetch_many(self.as_ref(), query.into_inner())
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
        query: ContestedResourceVotersQueryWasm,
    ) -> Result<JsValue, WasmSdkError> {
        let (voters, metadata, proof) =
            Voter::fetch_many_with_metadata_and_proof(self.as_ref(), query.into_inner(), None)
                .await?;

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

        let response = ProofMetadataResponseWasm::from_parts(data, metadata.into(), proof.into());

        Ok(JsValue::from(response))
    }
}
