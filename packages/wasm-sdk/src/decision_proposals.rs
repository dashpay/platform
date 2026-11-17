//! Decision proposals (protocol version 15): yes/no questions any identity puts to the
//! masternodes. Thin bindings over `dash_sdk::platform::decision_proposals`: a proposal is a
//! `proposal` document of the decision proposals system contract, created and edited like any
//! document, and its poll is named by the proposal's id.

use crate::error::WasmSdkError;
use crate::queries::utils::{documents_map, documents_page_of};
use crate::queries::voting::yes_no::{state_to_js, vote_to_js};
use crate::sdk::WasmSdk;
use crate::state_transitions::identity::{yes_no_vote_choice_from_options, MasternodeVoteSigning};
use dash_sdk::dpp::decision_proposal::{
    DECISION_PROPOSALS_CONTRACT_ID, PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePollPurpose;
use dash_sdk::platform::decision_proposals::{
    build_decision_proposal_edit, DecisionProposalEdit, DecisionProposalEndDates,
    DecisionProposalInput,
};
use dash_sdk::platform::documents::DocumentsPage;
use dash_sdk::platform::yes_no_vote_polls::{
    MasternodeYesNoVote, MasternodeYesNoVoteQuery, YesNoVotePollState, YesNoVotePollStateQuery,
};
use dash_sdk::platform::{Fetch, Identifier};
use drive_proof_verifier::types::Documents;
use js_sys::{Map, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::utils::{
    try_from_options_optional_with, try_from_options_with, try_to_string, try_to_u64, try_to_u8,
};
use wasm_dpp2::voting::vote::VoteWasm;

#[wasm_bindgen(typescript_custom_section)]
const DECISION_PROPOSALS_TS: &'static str = r#"
/**
 * Options for `buildDecisionProposal`: a yes/no question put to the masternodes (protocol
 * version 15). Creating the proposal opens its vote poll; the owner pays the decision
 * proposal fund, 0.5 Dash, on top of the fees.
 */
export interface DecisionProposalOptions {
  /** The proposer, the document's owner. */
  ownerId: IdentifierLike;
  /** The question, in one line: 1 to 128 characters. */
  title: string;
  /** What a yes and a no mean: 1 to 4096 characters. */
  description: string;
  /** Where the proposal is discussed, a URI of at most 512 characters. */
  url?: string;
  /**
   * The proposal passes when the yes voting power is strictly more than this percent of the
   * yes plus no voting power: 50 to 80.
   */
  passPercent: number;
  /**
   * The least yes plus no voting power the proposal needs, as a percent of the masternode
   * list's total voting power when it closes, rounded up: 0 to 50.
   */
  minimumTurnoutPercent: number;
  /**
   * When voting closes, in milliseconds since the Unix epoch: from a week to four weeks after
   * the block that creates the proposal, which is a little later than now.
   */
  endDate: bigint | number;
}

/**
 * New text for a proposal (`buildDecisionProposalEdit`). A field left out keeps its value.
 * Platform accepts the edit for a day after the proposal is created; the rule and the end
 * date never change.
 */
export interface DecisionProposalEdit {
  /** The new title. */
  title?: string;
  /** The new description. */
  description?: string;
  /** The new url; `null` removes it. */
  url?: string | null;
}

/** One page of the proposals by end date (`getDecisionProposalsByEndDate`). */
export interface DecisionProposalsByEndDateQuery {
  /**
   * The earliest end date listed, included. The current time lists the open proposals.
   * Omit it for no lower bound.
   */
  endDateFrom?: bigint | number;
  /** The end date the listing stops before. Omit it for no upper bound. */
  endDateTo?: bigint | number;
  /** Continue after this proposal; omit for the first page. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of proposals to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** One page of an identity's proposals (`getDecisionProposalsByOwner`). */
export interface DecisionProposalsByOwnerQuery {
  /** The proposer. */
  ownerId: IdentifierLike;
  /** Continue after this proposal; omit for the first page. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of proposals to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** One page of every proposal, newest first (`getDecisionProposalsByCreatedAt`). */
export interface DecisionProposalsPageQuery {
  /** Continue after this proposal; omit for the first page. */
  startAfter?: IdentifierLike;
  /**
   * Maximum number of proposals to return, 1 to 100.
   * @default 100
   */
  limit?: number;
}

/** Options for `decisionProposalVote`: a masternode's answer on a proposal. */
export interface DecisionProposalVoteOptions {
  /**
   * The ProTxHash of the masternode: an Identifier, its 32 bytes, the hex Core's RPC shows,
   * or base58.
   */
  masternodeProTxHash: IdentifierLike;
  /** The proposal. */
  proposalId: IdentifierLike;
  /**
   * The answer. A masternode may vote 5 times on a proposal, but not give the answer it
   * already gave.
   */
  voteChoice: YesNoAbstainVoteChoiceName;
  /** The masternode's voting public key. */
  votingKey: IdentityPublicKey;
  /** Signer holding the private key of the voting key. */
  signer: IdentitySigner;
  /** Optional settings for the broadcast. */
  settings?: PutSettings;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "DecisionProposalOptions")]
    pub type DecisionProposalOptionsJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalEdit")]
    pub type DecisionProposalEditJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalsByEndDateQuery")]
    pub type DecisionProposalsByEndDateQueryJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalsByOwnerQuery")]
    pub type DecisionProposalsByOwnerQueryJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalsPageQuery")]
    pub type DecisionProposalsPageQueryJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalVoteOptions")]
    pub type DecisionProposalVoteOptionsJs;
}

/// A required string option.
fn string_option(options: &JsValue, property: &str) -> Result<String, WasmSdkError> {
    Ok(try_from_options_with(options, property, |value| {
        try_to_string(value, property)
    })?)
}

/// An optional string option.
fn optional_string_option(
    options: &JsValue,
    property: &str,
) -> Result<Option<String>, WasmSdkError> {
    Ok(try_from_options_optional_with(
        options,
        property,
        |value| try_to_string(value, property),
    )?)
}

/// An optional end date option.
fn optional_end_date_option(
    options: &JsValue,
    property: &str,
) -> Result<Option<u64>, WasmSdkError> {
    Ok(try_from_options_optional_with(
        options,
        property,
        |value| try_to_u64(value, property),
    )?)
}

/// The page a listing query asks for (see `documents_page_of`). An absent query is the first
/// page of 100.
fn page_of(query: &JsValue) -> Result<DocumentsPage, WasmSdkError> {
    if query.is_undefined() || query.is_null() {
        return Ok(DocumentsPage::default());
    }
    documents_page_of(query)
}

fn proposal_wasm(document: Document, entropy: Option<[u8; 32]>) -> DocumentWasm {
    DocumentWasm::new(
        document,
        DECISION_PROPOSALS_CONTRACT_ID,
        PROPOSAL_DOCUMENT_TYPE_NAME.to_string(),
        entropy,
    )
}

fn proposals_map(documents: Documents) -> Map {
    documents_map(
        documents,
        DECISION_PROPOSALS_CONTRACT_ID,
        PROPOSAL_DOCUMENT_TYPE_NAME,
    )
}

fn proposal_purpose(proposal_id: IdentifierLikeJs) -> Result<YesNoVotePollPurpose, WasmSdkError> {
    let document_id: Identifier = proposal_id.try_into()?;
    Ok(YesNoVotePollPurpose::DecisionProposal { document_id })
}

#[wasm_bindgen]
impl WasmSdk {
    /// Builds a proposal: a `proposal` document of the decision proposals contract with a
    /// fresh entropy, to create with `documentCreate`. The create states the decision proposal
    /// fund itself. Platform refuses a rule outside the contract's limits and an end date less
    /// than a week or more than four weeks after the block that creates the proposal. The id
    /// is final once `documentCreate` returns the stored document.
    ///
    /// @param options - The proposer, the question and its rule.
    /// @returns The proposal, ready for `documentCreate`.
    #[wasm_bindgen(js_name = "buildDecisionProposal")]
    pub async fn build_decision_proposal(
        &self,
        options: DecisionProposalOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let owner_id: Identifier = IdentifierWasm::try_from_options(&options, "ownerId")?.into();
        let input = DecisionProposalInput {
            title: string_option(&options, "title")?,
            description: string_option(&options, "description")?,
            url: optional_string_option(&options, "url")?,
            pass_percent: try_from_options_with(&options, "passPercent", |value| {
                try_to_u8(value, "passPercent")
            })?,
            minimum_turnout_percent: try_from_options_with(
                &options,
                "minimumTurnoutPercent",
                |value| try_to_u8(value, "minimumTurnoutPercent"),
            )?,
            end_date: try_from_options_with(&options, "endDate", |value| {
                try_to_u64(value, "endDate")
            })?,
        };
        let built = self
            .as_ref()
            .build_decision_proposal(owner_id, &input)
            .await?;
        Ok(proposal_wasm(built.document, Some(built.entropy.0)))
    }

    /// A proposal with new text and its next revision, to replace with `documentReplace`.
    /// Platform accepts the edit for a day after the proposal is created; the rule and the
    /// end date never change. A document that is not a proposal is refused.
    ///
    /// @param proposal - The proposal as stored, from `getDecisionProposal`.
    /// @param edit - The new title, description or url; `url: null` removes the url.
    /// @returns The edited proposal.
    #[wasm_bindgen(js_name = "buildDecisionProposalEdit")]
    pub fn build_decision_proposal_edit(
        &self,
        proposal: &DocumentWasm,
        edit: DecisionProposalEditJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let edit: JsValue = edit.into();
        let url = Reflect::get(&edit, &JsValue::from_str("url"))
            .map_err(|_| WasmSdkError::invalid_argument("cannot read url"))?;
        let url = if url.is_undefined() {
            None
        } else if url.is_null() {
            Some(None)
        } else {
            Some(Some(try_to_string(&url, "url")?))
        };
        let edit = DecisionProposalEdit {
            title: optional_string_option(&edit, "title")?,
            description: optional_string_option(&edit, "description")?,
            url,
        };
        let data_contract_id: Identifier = proposal.data_contract_id().into();
        let document: Document = proposal.clone().into();
        Ok(proposal_wasm(
            build_decision_proposal_edit(
                &data_contract_id,
                &proposal.document_type_name(),
                &document,
                &edit,
            )?,
            None,
        ))
    }

    /// A proposal by id.
    ///
    /// @param proposalId - The proposal.
    /// @returns The `proposal` document, or undefined when there is none.
    #[wasm_bindgen(js_name = "getDecisionProposal")]
    pub async fn get_decision_proposal(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_decision_proposal(proposal_id)
            .await?
            .map(|document| proposal_wasm(document, None)))
    }

    /// One page of the proposals whose end date is within the query's bounds, by end date,
    /// then by creation time. `endDateFrom: Date.now()` lists the open proposals.
    ///
    /// @returns The proposals by id; pass the last id as `startAfter` for the next page.
    #[wasm_bindgen(
        js_name = "getDecisionProposalsByEndDate",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_decision_proposals_by_end_date(
        &self,
        query: Option<DecisionProposalsByEndDateQueryJs>,
    ) -> Result<Map, WasmSdkError> {
        let query: JsValue = query.map(Into::into).unwrap_or(JsValue::UNDEFINED);
        let end_dates = if query.is_undefined() || query.is_null() {
            DecisionProposalEndDates::default()
        } else {
            DecisionProposalEndDates {
                from: optional_end_date_option(&query, "endDateFrom")?,
                to: optional_end_date_option(&query, "endDateTo")?,
            }
        };
        let documents = self
            .as_ref()
            .fetch_decision_proposals_by_end_date(end_dates, page_of(&query)?)
            .await?;
        Ok(proposals_map(documents))
    }

    /// One page of an identity's proposals, by creation time.
    ///
    /// @returns The proposals by id; pass the last id as `startAfter` for the next page.
    #[wasm_bindgen(
        js_name = "getDecisionProposalsByOwner",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_decision_proposals_by_owner(
        &self,
        query: DecisionProposalsByOwnerQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let query: JsValue = query.into();
        if query.is_undefined() || query.is_null() {
            return Err(WasmSdkError::invalid_argument("Query object is required"));
        }
        let owner_id: Identifier = IdentifierWasm::try_from_options(&query, "ownerId")?.into();
        let documents = self
            .as_ref()
            .fetch_decision_proposals_by_owner(owner_id, page_of(&query)?)
            .await?;
        Ok(proposals_map(documents))
    }

    /// One page of every proposal, newest first.
    ///
    /// @returns The proposals by id; pass the last id as `startAfter` for the next page.
    #[wasm_bindgen(
        js_name = "getDecisionProposalsByCreatedAt",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_decision_proposals_by_created_at(
        &self,
        query: Option<DecisionProposalsPageQueryJs>,
    ) -> Result<Map, WasmSdkError> {
        let query: JsValue = query.map(Into::into).unwrap_or(JsValue::UNDEFINED);
        let documents = self
            .as_ref()
            .fetch_decision_proposals_by_created_at(page_of(&query)?)
            .await?;
        Ok(proposals_map(documents))
    }

    /// The id of a proposal's yes/no poll, the id `getVotePollsByEndDate` lists it under and
    /// votes name it by.
    ///
    /// @param proposalId - The proposal.
    /// @returns The poll's id.
    #[wasm_bindgen(js_name = "getDecisionProposalPollId")]
    pub async fn get_decision_proposal_poll_id(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<IdentifierWasm, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(self
            .as_ref()
            .decision_proposal_vote_poll_id(proposal_id)
            .await?
            .into())
    }

    /// The status and tallies of a proposal's poll: started, with the voting power cast so
    /// far, or finished, with the result it records. `undefined` for a proposal never created.
    ///
    /// # Example
    /// ```javascript
    /// const state = await sdk.getDecisionProposalPollState(proposalId);
    /// if (state?.status === 'finished') console.log(state.result.passed);
    /// ```
    #[wasm_bindgen(
        js_name = "getDecisionProposalPollState",
        unchecked_return_type = "YesNoVotePollState | undefined"
    )]
    pub async fn get_decision_proposal_poll_state(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let query = YesNoVotePollStateQuery::new(proposal_purpose(proposal_id)?);
        let state = YesNoVotePollState::fetch(self.as_ref(), query).await?;
        state_to_js(state)
    }

    /// A masternode's answer on a proposal and how many times it has voted on it. `undefined`
    /// when the masternode has not voted on it, and once the poll finished.
    #[wasm_bindgen(
        js_name = "getMasternodeDecisionProposalVote",
        unchecked_return_type = "MasternodeYesNoVote | undefined"
    )]
    pub async fn get_masternode_decision_proposal_vote(
        &self,
        #[wasm_bindgen(js_name = "proTxHash")] pro_tx_hash: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let pro_tx_hash: Identifier = pro_tx_hash
            .try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid proTxHash: {err}")))?;
        let query = MasternodeYesNoVoteQuery::new(pro_tx_hash, proposal_purpose(proposal_id)?);
        let vote = MasternodeYesNoVote::fetch(self.as_ref(), query).await?;
        vote_to_js(vote)
    }

    /// Votes on a proposal for a masternode: builds the vote, signs it with the voting key,
    /// broadcasts it and waits for the block that records it. Every option is read before
    /// anything reaches the network.
    ///
    /// @param options - The masternode, the proposal, the answer, the voting key and signer.
    /// @returns The vote as Platform recorded it.
    #[wasm_bindgen(js_name = "decisionProposalVote")]
    pub async fn decision_proposal_vote(
        &self,
        options: DecisionProposalVoteOptionsJs,
    ) -> Result<VoteWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let pro_tx_hash: Identifier =
            IdentifierWasm::try_from_options(&options, "masternodeProTxHash")?.into();
        let proposal_id: Identifier =
            IdentifierWasm::try_from_options(&options, "proposalId")?.into();
        let vote_choice = yes_no_vote_choice_from_options(&options, "voteChoice")?;
        let signing = MasternodeVoteSigning::from_options(&options)?;
        let vote = self
            .as_ref()
            .decision_proposal_vote(proposal_id, vote_choice)
            .await?;
        self.put_masternode_vote(pro_tx_hash, vote, signing).await
    }
}
