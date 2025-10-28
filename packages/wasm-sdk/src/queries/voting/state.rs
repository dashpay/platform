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
use serde::Deserialize;
use serde_json::Value as JsonValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::block::BlockInfoWasm;
use wasm_dpp2::identifier::IdentifierWasm;
use wasm_dpp2::{ContenderWithSerializedDocumentWasm, ContestedDocumentVotePollWinnerInfoWasm};

use crate::queries::utils::{
    convert_json_values_to_platform_values, convert_optional_limit, deserialize_required_query,
    identifier_from_base58,
};
use crate::sdk::WasmSdk;
use crate::{ProofMetadataResponseWasm, WasmSdkError};

#[wasm_bindgen(typescript_custom_section)]
const CONTESTED_RESOURCE_VOTE_STATE_QUERY_TS: &'static str = r#"
/**
 * Query configuration for contested resource vote state.
 */
export interface ContestedResourceVoteStateQuery {
  /**
   * Data contract identifier (base58 string).
   */
  dataContractId: string;

  /**
   * Contested document type name.
   */
  documentTypeName: string;

  /**
   * Index name to query.
   */
  indexName: string;

  /**
   * Optional index values used as query parameters.
   * @default undefined
   */
  indexValues?: unknown[];

  /**
   * Result projection type.
   * @default 'documentsAndVoteTally'
   */
  resultType?: 'documents' | 'voteTally' | 'documentsAndVoteTally';

  /**
   * Maximum number of records to return.
   * @default undefined (no explicit limit)
   */
  limit?: number;

  /**
   * Contender identifier to resume from (exclusive by default).
   * @default undefined
   */
  startAtContenderId?: string;

  /**
   * Include the start contender when true.
   * @default true
   */
  startAtIncluded?: boolean;

  /**
   * Include locked and abstaining tallies when true.
   * @default false
   */
  includeLockedAndAbstaining?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ContestedResourceVoteStateQuery")]
    pub type ContestedResourceVoteStateQueryJs;
}

#[wasm_bindgen(js_name = "ContestedResourceVoteWinner")]
#[derive(Clone)]
pub struct ContestedResourceVoteWinnerWasm {
    info: ContestedDocumentVotePollWinnerInfoWasm,
    block: BlockInfoWasm,
}

impl ContestedResourceVoteWinnerWasm {
    fn from_parts(info: ContestedDocumentVotePollWinnerInfo, block: BlockInfoWasm) -> Self {
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
    pub fn block(&self) -> BlockInfoWasm {
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
    #[wasm_bindgen(getter_with_clone)]
    pub contenders: Array,
    #[wasm_bindgen(getter_with_clone, js_name = "lockVoteTally")]
    pub lock_vote_tally: Option<u32>,
    #[wasm_bindgen(getter_with_clone, js_name = "abstainVoteTally")]
    pub abstain_vote_tally: Option<u32>,
    #[wasm_bindgen(getter_with_clone)]
    pub winner: Option<ContestedResourceVoteWinnerWasm>,
}

impl ContestedResourceVoteStateWasm {
    fn new(
        contenders: Vec<ContestedResourceContenderWasm>,
        lock_vote_tally: Option<u32>,
        abstain_vote_tally: Option<u32>,
        winner: Option<ContestedResourceVoteWinnerWasm>,
    ) -> Self {
        let array = Array::new();
        for contender in contenders {
            array.push(&JsValue::from(contender));
        }

        Self {
            contenders: array,
            lock_vote_tally,
            abstain_vote_tally,
            winner,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceVoteStateQueryInput {
    data_contract_id: String,
    document_type_name: String,
    index_name: String,
    #[serde(default)]
    index_values: Option<Vec<JsonValue>>,
    #[serde(default)]
    result_type: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    start_at_contender_id: Option<String>,
    #[serde(default)]
    start_at_included: Option<bool>,
    #[serde(default)]
    include_locked_and_abstaining: Option<bool>,
}

fn parse_vote_state_result_type(
    result_type: Option<String>,
) -> Result<ContestedDocumentVotePollDriveQueryResultType, WasmSdkError> {
    match result_type {
        None => Ok(ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally),
        Some(value) => match value.as_str() {
            "documents" | "DOCUMENTS" => {
                Ok(ContestedDocumentVotePollDriveQueryResultType::Documents)
            }
            "voteTally" | "VOTE_TALLY" => {
                Ok(ContestedDocumentVotePollDriveQueryResultType::VoteTally)
            }
            "documentsAndVoteTally" | "DOCUMENTS_AND_VOTE_TALLY" => {
                Ok(ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally)
            }
            other => Err(WasmSdkError::invalid_argument(format!(
                "Unsupported result type '{}'",
                other
            ))),
        },
    }
}

fn create_contested_resource_vote_state_query(
    query: ContestedResourceVoteStateQueryInput,
) -> Result<ContestedDocumentVotePollDriveQuery, WasmSdkError> {
    let ContestedResourceVoteStateQueryInput {
        data_contract_id,
        document_type_name,
        index_name,
        index_values,
        result_type,
        limit,
        start_at_contender_id,
        start_at_included,
        include_locked_and_abstaining,
    } = query;

    let index_values = convert_json_values_to_platform_values(index_values, "indexValues")?;

    let contract_id = identifier_from_base58(&data_contract_id, "contract ID")?;

    let result_type = parse_vote_state_result_type(result_type)?;
    let limit = convert_optional_limit(limit, "limit")?;

    let start_at = match start_at_contender_id {
        Some(contender_id) => {
            let identifier = identifier_from_base58(&contender_id, "contender ID")?;

            Some((identifier.to_buffer(), start_at_included.unwrap_or(true)))
        }
        None => None,
    };

    let allow_include_locked_and_abstaining_vote_tally =
        include_locked_and_abstaining.unwrap_or(false);

    Ok(ContestedDocumentVotePollDriveQuery {
        vote_poll: ContestedDocumentResourceVotePoll {
            contract_id,
            document_type_name,
            index_name,
            index_values,
        },
        result_type,
        offset: None,
        limit,
        start_at,
        allow_include_locked_and_abstaining_vote_tally,
    })
}

fn parse_contested_resource_vote_state_query(
    query: ContestedResourceVoteStateQueryJs,
) -> Result<ContestedResourceVoteStateQueryInput, WasmSdkError> {
    deserialize_required_query(
        query,
        "Query object is required",
        "contested resource vote state query",
    )
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
        query: ContestedResourceVoteStateQueryJs,
    ) -> Result<ContestedResourceVoteStateWasm, WasmSdkError> {
        let drive_query = parse_contested_resource_vote_state_query(query)
            .and_then(create_contested_resource_vote_state_query)?;

        let contenders =
            ContenderWithSerializedDocument::fetch_many(self.as_ref(), drive_query).await?;

        Ok(convert_contenders(contenders))
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteStateWithProofInfo")]
    pub async fn get_contested_resource_vote_state_with_proof_info(
        &self,
        query: ContestedResourceVoteStateQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let drive_query = parse_contested_resource_vote_state_query(query)
            .and_then(create_contested_resource_vote_state_query)?;

        let (contenders, metadata, proof) =
            ContenderWithSerializedDocument::fetch_many_with_metadata_and_proof(
                self.as_ref(),
                drive_query,
                None,
            )
            .await?;

        let state = convert_contenders(contenders);

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            state, metadata, proof,
        ))
    }
}
