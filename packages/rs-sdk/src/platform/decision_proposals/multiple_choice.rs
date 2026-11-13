//! Multiple choice proposals (protocol version 15): questions whose answers are the choices
//! entered for them. A proposal is a `multipleChoiceProposal` document of the decision
//! proposals system contract and an answer is a `choice` document; the choices contend in a
//! masternode vote that the first choice opens, and the one with the most votes is kept.
//!
//! * [`Sdk::build_multiple_choice_proposal`] builds a proposal, put like any document create
//!   with [`PutDocument`](crate::platform::transition::put_document::PutDocument). Its owner
//!   pays the multiple choice proposal processing fee
//!   ([`Sdk::multiple_choice_proposal_processing_fee`]) on top of the fees; the create does
//!   not state it.
//! * [`build_multiple_choice_proposal_edit`] gives a proposal a new title, url or description
//!   document, put as a replace, for two hours after the proposal is created.
//! * [`Sdk::build_choice`] builds an answer to a proposal, put like any document create. The
//!   put states the contest fund ([`Sdk::choice_fund_to_join`]), which its owner pays. Anyone
//!   enters choices for an open proposal, and only its owner for one that is not.
//! * [`Sdk::fetch_multiple_choice_proposal`] reads a proposal by id, and two listings read
//!   them a page at a time: [`Sdk::fetch_multiple_choice_proposals_by_owner`] in filing order
//!   and [`Sdk::fetch_multiple_choice_proposals_by_created_at`] newest first.
//!   [`MultipleChoiceProposal::from_document`] reads a proposal's properties.
//! * [`Sdk::fetch_multiple_choice_proposal_vote`] reads the vote: the choices with their
//!   tallies while it runs, and its outcome with every choice's tally afterwards.
//!   [`Sdk::fetch_kept_choice`] reads the choice a finished vote kept.
//! * [`multiple_choice_proposal_vote`] builds a masternode's answer, put with
//!   [`PutVote`](crate::platform::transition::vote::PutVote), and
//!   [`Sdk::fetch_masternode_multiple_choice_vote`] reads the answer a masternode gave.
//!
//! A choice is named by its document id wherever a contest among identities names a contender
//! by an identity id: in a vote ([`MultipleChoiceVoteChoice::Choice`]), in the vote's state and
//! in its winner. The vote itself is the contest [`multiple_choice_proposal_vote_poll`] names
//! from the proposal's id, which the contested resource queries take as it is.

use super::{
    new_document, proposals_by_created_at_query, proposals_by_owner_query, DecisionProposalDocument,
};
use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::page::DocumentsPage;
use crate::platform::query::VoteQuery;
use crate::platform::{DataContract, Document, Fetch, FetchMany, Identifier};
use crate::{Error, Sdk};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::decision_proposal::multiple_choice::{
    choice_properties, proposal_properties, CHOICE_DOCUMENT_TYPE_NAME,
};
pub use dpp::decision_proposal::multiple_choice::{
    multiple_choice_proposal_vote, multiple_choice_proposal_vote_poll, MultipleChoiceVoteChoice,
};
use dpp::decision_proposal::{
    is_multiple_choice_proposal, multiple_choice_proposal_processing_fee,
    MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::{Bytes32, Value};
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{Rng, SeedableRng};
use dpp::version::feature_initial_protocol_versions::DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
use dpp::voting::votes::Vote;
use drive::query::{WhereClause, WhereOperator};
use drive_proof_verifier::types::{Contenders, Documents};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The document a multiple choice proposal points at for what is being decided. Platform
/// refuses one that is not of a moderated type the elected team of its contract may delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptionDocument {
    /// The document.
    pub id: Identifier,
    /// The contract it lives in, one whose moderators the masternodes elect.
    pub contract_id: Identifier,
    /// Its document type.
    pub document_type: String,
}

/// What a new multiple choice proposal asks. Platform refuses a proposal that gives neither a
/// description document nor a url.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultipleChoiceProposalInput {
    /// The question, in one line: 1 to 128 characters.
    pub title: String,
    /// The document saying what is being decided.
    pub description_document: Option<DescriptionDocument>,
    /// Where what is being decided is described, for a proposal that points at no description
    /// document, or where it is discussed, for one that does: a URI of at most 512 characters.
    pub url: Option<String>,
    /// Whether anyone may enter choices; only the proposal's owner may when it is false. It
    /// never changes.
    pub open: bool,
}

/// What a multiple choice proposal changes to. A field left `None` keeps its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultipleChoiceProposalEdit {
    /// The new title.
    pub title: Option<String>,
    /// The new description document: `Some(Some(document))` points the proposal at it,
    /// `Some(None)` leaves the proposal described at its url alone.
    pub description_document: Option<Option<DescriptionDocument>>,
    /// The new url: `Some(Some(url))` sets it, `Some(None)` removes it.
    pub url: Option<Option<String>>,
}

/// A multiple choice proposal: its document and the properties read out of it.
#[derive(Debug, Clone, PartialEq)]
pub struct MultipleChoiceProposal {
    /// The `multipleChoiceProposal` document.
    pub document: Document,
    /// The question, in one line.
    pub title: String,
    /// The document saying what is being decided, when the proposal points at one.
    pub description_document: Option<DescriptionDocument>,
    /// Where what is being decided is described or discussed. Nothing on chain moderates it.
    pub url: Option<String>,
    /// Whether anyone may enter choices; only the proposal's owner may when it is false.
    pub open: bool,
}

impl MultipleChoiceProposal {
    /// Reads a proposal out of a `multipleChoiceProposal` document.
    pub fn from_document(document: Document) -> Result<Self, Error> {
        let malformed = |error: dpp::platform_value::Error| {
            Error::Generic(format!(
                "multiple choice proposal {} is malformed: {error}",
                document.id()
            ))
        };
        let properties = document.properties();
        let description_document = properties
            .get_optional_identifier(proposal_properties::DESCRIPTION_ID)
            .map_err(malformed)?
            .map(|id| {
                Ok::<_, Error>(DescriptionDocument {
                    id,
                    contract_id: properties
                        .get_identifier(proposal_properties::DESCRIPTION_CONTRACT_ID)
                        .map_err(malformed)?,
                    document_type: properties
                        .get_string(proposal_properties::DESCRIPTION_DOCUMENT_TYPE)
                        .map_err(malformed)?,
                })
            })
            .transpose()?;
        Ok(Self {
            title: properties
                .get_string(proposal_properties::TITLE)
                .map_err(malformed)?,
            description_document,
            url: properties
                .get_optional_string(proposal_properties::URL)
                .map_err(malformed)?,
            open: properties
                .get_bool(proposal_properties::OPEN)
                .map_err(malformed)?,
            document,
        })
    }

    /// The proposal's id, which its choices and its vote name it by.
    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    /// The proposer, the only identity that enters choices when the proposal is not open.
    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id()
    }
}

/// An answer to enter for a multiple choice proposal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceInput {
    /// The proposal the choice answers.
    pub proposal_id: Identifier,
    /// The answer, in one line: 1 to 128 characters.
    pub title: String,
    /// What voting for the answer means: 1 to 4096 characters.
    pub description: Option<String>,
}

/// A choice: its document and the properties read out of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    /// The `choice` document.
    pub document: Document,
    /// The proposal the choice answers.
    pub proposal_id: Identifier,
    /// The answer, in one line.
    pub title: String,
    /// What voting for the answer means.
    pub description: Option<String>,
}

impl Choice {
    /// Reads a choice out of a `choice` document.
    pub fn from_document(document: Document) -> Result<Self, Error> {
        let malformed = |error: dpp::platform_value::Error| {
            Error::Generic(format!("choice {} is malformed: {error}", document.id()))
        };
        let properties = document.properties();
        Ok(Self {
            proposal_id: properties
                .get_identifier(choice_properties::PROPOSAL_ID)
                .map_err(malformed)?,
            title: properties
                .get_string(choice_properties::TITLE)
                .map_err(malformed)?,
            description: properties
                .get_optional_string(choice_properties::DESCRIPTION)
                .map_err(malformed)?,
            document,
        })
    }

    /// The choice's id, which a vote towards it carries.
    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    /// The identity that entered the choice.
    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id()
    }
}

/// A choice in the vote among a proposal's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct ChoiceInVote {
    /// The choice, while the vote runs. `None` once it ended: the choices that were not kept
    /// are no longer in state, and the one that was kept is an ordinary document
    /// ([`Sdk::fetch_kept_choice`]).
    pub choice: Option<Choice>,
    /// The votes towards the choice: so far while the vote runs, final once it ended.
    pub vote_tally: Option<u32>,
}

/// The vote among a multiple choice proposal's choices.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MultipleChoiceProposalVote {
    /// The choices by their document ids. Empty for a proposal no choice was entered for.
    pub choices: BTreeMap<Identifier, ChoiceInVote>,
    /// The abstaining votes.
    pub abstain_vote_tally: Option<u32>,
    /// The votes for none of the choices.
    pub none_of_the_choices_vote_tally: Option<u32>,
    /// How the vote ended and at which block; `None` while it runs. A winner is named by the
    /// document id of the choice that was kept; locked means none of the choices was.
    pub winner: Option<(ContestedDocumentVotePollWinnerInfo, BlockInfo)>,
}

impl MultipleChoiceProposalVote {
    /// Reads the vote out of the state of its contest, as the contested resource vote state
    /// query returns it, with `contract` the decision proposals contract.
    pub fn from_contenders(
        contract: &DataContract,
        contenders: Contenders,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let document_type = contract
            .document_type_for_name(CHOICE_DOCUMENT_TYPE_NAME)
            .map_err(|e| Error::Protocol(e.into()))?;
        let choices = contenders
            .contenders
            .iter()
            .map(|(choice_id, contender)| {
                let mut contender = contender.try_to_contender(document_type, platform_version)?;
                Ok((
                    *choice_id,
                    ChoiceInVote {
                        choice: contender
                            .take_document()
                            .map(Choice::from_document)
                            .transpose()?,
                        vote_tally: contender.vote_tally(),
                    },
                ))
            })
            .collect::<Result<_, Error>>()?;
        Ok(Self {
            choices,
            abstain_vote_tally: contenders.abstain_vote_tally,
            none_of_the_choices_vote_tally: contenders.lock_vote_tally,
            winner: contenders.winner,
        })
    }

    /// The document id of the choice the vote kept; `None` while the vote runs, and when it
    /// kept none.
    pub fn kept_choice_id(&self) -> Option<Identifier> {
        match self.winner {
            Some((ContestedDocumentVotePollWinnerInfo::WonByIdentity(choice_id), _)) => {
                Some(choice_id)
            }
            _ => None,
        }
    }
}

/// The properties naming the description document `description_document`.
fn description_document_properties(
    description_document: &DescriptionDocument,
) -> [(&'static str, Value); 3] {
    [
        (
            proposal_properties::DESCRIPTION_ID,
            Value::Identifier(description_document.id.to_buffer()),
        ),
        (
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            Value::Identifier(description_document.contract_id.to_buffer()),
        ),
        (
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
            Value::Text(description_document.document_type.clone()),
        ),
    ]
}

/// Builds a multiple choice proposal of `owner_id` asking what `input` says, in `contract`,
/// the decision proposals contract, with its id derived from `entropy`.
pub fn build_multiple_choice_proposal_document(
    contract: &DataContract,
    owner_id: Identifier,
    input: &MultipleChoiceProposalInput,
    entropy: Bytes32,
) -> Result<DecisionProposalDocument, Error> {
    let mut properties = BTreeMap::from([
        (
            proposal_properties::TITLE.to_string(),
            Value::Text(input.title.clone()),
        ),
        (
            proposal_properties::OPEN.to_string(),
            Value::Bool(input.open),
        ),
    ]);
    if let Some(description_document) = &input.description_document {
        properties.extend(
            description_document_properties(description_document)
                .map(|(name, value)| (name.to_string(), value)),
        );
    }
    if let Some(url) = &input.url {
        properties.insert(
            proposal_properties::URL.to_string(),
            Value::Text(url.clone()),
        );
    }
    new_document(
        contract,
        MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
        owner_id,
        properties,
        entropy,
    )
}

/// `proposal`, a `document_type_name` document of the contract `data_contract_id`, with what
/// `edit` changes and its next revision, ready to be put as a replace. Anything but a multiple
/// choice proposal is refused, as is an edit that leaves the proposal as it is: a replace that
/// changes nothing still pays its fees. Consensus refuses the replace more than two hours
/// after the proposal was created, one that leaves the proposal with neither a description
/// document nor a url, and any change to whether it is open.
pub fn build_multiple_choice_proposal_edit(
    data_contract_id: &Identifier,
    document_type_name: &str,
    proposal: &Document,
    edit: &MultipleChoiceProposalEdit,
) -> Result<Document, Error> {
    if !is_multiple_choice_proposal(data_contract_id, document_type_name) {
        return Err(Error::Generic(format!(
            "a {document_type_name} document of contract {data_contract_id} is not a multiple \
             choice proposal"
        )));
    }
    let mut document = proposal.clone();
    if let Some(title) = &edit.title {
        document.set(proposal_properties::TITLE, Value::Text(title.clone()));
    }
    match &edit.description_document {
        Some(Some(description_document)) => {
            for (name, value) in description_document_properties(description_document) {
                document.set(name, value);
            }
        }
        Some(None) => {
            for name in [
                proposal_properties::DESCRIPTION_ID,
                proposal_properties::DESCRIPTION_CONTRACT_ID,
                proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
            ] {
                document.remove(name);
            }
        }
        None => {}
    }
    match &edit.url {
        Some(Some(url)) => document.set(proposal_properties::URL, Value::Text(url.clone())),
        Some(None) => {
            document.remove(proposal_properties::URL);
        }
        None => {}
    }
    if document.properties() == proposal.properties() {
        return Err(Error::Generic(format!(
            "the edit of multiple choice proposal {} changes nothing",
            proposal.id()
        )));
    }
    document.increment_revision()?;
    Ok(document)
}

/// Builds a choice of `owner_id` answering what `input` says, in `contract`, the decision
/// proposals contract, with its id derived from `entropy`.
pub fn build_choice_document(
    contract: &DataContract,
    owner_id: Identifier,
    input: &ChoiceInput,
    entropy: Bytes32,
) -> Result<DecisionProposalDocument, Error> {
    let mut properties = BTreeMap::from([
        (
            choice_properties::PROPOSAL_ID.to_string(),
            Value::Identifier(input.proposal_id.to_buffer()),
        ),
        (
            choice_properties::TITLE.to_string(),
            Value::Text(input.title.clone()),
        ),
    ]);
    if let Some(description) = &input.description {
        properties.insert(
            choice_properties::DESCRIPTION.to_string(),
            Value::Text(description.clone()),
        );
    }
    new_document(
        contract,
        CHOICE_DOCUMENT_TYPE_NAME,
        owner_id,
        properties,
        entropy,
    )
}

/// The query for the choice the finished vote of the proposal `proposal_id` kept, which the
/// `byProposal` index holds as the one choice of that proposal.
fn kept_choice_query(
    contract: Arc<DataContract>,
    proposal_id: Identifier,
) -> Result<DocumentQuery, Error> {
    Ok(
        DocumentQuery::new(contract, CHOICE_DOCUMENT_TYPE_NAME)?.with_where(WhereClause {
            field: choice_properties::PROPOSAL_ID.to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(proposal_id.to_buffer()),
        }),
    )
}

/// The answer the recorded `vote` gives in the vote among the choices of the proposal
/// `proposal_id`; `None` for a vote on anything else.
fn multiple_choice_vote_choice_of(
    vote: &Vote,
    proposal_id: Identifier,
) -> Option<MultipleChoiceVoteChoice> {
    let Vote::ResourceVote(resource_vote) = vote else {
        return None;
    };
    let VotePoll::ContestedDocumentResourceVotePoll(vote_poll) = resource_vote.vote_poll() else {
        return None;
    };
    if *vote_poll != multiple_choice_proposal_vote_poll(proposal_id) {
        return None;
    }
    Some(match resource_vote.resource_vote_choice() {
        ResourceVoteChoice::TowardsIdentity(choice_id) => {
            MultipleChoiceVoteChoice::Choice(choice_id)
        }
        ResourceVoteChoice::Abstain => MultipleChoiceVoteChoice::Abstain,
        ResourceVoteChoice::Lock => MultipleChoiceVoteChoice::NoneOfTheChoices,
    })
}

impl Sdk {
    /// Builds a multiple choice proposal of `owner_id` asking what `input` says (see
    /// [`build_multiple_choice_proposal_document`]). The result is put like any document
    /// create; the owner pays [`Self::multiple_choice_proposal_processing_fee`] on top of the
    /// fees.
    pub async fn build_multiple_choice_proposal(
        &self,
        owner_id: Identifier,
        input: &MultipleChoiceProposalInput,
    ) -> Result<DecisionProposalDocument, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let entropy = Bytes32::new(StdRng::from_os_rng().random());
        build_multiple_choice_proposal_document(&contract, owner_id, input, entropy)
    }

    /// Builds a choice of `owner_id` answering what `input` says (see
    /// [`build_choice_document`]). The result is put like any document create, which states
    /// the contest fund; the owner pays it ([`Self::choice_fund_to_join`]) on top of the fees.
    /// The first choice opens the vote, and later ones join it while its join window is open.
    pub async fn build_choice(
        &self,
        owner_id: Identifier,
        input: &ChoiceInput,
    ) -> Result<DecisionProposalDocument, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let entropy = Bytes32::new(StdRng::from_os_rng().random());
        build_choice_document(&contract, owner_id, input, entropy)
    }

    /// What creating one multiple choice proposal adds to the processing fee of its batch, in
    /// credits. The create does not state it: whoever pays the fees pays it, and an identity
    /// whose balance does not cover it creates no proposal.
    pub async fn multiple_choice_proposal_processing_fee(&self) -> Result<Credits, Error> {
        self.require_protocol_version(
            DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION,
            "the multiple choice proposal processing fee",
        )
        .await?;
        Ok(multiple_choice_proposal_processing_fee(self.version()))
    }

    /// The fund a choice entered now for the proposal `proposal_id` pays, in credits: the
    /// contest's fund, doubled once the vote holds 250 choices and again for every 50 more
    /// (see [`Self::contest_fund_to_join`]). A put of a choice reads it and states it itself.
    pub async fn choice_fund_to_join(&self, proposal_id: Identifier) -> Result<Credits, Error> {
        self.require_protocol_version(
            DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION,
            "the vote among a multiple choice proposal's choices",
        )
        .await?;
        self.contest_fund_to_join_vote_poll(&multiple_choice_proposal_vote_poll(proposal_id))
            .await
    }

    /// The multiple choice proposal with id `proposal_id`, `None` when there is none.
    pub async fn fetch_multiple_choice_proposal(
        &self,
        proposal_id: Identifier,
    ) -> Result<Option<Document>, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let query = DocumentQuery::new(contract, MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME)?
            .with_document_id(&proposal_id);
        Document::fetch(self, query).await
    }

    /// One page of the multiple choice proposals of `owner_id`, by creation time.
    /// [`DocumentsPage::after`] gives the next page.
    pub async fn fetch_multiple_choice_proposals_by_owner(
        &self,
        owner_id: Identifier,
        page: DocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch_many(
            self,
            proposals_by_owner_query(
                contract,
                MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
                owner_id,
                page,
            )?,
        )
        .await
    }

    /// One page of every multiple choice proposal, newest first. [`DocumentsPage::after`]
    /// gives the next page.
    pub async fn fetch_multiple_choice_proposals_by_created_at(
        &self,
        page: DocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch_many(
            self,
            proposals_by_created_at_query(
                contract,
                MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
                page,
            )?,
        )
        .await
    }

    /// The vote among the choices of the proposal `proposal_id`: every choice with its tally
    /// while the vote runs, and its outcome with every choice's final tally once it ended. A
    /// proposal no choice was entered for has an empty vote. The choices are read in pages of
    /// 100, so a vote of `n` choices costs `n / 100 + 1` proved queries.
    pub async fn fetch_multiple_choice_proposal_vote(
        &self,
        proposal_id: Identifier,
    ) -> Result<MultipleChoiceProposalVote, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let contenders = self
            .fetch_contested_resource_vote_state(
                &multiple_choice_proposal_vote_poll(proposal_id),
                None,
            )
            .await?;
        MultipleChoiceProposalVote::from_contenders(&contract, contenders, self.version())
    }

    /// The choice the finished vote of the proposal `proposal_id` kept, stored as an ordinary
    /// document under the identity that entered it. `None` while the vote runs, for a vote
    /// that kept none of the choices, and for a proposal no choice was entered for.
    pub async fn fetch_kept_choice(
        &self,
        proposal_id: Identifier,
    ) -> Result<Option<Document>, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch(self, kept_choice_query(contract, proposal_id)?).await
    }

    /// The answer the masternode `pro_tx_hash` gave in the vote among the choices of the
    /// proposal `proposal_id`. `None` when the masternode has not voted in it, and once the
    /// vote ended.
    pub async fn fetch_masternode_multiple_choice_vote(
        &self,
        pro_tx_hash: Identifier,
        proposal_id: Identifier,
    ) -> Result<Option<MultipleChoiceVoteChoice>, Error> {
        self.require_protocol_version(
            DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION,
            "the vote among a multiple choice proposal's choices",
        )
        .await?;
        let vote_poll_id = multiple_choice_proposal_vote_poll(proposal_id).unique_id()?;
        let vote = Vote::fetch(self, VoteQuery::new(pro_tx_hash, vote_poll_id)).await?;
        // The query starts at the poll's id, so it may answer with the masternode's vote on a
        // later poll
        Ok(vote.and_then(|vote| multiple_choice_vote_choice_of(&vote, proposal_id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use dpp::decision_proposal::multiple_choice::CHOICE_CONTEST_INDEX_NAME;
    use dpp::decision_proposal::DECISION_PROPOSALS_CONTRACT_ID;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::INITIAL_REVISION;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::voting::contender_structs::{
        ContenderWithSerializedDocument, ContenderWithSerializedDocumentV0,
    };
    use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
    use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use drive::config::DEFAULT_QUERY_LIMIT;
    use drive::query::vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
    };
    use drive::query::DriveDocumentQuery;

    fn proposals_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(
                SystemDataContract::DecisionProposals,
                PlatformVersion::latest(),
            )
            .expect("the decision proposals contract loads at the latest version"),
        )
    }

    fn description_document() -> DescriptionDocument {
        DescriptionDocument {
            id: Identifier::from([5; 32]),
            contract_id: Identifier::from([6; 32]),
            document_type: "post".to_string(),
        }
    }

    fn input() -> MultipleChoiceProposalInput {
        MultipleChoiceProposalInput {
            title: "Which block size?".to_string(),
            description_document: Some(description_document()),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            open: true,
        }
    }

    /// `document` with the system fields the platform fills in when it stores the create.
    fn stored(document: &Document) -> Document {
        let mut document = document.clone();
        document.set_created_at(Some(1_700_000_000_000));
        document.set_updated_at(Some(1_700_000_000_000));
        document
    }

    fn built_proposal(input: &MultipleChoiceProposalInput) -> Document {
        build_multiple_choice_proposal_document(
            &proposals_contract(),
            Identifier::from([3; 32]),
            input,
            Bytes32::new([9; 32]),
        )
        .expect("builds")
        .document
    }

    fn built_choice(description: Option<&str>) -> Document {
        build_choice_document(
            &proposals_contract(),
            Identifier::from([4; 32]),
            &ChoiceInput {
                proposal_id: Identifier::from([7; 32]),
                title: "Double it".to_string(),
                description: description.map(str::to_string),
            },
            Bytes32::new([8; 32]),
        )
        .expect("builds")
        .document
    }

    #[test]
    fn should_build_a_multiple_choice_proposal_the_schema_accepts() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let described = [
            input(),
            MultipleChoiceProposalInput {
                url: None,
                ..input()
            },
            MultipleChoiceProposalInput {
                description_document: None,
                open: false,
                ..input()
            },
        ];
        for input in described {
            let built = build_multiple_choice_proposal_document(
                &contract,
                Identifier::from([3; 32]),
                &input,
                Bytes32::new([9; 32]),
            )
            .expect("builds");
            assert_eq!(built.entropy, Bytes32::new([9; 32]));
            assert_eq!(built.document.revision(), Some(INITIAL_REVISION));
            assert_eq!(
                built.document.id(),
                Document::generate_document_id_v0(
                    &DECISION_PROPOSALS_CONTRACT_ID,
                    &Identifier::from([3; 32]),
                    MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
                    &[9; 32],
                )
            );
            let document = stored(&built.document);
            let validation = contract
                .validate_document(
                    MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
                    &document,
                    platform_version,
                )
                .expect("validates");
            assert!(validation.is_valid(), "{input:?}: {:?}", validation.errors);

            // A multiple choice proposal opens no poll, so its create states no fund
            let document_type = contract
                .document_type_for_name(MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME)
                .expect("the multiple choice proposal type");
            assert_eq!(
                document_type
                    .prefunded_voting_balance_for_document(&built.document, platform_version)
                    .expect("active"),
                None
            );

            let proposal = MultipleChoiceProposal::from_document(document.clone()).expect("reads");
            assert_eq!(
                proposal,
                MultipleChoiceProposal {
                    document,
                    title: input.title,
                    description_document: input.description_document,
                    url: input.url,
                    open: input.open,
                }
            );
            assert_eq!(proposal.owner_id(), Identifier::from([3; 32]));
        }
    }

    /// The builder writes what it is given: which descriptions Platform takes is the
    /// contract's to say, so a proposal described nowhere is built and left to it.
    #[test]
    fn should_leave_out_the_description_document_and_url_not_given() {
        let document = built_proposal(&MultipleChoiceProposalInput {
            description_document: None,
            url: None,
            ..input()
        });
        for property in [
            proposal_properties::DESCRIPTION_ID,
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
            proposal_properties::URL,
        ] {
            assert!(document.get(property).is_none(), "{property}");
        }

        let mut no_document_type = built_proposal(&input());
        no_document_type.remove(proposal_properties::DESCRIPTION_DOCUMENT_TYPE);
        assert!(MultipleChoiceProposal::from_document(no_document_type).is_err());
    }

    fn edit_proposal(
        proposal: &Document,
        edit: MultipleChoiceProposalEdit,
    ) -> Result<Document, Error> {
        build_multiple_choice_proposal_edit(
            &DECISION_PROPOSALS_CONTRACT_ID,
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            proposal,
            &edit,
        )
    }

    #[test]
    fn should_edit_the_title_url_and_description_document_and_never_whether_it_is_open() {
        let proposal = stored(&built_proposal(&input()));

        let retitled = edit_proposal(
            &proposal,
            MultipleChoiceProposalEdit {
                title: Some("Which block size, then?".to_string()),
                ..Default::default()
            },
        )
        .expect("edits");
        assert_eq!(retitled.id(), proposal.id());
        assert_eq!(retitled.revision(), Some(INITIAL_REVISION + 1));
        let read = MultipleChoiceProposal::from_document(retitled).expect("reads");
        assert_eq!(read.title, "Which block size, then?");
        assert_eq!(read.description_document, Some(description_document()));
        assert_eq!(read.url, input().url);
        assert!(read.open);

        let elsewhere = DescriptionDocument {
            id: Identifier::from([1; 32]),
            contract_id: Identifier::from([2; 32]),
            document_type: "note".to_string(),
        };
        let pointed_elsewhere = edit_proposal(
            &proposal,
            MultipleChoiceProposalEdit {
                description_document: Some(Some(elsewhere.clone())),
                url: Some(None),
                ..Default::default()
            },
        )
        .expect("edits");
        let read = MultipleChoiceProposal::from_document(pointed_elsewhere).expect("reads");
        assert_eq!(read.description_document, Some(elsewhere));
        assert_eq!(read.url, None);

        // All three properties naming the description document go together
        let at_its_url = edit_proposal(
            &proposal,
            MultipleChoiceProposalEdit {
                description_document: Some(None),
                ..Default::default()
            },
        )
        .expect("edits");
        for property in [
            proposal_properties::DESCRIPTION_ID,
            proposal_properties::DESCRIPTION_CONTRACT_ID,
            proposal_properties::DESCRIPTION_DOCUMENT_TYPE,
        ] {
            assert!(at_its_url.get(property).is_none(), "{property}");
        }
        let read = MultipleChoiceProposal::from_document(at_its_url).expect("reads");
        assert_eq!(read.description_document, None);
        assert_eq!(read.url, input().url);
    }

    #[test]
    fn should_refuse_an_edit_that_changes_nothing_or_is_not_of_a_multiple_choice_proposal() {
        let proposal = stored(&built_proposal(&input()));
        let unchanged = [
            MultipleChoiceProposalEdit::default(),
            MultipleChoiceProposalEdit {
                title: Some(input().title),
                description_document: Some(Some(description_document())),
                url: Some(input().url),
            },
        ];
        for edit in unchanged {
            let error = edit_proposal(&proposal, edit.clone()).expect_err("changes nothing");
            assert!(error.to_string().contains("changes nothing"), "{edit:?}");
        }

        let edit = MultipleChoiceProposalEdit {
            title: Some("Another question?".to_string()),
            ..Default::default()
        };
        for (data_contract_id, document_type_name) in [
            (
                Identifier::from([1; 32]),
                MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            ),
            (DECISION_PROPOSALS_CONTRACT_ID, "proposal"),
            (DECISION_PROPOSALS_CONTRACT_ID, CHOICE_DOCUMENT_TYPE_NAME),
        ] {
            let error = build_multiple_choice_proposal_edit(
                &data_contract_id,
                document_type_name,
                &proposal,
                &edit,
            )
            .expect_err("not a multiple choice proposal");
            assert!(
                error
                    .to_string()
                    .contains("is not a multiple choice proposal"),
                "{document_type_name} of {data_contract_id}: {error}"
            );
        }
    }

    #[test]
    fn should_build_a_choice_the_schema_accepts_and_the_vote_of_its_proposal_takes() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let document_type = contract
            .document_type_for_name(CHOICE_DOCUMENT_TYPE_NAME)
            .expect("the choice type");
        for description in [Some("Twice the size it is today."), None] {
            let built = built_choice(description);
            assert_eq!(built.revision(), None);
            let mut document = built.clone();
            document.set_created_at(Some(1_700_000_000_000));
            let validation = contract
                .validate_document(CHOICE_DOCUMENT_TYPE_NAME, &document, platform_version)
                .expect("validates");
            assert!(validation.is_valid(), "{:?}", validation.errors);

            // The choice joins the vote its proposal's id names, and its create states the
            // fund of that contest under the contested index
            assert_eq!(
                document_type
                    .contested_vote_poll_for_document(&built, platform_version)
                    .expect("active"),
                Some(multiple_choice_proposal_vote_poll(Identifier::from(
                    [7; 32]
                )))
            );
            let (fund_name, fund) = document_type
                .prefunded_voting_balance_for_document(&built, platform_version)
                .expect("active")
                .expect("a contested create states a fund");
            assert_eq!(fund_name, CHOICE_CONTEST_INDEX_NAME);
            assert_eq!(
                fund,
                multiple_choice_proposal_vote_poll(Identifier::from([7; 32]))
                    .required_vote_resolution_fund_to_join(0, platform_version)
            );

            let choice = Choice::from_document(document.clone()).expect("reads");
            assert_eq!(
                choice,
                Choice {
                    document,
                    proposal_id: Identifier::from([7; 32]),
                    title: "Double it".to_string(),
                    description: description.map(str::to_string),
                }
            );
            assert_eq!(choice.owner_id(), Identifier::from([4; 32]));
        }
    }

    /// A contender of the vote with `vote_tally` votes: the choice `document`, when the vote
    /// still holds it.
    fn contender(
        choice_id: Identifier,
        document: Option<&Document>,
        vote_tally: u32,
    ) -> (Identifier, ContenderWithSerializedDocument) {
        let contract = proposals_contract();
        let document_type = contract
            .document_type_for_name(CHOICE_DOCUMENT_TYPE_NAME)
            .expect("the choice type");
        let serialized_document = document.map(|document| {
            document
                .serialize(document_type, &contract, PlatformVersion::latest())
                .expect("serializes")
        });
        (
            choice_id,
            ContenderWithSerializedDocumentV0 {
                identity_id: choice_id,
                serialized_document,
                vote_tally: Some(vote_tally),
            }
            .into(),
        )
    }

    #[test]
    fn should_read_a_running_vote_with_its_choices_and_a_finished_one_with_its_tallies() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let mut double = built_choice(Some("Twice the size it is today."));
        double.set_created_at(Some(1_700_000_000_000));
        let keep_id = Identifier::from([2; 32]);

        let running = MultipleChoiceProposalVote::from_contenders(
            &contract,
            Contenders {
                winner: None,
                contenders: BTreeMap::from([contender(double.id(), Some(&double), 12)]),
                abstain_vote_tally: Some(3),
                lock_vote_tally: Some(1),
            },
            platform_version,
        )
        .expect("reads");
        assert_eq!(
            running,
            MultipleChoiceProposalVote {
                choices: BTreeMap::from([(
                    double.id(),
                    ChoiceInVote {
                        choice: Some(Choice::from_document(double.clone()).expect("reads")),
                        vote_tally: Some(12),
                    }
                )]),
                abstain_vote_tally: Some(3),
                none_of_the_choices_vote_tally: Some(1),
                winner: None,
            }
        );
        assert_eq!(running.kept_choice_id(), None);

        // Once the vote ended its record names the choices by id alone
        let finished = MultipleChoiceProposalVote::from_contenders(
            &contract,
            Contenders {
                winner: Some((
                    ContestedDocumentVotePollWinnerInfo::WonByIdentity(double.id()),
                    BlockInfo::default(),
                )),
                contenders: BTreeMap::from([
                    contender(double.id(), None, 12),
                    contender(keep_id, None, 4),
                ]),
                abstain_vote_tally: Some(3),
                lock_vote_tally: Some(1),
            },
            platform_version,
        )
        .expect("reads");
        assert_eq!(finished.kept_choice_id(), Some(double.id()));
        assert_eq!(
            finished.choices[&keep_id],
            ChoiceInVote {
                choice: None,
                vote_tally: Some(4),
            }
        );

        let none_kept = MultipleChoiceProposalVote {
            winner: Some((
                ContestedDocumentVotePollWinnerInfo::Locked,
                BlockInfo::default(),
            )),
            ..Default::default()
        };
        assert_eq!(none_kept.kept_choice_id(), None);
    }

    /// The index `query` is served through, and that a proof of it can be verified: the path
    /// query the verifier rebuilds constructs.
    fn served_through(query: &DocumentQuery) -> String {
        let platform_version = PlatformVersion::latest();
        let drive_query = DriveDocumentQuery::try_from(query).expect("converts");
        drive_query
            .construct_path_query(None, platform_version)
            .expect("a provable path query");
        drive_query
            .find_best_index(platform_version)
            .expect("an index serves it")
            .name
            .clone()
    }

    #[test]
    fn should_serve_every_listing_through_the_index_the_schema_declares_for_it() {
        let contract = proposals_contract();
        let page = DocumentsPage {
            limit: Some(20),
            start_after: Some(Identifier::from([5; 32])),
        };
        let by_owner = proposals_by_owner_query(
            contract.clone(),
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            Identifier::from([3; 32]),
            page,
        )
        .expect("builds");
        assert_eq!(served_through(&by_owner), "byOwner");
        assert_eq!(by_owner.limit, 20);
        let by_created_at = proposals_by_created_at_query(
            contract.clone(),
            MULTIPLE_CHOICE_PROPOSAL_DOCUMENT_TYPE_NAME,
            DocumentsPage::default(),
        )
        .expect("builds");
        assert_eq!(served_through(&by_created_at), "byCreatedAt");
        assert_eq!(by_created_at.limit, 100);
        assert_eq!(
            served_through(
                &kept_choice_query(contract, Identifier::from([7; 32])).expect("builds")
            ),
            CHOICE_CONTEST_INDEX_NAME
        );
    }

    #[test]
    fn should_read_a_masternode_answer_only_from_a_vote_on_the_proposal() {
        let proposal_id = Identifier::from([7; 32]);
        let choice_id = Identifier::from([9; 32]);
        for vote_choice in [
            MultipleChoiceVoteChoice::Choice(choice_id),
            MultipleChoiceVoteChoice::Abstain,
            MultipleChoiceVoteChoice::NoneOfTheChoices,
        ] {
            let vote = multiple_choice_proposal_vote(proposal_id, vote_choice);
            assert_eq!(
                multiple_choice_vote_choice_of(&vote, proposal_id),
                Some(vote_choice)
            );
            // The vote query starts at the poll's id and may answer with a later poll's vote
            assert_eq!(
                multiple_choice_vote_choice_of(&vote, Identifier::from([8; 32])),
                None
            );
        }
        let on_another_contest = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
            vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                ContestedDocumentResourceVotePoll {
                    index_name: "parentNameAndLabel".to_string(),
                    ..multiple_choice_proposal_vote_poll(proposal_id)
                },
            ),
            resource_vote_choice: ResourceVoteChoice::Abstain,
        }));
        assert_eq!(
            multiple_choice_vote_choice_of(&on_another_contest, proposal_id),
            None
        );
    }

    /// The fund of a choice and the vote are read from the contest the proposal's id names.
    #[tokio::test]
    async fn should_read_the_choice_fund_and_the_vote_from_the_contest_of_the_proposal() {
        let mut sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        let proposal_id = Identifier::from([7; 32]);
        let mut double = built_choice(Some("Twice the size it is today."));
        double.set_created_at(Some(1_700_000_000_000));
        let state = || Contenders {
            winner: None,
            contenders: BTreeMap::from([contender(double.id(), Some(&double), 12)]),
            abstain_vote_tally: Some(3),
            lock_vote_tally: Some(1),
        };
        let contest_query = |result_type, with_other_tallies| ContestedDocumentVotePollDriveQuery {
            vote_poll: multiple_choice_proposal_vote_poll(proposal_id),
            result_type,
            offset: None,
            limit: Some(DEFAULT_QUERY_LIMIT),
            start_at: None,
            allow_include_locked_and_abstaining_vote_tally: with_other_tallies,
        };
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                contest_query(
                    ContestedDocumentVotePollDriveQueryResultType::VoteTally,
                    false,
                ),
                Some(state()),
            )
            .await
            .expect("registers the tallies");
        sdk.mock()
            .expect_fetch_many::<_, ContenderWithSerializedDocument, _, Contenders>(
                contest_query(
                    ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                    true,
                ),
                Some(state()),
            )
            .await
            .expect("registers the vote state");
        // One choice entered: the next pays the contest's fund, not yet doubled
        assert_eq!(
            sdk.choice_fund_to_join(proposal_id).await.expect("reads"),
            10_000_000_000
        );
        // The mock SDK holds no system contract, so the vote is read as
        // `fetch_multiple_choice_proposal_vote` reads it, with the contract given
        let contenders = sdk
            .fetch_contested_resource_vote_state(
                &multiple_choice_proposal_vote_poll(proposal_id),
                None,
            )
            .await
            .expect("reads");
        let vote = MultipleChoiceProposalVote::from_contenders(
            &proposals_contract(),
            contenders,
            sdk.version(),
        )
        .expect("reads");
        assert_eq!(vote.choices.len(), 1);
        assert_eq!(
            vote.choices[&double.id()],
            ChoiceInVote {
                choice: Some(Choice::from_document(double.clone()).expect("reads")),
                vote_tally: Some(12),
            }
        );
        assert_eq!(vote.none_of_the_choices_vote_tally, Some(1));
    }

    #[tokio::test]
    async fn should_price_a_multiple_choice_proposal_by_the_fee_schedule() {
        let sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        assert_eq!(
            sdk.multiple_choice_proposal_processing_fee()
                .await
                .expect("active"),
            20_000_000_000
        );
    }
}
