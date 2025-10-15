use dash_sdk::dpp::voting::contender_structs::ContenderWithSerializedDocument;
use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dash_sdk::dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dash_sdk::platform::FetchMany;
use drive::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
};
use drive_proof_verifier::types::Contenders;
use js_sys::Array;
use platform_value::Identifier;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::block::BlockInfoWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::{ContenderWithSerializedDocumentWasm, ContestedDocumentVotePollWinnerInfoWasm};

type ContestedResourceVoteWinnerBlockWasm = BlockInfoWasm;
use crate::sdk::WasmSdk;
use crate::utils::js_values_to_platform_values;
use crate::{ProofMetadataResponseWasm, WasmSdkError};

#[wasm_bindgen(js_name = "ContestedResourceVoteWinner")]
#[derive(Clone)]
pub struct ContestedResourceVoteWinnerWasm {
    info: ContestedDocumentVotePollWinnerInfoWasm,
    block: ContestedResourceVoteWinnerBlockWasm,
}

impl ContestedResourceVoteWinnerWasm {
    fn from_parts(
        info: ContestedDocumentVotePollWinnerInfo,
        block: ContestedResourceVoteWinnerBlockWasm,
    ) -> Self {
        Self {
            info: info.into(),
            block,
        }
    }
}

#[wasm_bindgen(js_class = ContestedResourceVoteWinner)]
impl ContestedResourceVoteWinnerWasm {
    #[wasm_bindgen(getter = kind)]
    pub fn kind(&self) -> String {
        self.info.kind()
    }

    #[wasm_bindgen(getter = identityId)]
    pub fn identity_id(&self) -> Option<IdentifierWasm> {
        self.info.identity_id()
    }

    #[wasm_bindgen(getter = block)]
    pub fn block(&self) -> ContestedResourceVoteWinnerBlockWasm {
        self.block.clone()
    }

    #[wasm_bindgen(getter = info)]
    pub fn info(&self) -> ContestedDocumentVotePollWinnerInfoWasm {
        self.info
    }
}

#[wasm_bindgen(js_name = "ContestedResourceContender")]
#[derive(Clone)]
pub struct ContestedResourceContenderWasm {
    identity_id: Identifier,
    contender: ContenderWithSerializedDocumentWasm,
}

impl ContestedResourceContenderWasm {
    fn from_parts(identity: Identifier, contender: ContenderWithSerializedDocument) -> Self {
        Self {
            identity_id: identity,
            contender: contender.into(),
        }
    }
}

#[wasm_bindgen(js_class = ContestedResourceContender)]
impl ContestedResourceContenderWasm {
    #[wasm_bindgen(getter = identityId)]
    pub fn identity_id(&self) -> IdentifierWasm {
        IdentifierWasm::from(self.identity_id.clone())
    }

    #[wasm_bindgen(getter = serializedDocument)]
    pub fn serialized_document(&self) -> JsValue {
        self.contender.serialized_document()
    }

    #[wasm_bindgen(getter = voteTally)]
    pub fn vote_tally(&self) -> Option<u32> {
        self.contender.vote_tally()
    }

    #[wasm_bindgen(getter = contender)]
    pub fn contender(&self) -> ContenderWithSerializedDocumentWasm {
        self.contender.clone()
    }
}

#[wasm_bindgen(js_name = "ContestedResourceVoteState")]
#[derive(Clone)]
pub struct ContestedResourceVoteStateWasm {
    contenders: Vec<ContestedResourceContenderWasm>,
    lock_vote_tally: Option<u32>,
    abstain_vote_tally: Option<u32>,
    winner: Option<ContestedResourceVoteWinnerWasm>,
}

impl ContestedResourceVoteStateWasm {
    fn new(
        contenders: Vec<ContestedResourceContenderWasm>,
        lock_vote_tally: Option<u32>,
        abstain_vote_tally: Option<u32>,
        winner: Option<ContestedResourceVoteWinnerWasm>,
    ) -> Self {
        Self {
            contenders,
            lock_vote_tally,
            abstain_vote_tally,
            winner,
        }
    }
}

#[wasm_bindgen(js_class = ContestedResourceVoteState)]
impl ContestedResourceVoteStateWasm {
    #[wasm_bindgen(getter = contenders)]
    pub fn contenders(&self) -> Array {
        let array = Array::new();
        for contender in &self.contenders {
            array.push(&JsValue::from(contender.clone()));
        }
        array
    }

    #[wasm_bindgen(getter = lockVoteTally)]
    pub fn lock_vote_tally(&self) -> Option<u32> {
        self.lock_vote_tally
    }

    #[wasm_bindgen(getter = abstainVoteTally)]
    pub fn abstain_vote_tally(&self) -> Option<u32> {
        self.abstain_vote_tally
    }

    #[wasm_bindgen(getter = winner)]
    pub fn winner(&self) -> Option<ContestedResourceVoteWinnerWasm> {
        self.winner.clone()
    }
}

#[wasm_bindgen(js_name = "ContestedResourceVoteStateQuery")]
pub struct ContestedResourceVoteStateQueryWasm(ContestedDocumentVotePollDriveQuery);

impl ContestedResourceVoteStateQueryWasm {
    pub(crate) fn into_inner(self) -> ContestedDocumentVotePollDriveQuery {
        self.0
    }
}

#[wasm_bindgen(js_name = "ContestedResourceVoteStateQueryBuilder")]
pub struct ContestedResourceVoteStateQueryBuilder {
    vote_poll: ContestedDocumentResourceVotePoll,
    result_type: ContestedDocumentVotePollDriveQueryResultType,
    limit: Option<u16>,
    start_at: Option<([u8; 32], bool)>,
    allow_include_locked_and_abstaining_vote_tally: bool,
}

#[wasm_bindgen(js_class = ContestedResourceVoteStateQueryBuilder)]
impl ContestedResourceVoteStateQueryBuilder {
    #[wasm_bindgen(constructor)]
    pub fn new(
        data_contract_id: &str,
        document_type_name: &str,
        index_name: &str,
    ) -> Result<ContestedResourceVoteStateQueryBuilder, WasmSdkError> {
        let contract_id = Identifier::from_string(
            data_contract_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

        Ok(Self {
            vote_poll: ContestedDocumentResourceVotePoll {
                contract_id,
                document_type_name: document_type_name.to_string(),
                index_name: index_name.to_string(),
                index_values: Vec::new(),
            },
            result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
            limit: None,
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: false,
        })
    }

    #[wasm_bindgen(js_name = "withIndexValues")]
    pub fn with_index_values(
        mut self,
        values: Vec<JsValue>,
    ) -> Result<ContestedResourceVoteStateQueryBuilder, WasmSdkError> {
        self.vote_poll.index_values = js_values_to_platform_values(values)?;
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withResultType")]
    pub fn with_result_type(
        mut self,
        result_type: &str,
    ) -> Result<ContestedResourceVoteStateQueryBuilder, WasmSdkError> {
        self.result_type = match result_type {
            "documents" | "DOCUMENTS" => ContestedDocumentVotePollDriveQueryResultType::Documents,
            "voteTally" | "VOTE_TALLY" => ContestedDocumentVotePollDriveQueryResultType::VoteTally,
            "documentsAndVoteTally" | "DOCUMENTS_AND_VOTE_TALLY" => {
                ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally
            }
            other => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Unsupported result type '{}'",
                    other
                )))
            }
        };
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withLimit")]
    pub fn with_limit(
        mut self,
        limit: Option<u32>,
    ) -> Result<ContestedResourceVoteStateQueryBuilder, WasmSdkError> {
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

    #[wasm_bindgen(js_name = "withStartAtContender")]
    pub fn with_start_at_contender(
        mut self,
        contender_id: &str,
        included: bool,
    ) -> Result<ContestedResourceVoteStateQueryBuilder, WasmSdkError> {
        let identifier = Identifier::from_string(
            contender_id,
            dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
        )
        .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contender ID: {}", e)))?;

        self.start_at = Some((identifier.to_buffer(), included));
        Ok(self)
    }

    #[wasm_bindgen(js_name = "withIncludeLockedAndAbstaining")]
    pub fn with_include_locked_and_abstaining(
        mut self,
        include: bool,
    ) -> ContestedResourceVoteStateQueryBuilder {
        self.allow_include_locked_and_abstaining_vote_tally = include;
        self
    }

    #[wasm_bindgen(js_name = "build")]
    pub fn build(self) -> ContestedResourceVoteStateQueryWasm {
        let ContestedResourceVoteStateQueryBuilder {
            vote_poll,
            result_type,
            limit,
            start_at,
            allow_include_locked_and_abstaining_vote_tally,
        } = self;

        ContestedResourceVoteStateQueryWasm(ContestedDocumentVotePollDriveQuery {
            vote_poll,
            result_type,
            offset: None,
            limit,
            start_at,
            allow_include_locked_and_abstaining_vote_tally,
        })
    }
}

fn convert_contenders(contenders: Contenders) -> ContestedResourceVoteStateWasm {
    let Contenders {
        winner,
        contenders: inner_contenders,
        abstain_vote_tally,
        lock_vote_tally,
    } = contenders;

    let wrappers = inner_contenders
        .into_iter()
        .map(|(identity, contender)| {
            ContestedResourceContenderWasm::from_parts(identity, contender)
        })
        .collect();

    let winner = winner.map(|(info, block)| {
        ContestedResourceVoteWinnerWasm::from_parts(info, BlockInfoWasm::from(block))
    });

    ContestedResourceVoteStateWasm::new(wrappers, lock_vote_tally, abstain_vote_tally, winner)
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "getContestedResourceVoteState")]
    pub async fn get_contested_resource_vote_state(
        &self,
        query: ContestedResourceVoteStateQueryWasm,
    ) -> Result<ContestedResourceVoteStateWasm, WasmSdkError> {
        let contenders =
            ContenderWithSerializedDocument::fetch_many(self.as_ref(), query.into_inner()).await?;

        let state = convert_contenders(contenders);

        Ok(state)
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteStateWithProofInfo")]
    pub async fn get_contested_resource_vote_state_with_proof_info(
        &self,
        query: ContestedResourceVoteStateQueryWasm,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let (contenders, metadata, proof) =
            ContenderWithSerializedDocument::fetch_many_with_metadata_and_proof(
                self.as_ref(),
                query.into_inner(),
                None,
            )
            .await?;

        let state = convert_contenders(contenders);

        Ok(ProofMetadataResponseWasm::from_parts(
            JsValue::from(state),
            metadata.into(),
            proof.into(),
        ))
    }
}
