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

use crate::sdk::WasmSdk;
use crate::utils::js_values_to_platform_values;
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
    fn from_parts(
        info: ContestedDocumentVotePollWinnerInfo,
        block: BlockInfoWasm,
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

#[wasm_bindgen(js_name = "ContestedResourceVoteStateQuery")]
pub struct ContestedResourceVoteStateQueryWasm(ContestedDocumentVotePollDriveQuery);

impl ContestedResourceVoteStateQueryWasm {
    pub(crate) fn into_inner(self) -> ContestedDocumentVotePollDriveQuery {
        self.0
    }

    pub(crate) fn from_query(query: ContestedDocumentVotePollDriveQuery) -> Self {
        Self(query)
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceVoteStateQueryFields {
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContestedResourceVoteStateQueryInput {
    data_contract_id: String,
    document_type_name: String,
    index_name: String,
    #[serde(flatten)]
    fields: ContestedResourceVoteStateQueryFields,
}

fn parse_vote_state_result_type(
    result_type: Option<String>,
) -> Result<ContestedDocumentVotePollDriveQueryResultType, WasmSdkError> {
    match result_type {
        None => Ok(ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally),
        Some(value) => match value.as_str() {
            "documents" | "DOCUMENTS" => Ok(ContestedDocumentVotePollDriveQueryResultType::Documents),
            "voteTally" | "VOTE_TALLY" => Ok(ContestedDocumentVotePollDriveQueryResultType::VoteTally),
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

fn create_contested_resource_vote_state_query(
    query: ContestedResourceVoteStateQueryInput,
) -> Result<ContestedDocumentVotePollDriveQuery, WasmSdkError> {
    let ContestedResourceVoteStateQueryInput {
        data_contract_id,
        document_type_name,
        index_name,
        fields:
            ContestedResourceVoteStateQueryFields {
                index_values,
                result_type,
                limit,
                start_at_contender_id,
                start_at_included,
                include_locked_and_abstaining,
            },
    } = query;

    let index_values: Vec<JsValue> = index_values
        .unwrap_or_default()
        .into_iter()
        .map(|value| {
            serde_wasm_bindgen::to_value(&value).map_err(|err| {
                WasmSdkError::invalid_argument(format!(
                    "Invalid indexValues entry: {}",
                    err
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let index_values = js_values_to_platform_values(index_values)?;

    let contract_id = Identifier::from_string(
        &data_contract_id,
        dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
    )
    .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contract ID: {}", e)))?;

    let result_type = parse_vote_state_result_type(result_type)?;
    let limit = convert_limit(limit)?;

    let start_at = match start_at_contender_id {
        Some(contender_id) => {
            let identifier = Identifier::from_string(
                &contender_id,
                dash_sdk::dpp::platform_value::string_encoding::Encoding::Base58,
            )
            .map_err(|e| WasmSdkError::invalid_argument(format!("Invalid contender ID: {}", e)))?;

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
    query: JsValue,
) -> Result<ContestedResourceVoteStateQueryInput, WasmSdkError> {
    if query.is_null() || query.is_undefined() {
        return Err(WasmSdkError::invalid_argument(
            "Query object is required".to_string(),
        ));
    } else {
        serde_wasm_bindgen::from_value(query).map_err(|err| {
            WasmSdkError::invalid_argument(format!(
                "Invalid contested resource vote state query: {}",
                err
            ))
        })
    }
}

#[wasm_bindgen(js_name = "buildContestedResourceVoteStateQuery")]
pub fn build_contested_resource_vote_state_query(
    query: ContestedResourceVoteStateQueryJs,
) -> Result<ContestedResourceVoteStateQueryWasm, WasmSdkError> {
    let query_value: JsValue = query.into();
    let query = parse_contested_resource_vote_state_query(query_value)
        .and_then(create_contested_resource_vote_state_query)?;

    Ok(ContestedResourceVoteStateQueryWasm::from_query(query))
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
    async fn fetch_contested_resource_vote_state(
        &self,
        query: ContestedDocumentVotePollDriveQuery,
    ) -> Result<ContestedResourceVoteStateWasm, WasmSdkError> {
        let contenders =
            ContenderWithSerializedDocument::fetch_many(self.as_ref(), query).await?;

        Ok(convert_contenders(contenders))
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteState")]
    pub async fn get_contested_resource_vote_state(
        &self,
        query: ContestedResourceVoteStateQueryJs,
    ) -> Result<ContestedResourceVoteStateWasm, WasmSdkError> {
        let query_value: JsValue = query.into();
        let query = parse_contested_resource_vote_state_query(query_value)
            .and_then(create_contested_resource_vote_state_query)?;

        self.fetch_contested_resource_vote_state(query).await
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteStateWithQuery")]
    pub async fn get_contested_resource_vote_state_with_query(
        &self,
        query: ContestedResourceVoteStateQueryWasm,
    ) -> Result<ContestedResourceVoteStateWasm, WasmSdkError> {
        self.fetch_contested_resource_vote_state(query.into_inner()).await
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteStateWithProofInfo")]
    pub async fn get_contested_resource_vote_state_with_proof_info(
        &self,
        query: ContestedResourceVoteStateQueryJs,
    ) -> Result<ProofMetadataResponseWasm, WasmSdkError> {
        let query_value: JsValue = query.into();
        let query = parse_contested_resource_vote_state_query(query_value)
            .and_then(create_contested_resource_vote_state_query)?;

        self.get_contested_resource_vote_state_with_proof_info_query(ContestedResourceVoteStateQueryWasm::from_query(query))
            .await
    }

    #[wasm_bindgen(js_name = "getContestedResourceVoteStateWithProofInfoQuery")]
    pub async fn get_contested_resource_vote_state_with_proof_info_query(
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

        Ok(ProofMetadataResponseWasm::from_sdk_parts(
            JsValue::from(state),
            metadata,
            proof,
        ))
    }
}
