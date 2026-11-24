//! Multiple choice proposals (protocol version 15): questions whose answers are the choices
//! entered for them. Thin bindings over
//! `dash_sdk::platform::decision_proposals::multiple_choice`: a proposal is a
//! `multipleChoiceProposal` document and an answer a `choice` document of the decision
//! proposals system contract, created like any document, and the choices contend in a
//! masternode vote named by the proposal's id.

use super::{optional_string_option, page_of, string_option};
use crate::error::WasmSdkError;
use crate::queries::utils::documents_map;
use crate::queries::voting::yes_no::{block_to_js, set};
use crate::sdk::WasmSdk;
use crate::state_transitions::identity::MasternodeVoteSigning;
use dash_sdk::dpp::decision_proposal::multiple_choice::CHOICE_DOCUMENT_TYPE_NAME;
use dash_sdk::dpp::decision_proposal::{
    DECISION_PROPOSALS_CONTRACT_ID, MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dash_sdk::dpp::document::Document;
use dash_sdk::dpp::voting::vote_polls::VotePoll;
use dash_sdk::platform::decision_proposals::multiple_choice::{
    build_multiple_choice_proposal_edit, multiple_choice_proposal_vote,
    multiple_choice_proposal_vote_poll, ChoiceInput, DescriptionDocument,
    MultipleChoiceProposalEdit, MultipleChoiceProposalInput, MultipleChoiceProposalVote,
    MultipleChoiceVoteChoice,
};
use dash_sdk::platform::Identifier;
use js_sys::{Map, Object, Reflect};
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_dpp2::data_contract::document::DocumentWasm;
use wasm_dpp2::error::WasmDppError;
use wasm_dpp2::identifier::{IdentifierLikeJs, IdentifierWasm};
use wasm_dpp2::utils::{try_from_options_with, try_to_string};
use wasm_dpp2::voting::vote::VoteWasm;
use wasm_dpp2::VotePollWasm;

#[wasm_bindgen(typescript_custom_section)]
const MULTIPLE_CHOICE_PROPOSALS_TS: &'static str = r#"
/**
 * The document a multiple choice proposal points at for what is being decided. Platform
 * refuses one that is not of a moderated type the elected team of its contract may delete.
 */
export interface MultipleChoiceProposalDescriptionDocument {
  /** The document. */
  id: IdentifierLike;
  /** The contract it lives in, one whose moderators the masternodes elect. */
  contractId: IdentifierLike;
  /** Its document type. */
  documentType: string;
}

/**
 * Options for `buildMultipleChoiceProposal`: a question put to the masternodes whose answers
 * are the choices entered for it (protocol version 15). Its owner pays the multiple choice
 * proposal processing fee (`getMultipleChoiceProposalProcessingFee`) on top of the fees.
 * Platform refuses a proposal that gives neither a description document nor a url.
 */
export interface MultipleChoiceProposalOptions {
  /** The proposer, the document's owner. */
  ownerId: IdentifierLike;
  /** The question, in one line: 1 to 128 characters. */
  title: string;
  /** The document saying what is being decided. */
  descriptionDocument?: MultipleChoiceProposalDescriptionDocument;
  /**
   * Where what is being decided is described, for a proposal that points at no description
   * document, or where it is discussed, for one that does: a URI of at most 512 characters.
   * Nothing on chain moderates it.
   */
  url?: string;
  /**
   * Whether anyone may enter choices; only the proposal's owner may when it is false. It
   * never changes.
   */
  open: boolean;
}

/**
 * What a multiple choice proposal changes to (`buildMultipleChoiceProposalEdit`). A field left
 * out keeps its value. Platform accepts the edit for two hours after the proposal is created;
 * whether it is open never changes.
 */
export interface MultipleChoiceProposalEdit {
  /** The new title. */
  title?: string;
  /** The new description document; `null` leaves the proposal described at its url alone. */
  descriptionDocument?: MultipleChoiceProposalDescriptionDocument | null;
  /** The new url; `null` removes it. */
  url?: string | null;
}

/**
 * Options for `buildMultipleChoiceProposalChoice`: an answer to a multiple choice proposal.
 * Its owner pays the contest fund (`getMultipleChoiceProposalChoiceFund`) on top of the fees.
 * Anyone enters choices for an open proposal, and only its owner for one that is not.
 */
export interface MultipleChoiceProposalChoiceOptions {
  /** The identity entering the choice, the document's owner. */
  ownerId: IdentifierLike;
  /** The proposal the choice answers. */
  proposalId: IdentifierLike;
  /** The answer, in one line: 1 to 128 characters. */
  title: string;
  /** What voting for the answer means: 1 to 4096 characters. */
  description?: string;
}

/**
 * A masternode's answer in the vote among a multiple choice proposal's choices: a choice, by
 * its document id (not the id of the identity that entered it); `'abstain'`; or
 * `'noneOfTheChoices'`, which keeps no choice when these votes outnumber the leading choice.
 */
export type MultipleChoiceVoteChoice =
  | { choiceId: IdentifierLike }
  | 'abstain'
  | 'noneOfTheChoices';

/** The answer a masternode gave (`getMasternodeMultipleChoiceProposalVote`). */
export type MasternodeMultipleChoiceVote =
  | { choiceId: Identifier }
  | 'abstain'
  | 'noneOfTheChoices';

/** A choice in the vote among a multiple choice proposal's choices. */
export interface MultipleChoiceProposalChoiceInVote {
  /**
   * The `choice` document, while the vote runs. Left out once it ended: the choices that were
   * not kept are no longer in state, and `getMultipleChoiceProposalKeptChoice` reads the one
   * that was.
   */
  choice?: Document;
  /** The votes towards the choice: so far while the vote runs, final once it ended. */
  voteTally?: number;
}

/** The vote among a multiple choice proposal's choices (`getMultipleChoiceProposalVote`). */
export interface MultipleChoiceProposalVote {
  /** The choices by their base58 document ids. Empty for a proposal with no choice. */
  choices: Map<string, MultipleChoiceProposalChoiceInVote>;
  /** The abstaining votes. */
  abstainVoteTally?: number;
  /** The votes for none of the choices. */
  noneOfTheChoicesVoteTally?: number;
  /** How the vote ended; left out while it runs. */
  result?: {
    /** The document id of the choice that was kept; left out when none was. */
    keptChoiceId?: Identifier;
    /** The block the vote ended at. */
    finalizationBlock: YesNoVotePollBlock;
  };
}

/** Options for `multipleChoiceProposalVote`: a masternode's answer on a proposal. */
export interface MultipleChoiceProposalVoteOptions {
  /**
   * The ProTxHash of the masternode: an Identifier, its 32 bytes, the hex Core's RPC shows,
   * or base58.
   */
  masternodeProTxHash: IdentifierLike;
  /** The proposal. */
  proposalId: IdentifierLike;
  /** The answer. */
  voteChoice: MultipleChoiceVoteChoice;
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
    #[wasm_bindgen(typescript_type = "MultipleChoiceProposalOptions")]
    pub type MultipleChoiceProposalOptionsJs;

    #[wasm_bindgen(typescript_type = "MultipleChoiceProposalEdit")]
    pub type MultipleChoiceProposalEditJs;

    #[wasm_bindgen(typescript_type = "MultipleChoiceProposalChoiceOptions")]
    pub type MultipleChoiceProposalChoiceOptionsJs;

    #[wasm_bindgen(typescript_type = "MultipleChoiceProposalVoteOptions")]
    pub type MultipleChoiceProposalVoteOptionsJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalsByOwnerQuery")]
    pub type MultipleChoiceProposalsByOwnerQueryJs;

    #[wasm_bindgen(typescript_type = "DecisionProposalsPageQuery")]
    pub type MultipleChoiceProposalsPageQueryJs;
}

/// The description document `value` names.
fn description_document(value: &JsValue) -> Result<DescriptionDocument, WasmSdkError> {
    Ok(DescriptionDocument {
        id: IdentifierWasm::try_from_options(value, "id")?.into(),
        contract_id: IdentifierWasm::try_from_options(value, "contractId")?.into(),
        document_type: string_option(value, "documentType")?,
    })
}

/// The property `property` of `options`: `None` when it is left out, `Some(None)` when it is
/// `null`, and what `read` makes of it otherwise.
fn nullable_option<T>(
    options: &JsValue,
    property: &str,
    read: impl FnOnce(&JsValue) -> Result<T, WasmSdkError>,
) -> Result<Option<Option<T>>, WasmSdkError> {
    let value = Reflect::get(options, &JsValue::from_str(property))
        .map_err(|_| WasmSdkError::invalid_argument(format!("cannot read {property}")))?;
    if value.is_undefined() {
        Ok(None)
    } else if value.is_null() {
        Ok(Some(None))
    } else {
        read(&value).map(|value| Some(Some(value)))
    }
}

/// The answer `property` of `options` gives: `{ choiceId }`, `'abstain'` or
/// `'noneOfTheChoices'`.
fn vote_choice_from_options(
    options: &JsValue,
    property: &str,
) -> Result<MultipleChoiceVoteChoice, WasmSdkError> {
    let value = Reflect::get(options, &JsValue::from_str(property))
        .map_err(|_| WasmSdkError::invalid_argument(format!("cannot read {property}")))?;
    match value.as_string().as_deref() {
        Some("abstain") => Ok(MultipleChoiceVoteChoice::Abstain),
        Some("noneOfTheChoices") => Ok(MultipleChoiceVoteChoice::NoneOfTheChoices),
        Some(other) => Err(WasmSdkError::invalid_argument(format!(
            "{property} on a multiple choice proposal is {{ choiceId }}, 'abstain' or \
             'noneOfTheChoices', got '{other}'"
        ))),
        None if value.is_object() => Ok(MultipleChoiceVoteChoice::Choice(
            IdentifierWasm::try_from_options(&value, "choiceId")?.into(),
        )),
        None => Err(WasmSdkError::invalid_argument(format!(
            "{property} on a multiple choice proposal is {{ choiceId }}, 'abstain' or \
             'noneOfTheChoices'"
        ))),
    }
}

/// `vote_choice` as `getMasternodeMultipleChoiceProposalVote` returns it.
fn vote_choice_to_js(vote_choice: MultipleChoiceVoteChoice) -> Result<JsValue, WasmSdkError> {
    Ok(match vote_choice {
        MultipleChoiceVoteChoice::Choice(choice_id) => {
            let object = Object::new();
            set(&object, "choiceId", IdentifierWasm::from(choice_id).into())?;
            object.into()
        }
        MultipleChoiceVoteChoice::Abstain => JsValue::from_str("abstain"),
        MultipleChoiceVoteChoice::NoneOfTheChoices => JsValue::from_str("noneOfTheChoices"),
    })
}

fn document_wasm(
    document: Document,
    document_type_name: &str,
    entropy: Option<[u8; 32]>,
) -> DocumentWasm {
    DocumentWasm::new(
        document,
        DECISION_PROPOSALS_CONTRACT_ID,
        document_type_name.to_string(),
        entropy,
    )
}

/// `vote` as `getMultipleChoiceProposalVote` returns it.
fn vote_to_js(vote: MultipleChoiceProposalVote) -> Result<JsValue, WasmSdkError> {
    let kept_choice_id = vote.kept_choice_id();
    let choices = Map::new();
    for (choice_id, choice_in_vote) in vote.choices {
        let entry = Object::new();
        if let Some(choice) = choice_in_vote.choice {
            set(
                &entry,
                "choice",
                document_wasm(choice.document, CHOICE_DOCUMENT_TYPE_NAME, None).into(),
            )?;
        }
        if let Some(vote_tally) = choice_in_vote.vote_tally {
            set(&entry, "voteTally", JsValue::from(vote_tally))?;
        }
        choices.set(
            &IdentifierWasm::from(choice_id).to_base58().into(),
            &entry.into(),
        );
    }
    let object = Object::new();
    set(&object, "choices", choices.into())?;
    if let Some(abstain_vote_tally) = vote.abstain_vote_tally {
        set(
            &object,
            "abstainVoteTally",
            JsValue::from(abstain_vote_tally),
        )?;
    }
    if let Some(none_of_the_choices_vote_tally) = vote.none_of_the_choices_vote_tally {
        set(
            &object,
            "noneOfTheChoicesVoteTally",
            JsValue::from(none_of_the_choices_vote_tally),
        )?;
    }
    if let Some((_, finalization_block)) = &vote.winner {
        let result = Object::new();
        if let Some(kept_choice_id) = kept_choice_id {
            set(
                &result,
                "keptChoiceId",
                IdentifierWasm::from(kept_choice_id).into(),
            )?;
        }
        set(
            &result,
            "finalizationBlock",
            block_to_js(finalization_block)?,
        )?;
        set(&object, "result", result.into())?;
    }
    Ok(object.into())
}

#[wasm_bindgen]
impl WasmSdk {
    /// Builds a multiple choice proposal: a `multipleChoiceProposal` document of the decision
    /// proposals contract with a fresh entropy, to create with `documentCreate`. Its owner
    /// pays the multiple choice proposal processing fee on top of the fees; the create does
    /// not state it. The id is final once `documentCreate` returns the stored document.
    ///
    /// @param options - The proposer, the question, where it is described and whether anyone
    /// may enter choices.
    /// @returns The proposal, ready for `documentCreate`.
    #[wasm_bindgen(js_name = "buildMultipleChoiceProposal")]
    pub async fn build_multiple_choice_proposal(
        &self,
        options: MultipleChoiceProposalOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let owner_id: Identifier = IdentifierWasm::try_from_options(&options, "ownerId")?.into();
        let input = MultipleChoiceProposalInput {
            title: string_option(&options, "title")?,
            description_document: nullable_option(
                &options,
                "descriptionDocument",
                description_document,
            )?
            .flatten(),
            url: optional_string_option(&options, "url")?,
            open: try_from_options_with(&options, "open", |value| {
                value
                    .as_bool()
                    .ok_or_else(|| WasmDppError::invalid_argument("'open' must be a boolean"))
            })?,
        };
        let built = self
            .as_ref()
            .build_multiple_choice_proposal(owner_id, &input)
            .await?;
        Ok(document_wasm(
            built.document,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            Some(built.entropy.0),
        ))
    }

    /// A multiple choice proposal with what the edit changes and its next revision, to
    /// replace with `documentReplace`. Platform accepts the edit for two hours after the
    /// proposal is created. A document that is not a multiple choice proposal is refused.
    ///
    /// @param proposal - The proposal as stored, from `getMultipleChoiceProposal`.
    /// @param edit - The new title, description document or url; `null` removes the
    /// description document or the url.
    /// @returns The edited proposal.
    #[wasm_bindgen(js_name = "buildMultipleChoiceProposalEdit")]
    pub fn build_multiple_choice_proposal_edit(
        &self,
        proposal: &DocumentWasm,
        edit: MultipleChoiceProposalEditJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let edit: JsValue = edit.into();
        let edit = MultipleChoiceProposalEdit {
            title: optional_string_option(&edit, "title")?,
            description_document: nullable_option(
                &edit,
                "descriptionDocument",
                description_document,
            )?,
            url: nullable_option(&edit, "url", |value| Ok(try_to_string(value, "url")?))?,
        };
        let data_contract_id: Identifier = proposal.data_contract_id().into();
        let document: Document = proposal.clone().into();
        Ok(document_wasm(
            build_multiple_choice_proposal_edit(
                &data_contract_id,
                &proposal.document_type_name(),
                &document,
                &edit,
            )?,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            None,
        ))
    }

    /// Builds a choice: a `choice` document of the decision proposals contract with a fresh
    /// entropy, to create with `documentCreate`, which states the contest fund. The first
    /// choice opens the vote, and later ones join it while its join window is open. The id,
    /// which a vote towards the choice carries, is final once `documentCreate` returns the
    /// stored document.
    ///
    /// @param options - The identity entering the choice, the proposal and the answer.
    /// @returns The choice, ready for `documentCreate`.
    #[wasm_bindgen(js_name = "buildMultipleChoiceProposalChoice")]
    pub async fn build_multiple_choice_proposal_choice(
        &self,
        options: MultipleChoiceProposalChoiceOptionsJs,
    ) -> Result<DocumentWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let owner_id: Identifier = IdentifierWasm::try_from_options(&options, "ownerId")?.into();
        let input = ChoiceInput {
            proposal_id: IdentifierWasm::try_from_options(&options, "proposalId")?.into(),
            title: string_option(&options, "title")?,
            description: optional_string_option(&options, "description")?,
        };
        let built = self.as_ref().build_choice(owner_id, &input).await?;
        Ok(document_wasm(
            built.document,
            CHOICE_DOCUMENT_TYPE_NAME,
            Some(built.entropy.0),
        ))
    }

    /// What creating one multiple choice proposal adds to the processing fee of its batch, in
    /// credits.
    #[wasm_bindgen(js_name = "getMultipleChoiceProposalProcessingFee")]
    pub async fn get_multiple_choice_proposal_processing_fee(&self) -> Result<u64, WasmSdkError> {
        Ok(self
            .as_ref()
            .multiple_choice_proposal_processing_fee()
            .await?)
    }

    /// The fund a choice entered now for a proposal pays, in credits: the contest's fund,
    /// doubled once the vote holds 250 choices and again for every 50 more.
    ///
    /// @param proposalId - The proposal.
    #[wasm_bindgen(js_name = "getMultipleChoiceProposalChoiceFund")]
    pub async fn get_multiple_choice_proposal_choice_fund(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<u64, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(self.as_ref().choice_fund_to_join(proposal_id).await?)
    }

    /// A multiple choice proposal by id.
    ///
    /// @param proposalId - The proposal.
    /// @returns The `multipleChoiceProposal` document, or undefined when there is none.
    #[wasm_bindgen(js_name = "getMultipleChoiceProposal")]
    pub async fn get_multiple_choice_proposal(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_multiple_choice_proposal(proposal_id)
            .await?
            .map(|document| {
                document_wasm(document, MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME, None)
            }))
    }

    /// One page of an identity's multiple choice proposals, by creation time.
    ///
    /// @returns The proposals by id; pass the last id as `startAfter` for the next page.
    #[wasm_bindgen(
        js_name = "getMultipleChoiceProposalsByOwner",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_multiple_choice_proposals_by_owner(
        &self,
        query: MultipleChoiceProposalsByOwnerQueryJs,
    ) -> Result<Map, WasmSdkError> {
        let query: JsValue = query.into();
        if query.is_undefined() || query.is_null() {
            return Err(WasmSdkError::invalid_argument("Query object is required"));
        }
        let owner_id: Identifier = IdentifierWasm::try_from_options(&query, "ownerId")?.into();
        let documents = self
            .as_ref()
            .fetch_multiple_choice_proposals_by_owner(owner_id, page_of(&query)?)
            .await?;
        Ok(documents_map(
            documents,
            DECISION_PROPOSALS_CONTRACT_ID,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
        ))
    }

    /// One page of every multiple choice proposal, newest first.
    ///
    /// @returns The proposals by id; pass the last id as `startAfter` for the next page.
    #[wasm_bindgen(
        js_name = "getMultipleChoiceProposalsByCreatedAt",
        unchecked_return_type = "Map<string, Document | undefined>"
    )]
    pub async fn get_multiple_choice_proposals_by_created_at(
        &self,
        query: Option<MultipleChoiceProposalsPageQueryJs>,
    ) -> Result<Map, WasmSdkError> {
        let query: JsValue = query.map(Into::into).unwrap_or(JsValue::UNDEFINED);
        let documents = self
            .as_ref()
            .fetch_multiple_choice_proposals_by_created_at(page_of(&query)?)
            .await?;
        Ok(documents_map(
            documents,
            DECISION_PROPOSALS_CONTRACT_ID,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
        ))
    }

    /// The vote among a proposal's choices as a vote poll, which `masternodeVote`,
    /// `getContestedResourceVoteState` and the other contested resource queries take. In it a
    /// choice is named by its document id wherever they name a contender by an identity id.
    ///
    /// @param proposalId - The proposal.
    #[wasm_bindgen(js_name = "getMultipleChoiceProposalVotePoll")]
    pub fn get_multiple_choice_proposal_vote_poll(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<VotePollWasm, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(
            VotePoll::ContestedDocumentResourceVotePoll(multiple_choice_proposal_vote_poll(
                proposal_id,
            ))
            .into(),
        )
    }

    /// The vote among a proposal's choices: every choice with its tally while the vote runs,
    /// and its result with every choice's final tally once it ended. A proposal no choice was
    /// entered for has no choices.
    ///
    /// # Example
    /// ```javascript
    /// const vote = await sdk.getMultipleChoiceProposalVote(proposalId);
    /// for (const [choiceId, { choice, voteTally }] of vote.choices) {
    ///   console.log(choiceId, choice?.properties.title, voteTally);
    /// }
    /// ```
    #[wasm_bindgen(
        js_name = "getMultipleChoiceProposalVote",
        unchecked_return_type = "MultipleChoiceProposalVote"
    )]
    pub async fn get_multiple_choice_proposal_vote(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        let vote = self
            .as_ref()
            .fetch_multiple_choice_proposal_vote(proposal_id)
            .await?;
        vote_to_js(vote)
    }

    /// The choice the finished vote of a proposal kept, an ordinary document of the identity
    /// that entered it. Undefined while the vote runs, when it kept none of the choices, and
    /// for a proposal no choice was entered for.
    ///
    /// @param proposalId - The proposal.
    #[wasm_bindgen(js_name = "getMultipleChoiceProposalKeptChoice")]
    pub async fn get_multiple_choice_proposal_kept_choice(
        &self,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<Option<DocumentWasm>, WasmSdkError> {
        let proposal_id: Identifier = proposal_id.try_into()?;
        Ok(self
            .as_ref()
            .fetch_kept_choice(proposal_id)
            .await?
            .map(|document| document_wasm(document, CHOICE_DOCUMENT_TYPE_NAME, None)))
    }

    /// The answer a masternode gave in the vote among a proposal's choices. Undefined when
    /// the masternode has not voted in it, and once the vote ended.
    #[wasm_bindgen(
        js_name = "getMasternodeMultipleChoiceProposalVote",
        unchecked_return_type = "MasternodeMultipleChoiceVote | undefined"
    )]
    pub async fn get_masternode_multiple_choice_proposal_vote(
        &self,
        #[wasm_bindgen(js_name = "proTxHash")] pro_tx_hash: IdentifierLikeJs,
        #[wasm_bindgen(js_name = "proposalId")] proposal_id: IdentifierLikeJs,
    ) -> Result<JsValue, WasmSdkError> {
        let pro_tx_hash: Identifier = pro_tx_hash
            .try_into()
            .map_err(|err| WasmSdkError::invalid_argument(format!("Invalid proTxHash: {err}")))?;
        let proposal_id: Identifier = proposal_id.try_into()?;
        match self
            .as_ref()
            .fetch_masternode_multiple_choice_vote(pro_tx_hash, proposal_id)
            .await?
        {
            Some(vote_choice) => vote_choice_to_js(vote_choice),
            None => Ok(JsValue::UNDEFINED),
        }
    }

    /// Votes in the vote among a proposal's choices for a masternode: builds the vote, signs
    /// it with the voting key, broadcasts it and waits for the block that records it. Every
    /// option is read before anything reaches the network.
    ///
    /// @param options - The masternode, the proposal, the answer, the voting key and signer.
    /// @returns The vote as Platform recorded it.
    #[wasm_bindgen(js_name = "multipleChoiceProposalVote")]
    pub async fn multiple_choice_proposal_vote(
        &self,
        options: MultipleChoiceProposalVoteOptionsJs,
    ) -> Result<VoteWasm, WasmSdkError> {
        let options: JsValue = options.into();
        let pro_tx_hash: Identifier =
            IdentifierWasm::try_from_options(&options, "masternodeProTxHash")?.into();
        let proposal_id: Identifier =
            IdentifierWasm::try_from_options(&options, "proposalId")?.into();
        let vote_choice = vote_choice_from_options(&options, "voteChoice")?;
        let signing = MasternodeVoteSigning::from_options(&options)?;
        let vote = multiple_choice_proposal_vote(proposal_id, vote_choice);
        self.put_masternode_vote(pro_tx_hash, vote, signing).await
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod binding_tests {
    use super::*;
    use dash_sdk::dpp::block::block_info::BlockInfo;
    use dash_sdk::dpp::decision_proposal::multiple_choice::choice_properties;
    use dash_sdk::dpp::document::DocumentV0;
    use dash_sdk::dpp::platform_value::Value;
    use dash_sdk::dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
    use dash_sdk::platform::decision_proposals::multiple_choice::{Choice, ChoiceInVote};
    use std::collections::BTreeMap;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn get(value: &JsValue, key: &str) -> JsValue {
        Reflect::get(value, &JsValue::from_str(key)).expect("a property")
    }

    fn base58(id: Identifier) -> String {
        IdentifierWasm::from(id).to_base58()
    }

    /// The choice `title` of the proposal `[7; 32]`, with the document id `id`.
    fn choice(id: Identifier, title: &str) -> Choice {
        Choice::from_document(
            DocumentV0 {
                id,
                owner_id: Identifier::new([4; 32]),
                properties: BTreeMap::from([
                    (
                        choice_properties::PROPOSAL_ID.to_string(),
                        Value::Identifier([7; 32]),
                    ),
                    (
                        choice_properties::TITLE.to_string(),
                        Value::Text(title.to_string()),
                    ),
                ]),
                ..Default::default()
            }
            .into(),
        )
        .expect("a choice")
    }

    fn choices_of(vote: &JsValue) -> Map {
        let choices = get(vote, "choices");
        assert!(choices.is_instance_of::<Map>(), "a Map: {choices:?}");
        choices.unchecked_into()
    }

    fn finalization_block() -> BlockInfo {
        BlockInfo {
            time_ms: 1_700_000_000_000,
            height: 7,
            core_height: 5,
            ..Default::default()
        }
    }

    #[wasm_bindgen_test]
    fn should_give_a_vote_with_no_choice_an_empty_map_and_nothing_else() {
        let vote = vote_to_js(MultipleChoiceProposalVote::default()).expect("converts");
        assert_eq!(choices_of(&vote).size(), 0);
        for property in ["abstainVoteTally", "noneOfTheChoicesVoteTally", "result"] {
            assert!(get(&vote, property).is_undefined(), "{property}");
        }
    }

    #[wasm_bindgen_test]
    fn should_give_a_running_vote_its_choices_by_document_id_with_their_tallies() {
        let double_id = Identifier::new([1; 32]);
        let keep_id = Identifier::new([2; 32]);
        let vote = vote_to_js(MultipleChoiceProposalVote {
            choices: BTreeMap::from([
                (
                    double_id,
                    ChoiceInVote {
                        choice: Some(choice(double_id, "Double it")),
                        vote_tally: Some(12),
                    },
                ),
                (
                    keep_id,
                    ChoiceInVote {
                        choice: Some(choice(keep_id, "Keep it")),
                        vote_tally: None,
                    },
                ),
            ]),
            abstain_vote_tally: Some(3),
            none_of_the_choices_vote_tally: Some(1),
            winner: None,
        })
        .expect("converts");

        let choices = choices_of(&vote);
        assert_eq!(choices.size(), 2);
        let double = choices.get(&base58(double_id).into());
        assert_eq!(get(&double, "voteTally").as_f64(), Some(12.0));
        let document = get(&double, "choice");
        assert_eq!(
            get(&document, "documentTypeName").as_string().as_deref(),
            Some(CHOICE_DOCUMENT_TYPE_NAME)
        );
        assert_eq!(
            IdentifierWasm::try_from_options(&document, "id")
                .expect("an id")
                .to_base58(),
            base58(double_id)
        );
        assert_eq!(
            IdentifierWasm::try_from_options(&document, "dataContractId")
                .expect("a contract id")
                .to_base58(),
            base58(DECISION_PROPOSALS_CONTRACT_ID)
        );
        let keep = choices.get(&base58(keep_id).into());
        assert!(!get(&keep, "choice").is_undefined());
        assert!(get(&keep, "voteTally").is_undefined());

        assert_eq!(get(&vote, "abstainVoteTally").as_f64(), Some(3.0));
        assert_eq!(get(&vote, "noneOfTheChoicesVoteTally").as_f64(), Some(1.0));
        assert!(get(&vote, "result").is_undefined());
    }

    #[wasm_bindgen_test]
    fn should_give_a_vote_that_kept_a_choice_its_id_and_every_final_tally() {
        let double_id = Identifier::new([1; 32]);
        let keep_id = Identifier::new([2; 32]);
        let tally = |vote_tally| ChoiceInVote {
            choice: None,
            vote_tally: Some(vote_tally),
        };
        let vote = vote_to_js(MultipleChoiceProposalVote {
            choices: BTreeMap::from([(double_id, tally(12)), (keep_id, tally(4))]),
            abstain_vote_tally: Some(3),
            none_of_the_choices_vote_tally: Some(1),
            winner: Some((
                ContestedDocumentVotePollWinnerInfo::WonByIdentity(double_id),
                finalization_block(),
            )),
        })
        .expect("converts");

        let choices = choices_of(&vote);
        for (choice_id, vote_tally) in [(double_id, 12.0), (keep_id, 4.0)] {
            let entry = choices.get(&base58(choice_id).into());
            assert!(get(&entry, "choice").is_undefined());
            assert_eq!(get(&entry, "voteTally").as_f64(), Some(vote_tally));
        }
        let result = get(&vote, "result");
        assert_eq!(
            IdentifierWasm::try_from_options(&result, "keptChoiceId")
                .expect("the kept choice")
                .to_base58(),
            base58(double_id)
        );
        let block = get(&result, "finalizationBlock");
        assert_eq!(get(&block, "height"), JsValue::from(7u64));
        assert_eq!(get(&block, "coreHeight").as_f64(), Some(5.0));
        assert_eq!(get(&block, "timeMs"), JsValue::from(1_700_000_000_000u64));
    }

    #[wasm_bindgen_test]
    fn should_give_a_vote_that_kept_no_choice_a_result_without_a_kept_choice() {
        let vote = vote_to_js(MultipleChoiceProposalVote {
            choices: BTreeMap::from([(
                Identifier::new([1; 32]),
                ChoiceInVote {
                    choice: None,
                    vote_tally: Some(2),
                },
            )]),
            abstain_vote_tally: Some(0),
            none_of_the_choices_vote_tally: Some(9),
            winner: Some((
                ContestedDocumentVotePollWinnerInfo::Locked,
                finalization_block(),
            )),
        })
        .expect("converts");

        let result = get(&vote, "result");
        assert!(result.is_object());
        assert!(get(&result, "keptChoiceId").is_undefined());
        assert!(get(&result, "finalizationBlock").is_object());
        assert_eq!(get(&vote, "abstainVoteTally").as_f64(), Some(0.0));
        assert_eq!(get(&vote, "noneOfTheChoicesVoteTally").as_f64(), Some(9.0));
    }

    #[wasm_bindgen_test]
    fn should_read_a_masternode_answer_back_as_it_is_given() {
        let choice_id = Identifier::new([1; 32]);
        let options = |vote_choice: JsValue| {
            let options = Object::new();
            set(&options, "voteChoice", vote_choice).expect("sets");
            JsValue::from(options)
        };
        for vote_choice in [
            MultipleChoiceVoteChoice::Choice(choice_id),
            MultipleChoiceVoteChoice::Abstain,
            MultipleChoiceVoteChoice::NoneOfTheChoices,
        ] {
            let given = vote_choice_to_js(vote_choice).expect("converts");
            assert_eq!(
                vote_choice_from_options(&options(given), "voteChoice").expect("reads"),
                vote_choice
            );
        }
        assert!(vote_choice_from_options(&options("lock".into()), "voteChoice").is_err());
        assert!(vote_choice_from_options(&options(JsValue::from(1)), "voteChoice").is_err());
    }
}
