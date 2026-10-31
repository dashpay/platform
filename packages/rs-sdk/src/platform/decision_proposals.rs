//! Decision proposals (protocol version 15): yes/no questions any identity puts to the
//! masternodes, as `proposal` documents of the decision proposals system contract. Creating a
//! proposal opens its yes/no vote poll, funded by the decision proposal fund (0.5 Dash) its
//! owner pays. Nothing acts on a result; it stays in the poll's stored info.
//!
//! * [`Sdk::build_decision_proposal`] builds a proposal, put like any document create with
//!   [`PutDocument`](crate::platform::transition::put_document::PutDocument). The create states
//!   the fund itself. Consensus refuses a rule outside the contract's limits and an end date
//!   less than a week or more than four weeks after the block that creates the proposal.
//! * [`build_decision_proposal_edit`] gives a proposal new text, put as a replace. Only the
//!   title, description and url change, and only for a day after the proposal is created.
//! * [`Sdk::fetch_decision_proposal`] reads a proposal by id, and three listings read them a
//!   page at a time through the contract's indexes:
//!   [`Sdk::fetch_decision_proposals_by_end_date`] closing soonest first,
//!   [`Sdk::fetch_decision_proposals_by_owner`] in filing order, and
//!   [`Sdk::fetch_decision_proposals_by_created_at`] newest first.
//!   [`DecisionProposal::from_document`] reads a proposal's properties.
//! * [`Sdk::decision_proposal_vote`] builds a masternode's answer on a proposal, put with
//!   [`PutVote`](crate::platform::transition::vote::PutVote).
//!
//! A proposal's poll is named by its purpose, [`YesNoVotePollPurpose::DecisionProposal`] with
//! the proposal's id ([`Sdk::decision_proposal_vote_poll_id`] gives its id): pass it to
//! [`YesNoVotePollStateQuery::new`](crate::platform::yes_no_vote_polls::YesNoVotePollStateQuery::new)
//! for the poll's status and tallies, and to
//! [`MasternodeYesNoVoteQuery::new`](crate::platform::yes_no_vote_polls::MasternodeYesNoVoteQuery::new)
//! for a masternode's answer.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::page::DocumentsPage;
use crate::platform::{DataContract, Document, Fetch, FetchMany, Identifier};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_documents_request::get_documents_request_v0::Start;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::decision_proposal::{
    properties, DECISION_PROPOSALS_CONTRACT_ID, PROPOSAL_DOCUMENT_TYPE_NAME,
};
use dpp::document::{DocumentV0, DocumentV0Getters, DocumentV0Setters, INITIAL_REVISION};
use dpp::platform_value::btreemap_extensions::BTreeValueMapHelper;
use dpp::platform_value::string_encoding::Encoding;
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::TimestampMillis;
use dpp::secp256k1::rand::rngs::StdRng;
use dpp::secp256k1::rand::{Rng, SeedableRng};
use dpp::version::feature_initial_protocol_versions::DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION;
use dpp::voting::vote_choices::yes_no_abstain_vote_choice::YesNoAbstainVoteChoice;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePollPurpose;
use dpp::voting::votes::Vote;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use drive_proof_verifier::types::Documents;
use std::collections::BTreeMap;
use std::sync::Arc;

/// What a new proposal asks and the rule it is decided by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionProposalInput {
    /// The question, in one line: 1 to 128 characters.
    pub title: String,
    /// What a yes and a no mean: 1 to 4096 characters.
    pub description: String,
    /// Where the proposal is discussed, a URI of at most 512 characters.
    pub url: Option<String>,
    /// The proposal passes when the yes voting power is strictly more than this percent of the
    /// yes plus no voting power: 50 to 80.
    pub pass_percent: u8,
    /// The least yes plus no voting power the proposal needs, as a percent of the masternode
    /// list's total voting power when it closes, rounded up: 0 to 50.
    pub minimum_turnout_percent: u8,
    /// When voting closes, in milliseconds since the Unix epoch: from a week to four weeks
    /// after the block that creates the proposal, which is a little later than now.
    pub end_date: TimestampMillis,
}

/// New text for a proposal. A field left `None` keeps its value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecisionProposalEdit {
    /// The new title.
    pub title: Option<String>,
    /// The new description.
    pub description: Option<String>,
    /// The new url: `Some(Some(url))` sets it, `Some(None)` removes it.
    pub url: Option<Option<String>>,
}

/// A proposal ready to be put with
/// [`PutDocument`](crate::platform::transition::put_document::PutDocument): the document and
/// the entropy its id derives from, which the put must reuse.
#[derive(Debug, Clone)]
pub struct DecisionProposalDocument {
    /// The document. Its id is a placeholder until it is put: from protocol version 14 the id
    /// also commits to the identity contract nonce of the create transition. The poll is named
    /// by the id of the document the put returns.
    pub document: Document,
    /// The entropy the document id derives from.
    pub entropy: Bytes32,
}

/// A proposal: its document and the properties read out of it.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionProposal {
    /// The `proposal` document.
    pub document: Document,
    /// The question, in one line.
    pub title: String,
    /// What a yes and a no mean.
    pub description: String,
    /// Where the proposal is discussed.
    pub url: Option<String>,
    /// The proposal passes when yes is strictly more than this percent of yes plus no.
    pub pass_percent: u8,
    /// The least yes plus no voting power the proposal needs, as a percent of the network's.
    pub minimum_turnout_percent: u8,
    /// When voting closes, in milliseconds since the Unix epoch.
    pub end_date: TimestampMillis,
}

impl DecisionProposal {
    /// Reads a proposal out of a `proposal` document.
    pub fn from_document(document: Document) -> Result<Self, Error> {
        let malformed = |error: dpp::platform_value::Error| {
            Error::Generic(format!("proposal {} is malformed: {error}", document.id()))
        };
        let properties = document.properties();
        Ok(Self {
            title: properties
                .get_string(properties::TITLE)
                .map_err(malformed)?,
            description: properties
                .get_string(properties::DESCRIPTION)
                .map_err(malformed)?,
            url: properties
                .get_optional_string(properties::URL)
                .map_err(malformed)?,
            pass_percent: properties
                .get_integer(properties::PASS_PERCENT)
                .map_err(malformed)?,
            minimum_turnout_percent: properties
                .get_integer(properties::MINIMUM_TURNOUT_PERCENT)
                .map_err(malformed)?,
            end_date: properties
                .get_integer(properties::END_DATE)
                .map_err(malformed)?,
            document,
        })
    }

    /// The proposal's id.
    pub fn id(&self) -> Identifier {
        self.document.id()
    }

    /// The proposer, who paid the fund.
    pub fn owner_id(&self) -> Identifier {
        self.document.owner_id()
    }

    /// The purpose of the proposal's poll, which names it in the poll queries.
    pub fn vote_poll_purpose(&self) -> YesNoVotePollPurpose {
        YesNoVotePollPurpose::DecisionProposal {
            document_id: self.id(),
        }
    }
}

/// The end dates a listing by end date covers: from `from` (included) to `to` (left out). An
/// open bound is `None`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DecisionProposalEndDates {
    /// The earliest end date listed, included.
    pub from: Option<TimestampMillis>,
    /// The end date the listing stops before.
    pub to: Option<TimestampMillis>,
}

/// Builds a proposal of `owner_id` asking what `input` says, in `contract`, the decision
/// proposals contract, with its id derived from `entropy`.
pub fn build_decision_proposal_document(
    contract: &DataContract,
    owner_id: Identifier,
    input: &DecisionProposalInput,
    entropy: Bytes32,
) -> Result<DecisionProposalDocument, Error> {
    let document_type = contract
        .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
        .map_err(|e| Error::Protocol(e.into()))?;
    let mut properties = BTreeMap::from([
        (
            properties::TITLE.to_string(),
            Value::Text(input.title.clone()),
        ),
        (
            properties::DESCRIPTION.to_string(),
            Value::Text(input.description.clone()),
        ),
        (
            properties::PASS_PERCENT.to_string(),
            Value::U8(input.pass_percent),
        ),
        (
            properties::MINIMUM_TURNOUT_PERCENT.to_string(),
            Value::U8(input.minimum_turnout_percent),
        ),
        (properties::END_DATE.to_string(), Value::U64(input.end_date)),
    ]);
    if let Some(url) = &input.url {
        properties.insert(properties::URL.to_string(), Value::Text(url.clone()));
    }
    let document = DocumentV0 {
        id: Document::generate_document_id_v0(
            &contract.id(),
            &owner_id,
            PROPOSAL_DOCUMENT_TYPE_NAME,
            entropy.as_slice(),
        ),
        owner_id,
        properties,
        revision: document_type
            .requires_revision()
            .then_some(INITIAL_REVISION),
        ..Default::default()
    }
    .into();
    Ok(DecisionProposalDocument { document, entropy })
}

/// `proposal` with the new text `edit` gives it and its next revision, ready to be put as a
/// replace. Consensus refuses the replace more than a day after the proposal was created (or a
/// fifth of the time it runs, if that is shorter), and refuses any change to its rule or end
/// date.
pub fn build_decision_proposal_edit(
    proposal: &Document,
    edit: &DecisionProposalEdit,
) -> Result<Document, Error> {
    if edit == &DecisionProposalEdit::default() {
        return Err(Error::Generic(format!(
            "the edit of proposal {} changes nothing",
            proposal.id()
        )));
    }
    let revision = proposal
        .revision()
        .ok_or_else(|| Error::Generic(format!("proposal {} carries no revision", proposal.id())))?;
    let mut document = proposal.clone();
    document.set_revision(Some(revision + 1));
    if let Some(title) = &edit.title {
        document.set(properties::TITLE, Value::Text(title.clone()));
    }
    if let Some(description) = &edit.description {
        document.set(properties::DESCRIPTION, Value::Text(description.clone()));
    }
    match &edit.url {
        Some(Some(url)) => document.set(properties::URL, Value::Text(url.clone())),
        Some(None) => {
            document.remove(properties::URL);
        }
        None => {}
    }
    Ok(document)
}

/// The query for the proposals whose end date is within `end_dates`, by end date, then by
/// creation time (`byEndDate`), one page of them.
fn proposals_by_end_date_query(
    contract: Arc<DataContract>,
    end_dates: DecisionProposalEndDates,
    page: DocumentsPage,
) -> Result<DocumentQuery, Error> {
    let end_date_clause = |operator, value| WhereClause {
        field: properties::END_DATE.to_string(),
        operator,
        value,
    };
    let range = match (end_dates.from, end_dates.to) {
        (Some(from), Some(to)) => Some(end_date_clause(
            WhereOperator::BetweenExcludeRight,
            Value::Array(vec![Value::U64(from), Value::U64(to)]),
        )),
        (Some(from), None) => Some(end_date_clause(
            WhereOperator::GreaterThanOrEquals,
            Value::U64(from),
        )),
        (None, Some(to)) => Some(end_date_clause(WhereOperator::LessThan, Value::U64(to))),
        (None, None) => None,
    };
    let mut query = DocumentQuery::new(contract, PROPOSAL_DOCUMENT_TYPE_NAME)?;
    if let Some(range) = range {
        query = query.with_where(range);
    }
    paged(
        query.with_order_by(OrderClause {
            field: properties::END_DATE.to_string(),
            ascending: true,
        }),
        page,
    )
}

/// The query for the proposals of `owner_id`, by creation time (`byOwner`), one page of them.
fn proposals_by_owner_query(
    contract: Arc<DataContract>,
    owner_id: Identifier,
    page: DocumentsPage,
) -> Result<DocumentQuery, Error> {
    let query = DocumentQuery::new(contract, PROPOSAL_DOCUMENT_TYPE_NAME)?
        .with_where(WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(owner_id.to_buffer()),
        })
        // The order clause pins the index: a bare equality on the first property of a
        // two-property index is proven absent instead of served.
        .with_order_by(OrderClause {
            field: "$createdAt".to_string(),
            ascending: true,
        });
    paged(query, page)
}

/// The query for every proposal, newest first (`byCreatedAt`), one page of them.
fn proposals_by_created_at_query(
    contract: Arc<DataContract>,
    page: DocumentsPage,
) -> Result<DocumentQuery, Error> {
    let query =
        DocumentQuery::new(contract, PROPOSAL_DOCUMENT_TYPE_NAME)?.with_order_by(OrderClause {
            field: "$createdAt".to_string(),
            ascending: false,
        });
    paged(query, page)
}

/// `query` limited to `page`.
fn paged(query: DocumentQuery, page: DocumentsPage) -> Result<DocumentQuery, Error> {
    let mut query = query.with_limit(page.page_size());
    query.start = page
        .start_after
        .map(|id| Start::StartAfter(id.to_buffer().to_vec()));
    Ok(query)
}

impl Sdk {
    /// The decision proposals system contract: from the context provider when it holds it,
    /// fetched otherwise. The contract exists from protocol version 15; below 15 the SDK
    /// first learns the network's version, and a network still below 15 has no such contract,
    /// which is an error.
    pub async fn fetch_decision_proposals_contract(&self) -> Result<Arc<DataContract>, Error> {
        self.require_protocol_version(
            DECISION_PROPOSALS_CONTRACT_INITIAL_PROTOCOL_VERSION,
            "the decision proposals contract",
        )
        .await?;
        self.fetch_system_data_contract(DECISION_PROPOSALS_CONTRACT_ID)
            .await?
            .ok_or_else(|| {
                Error::MissingDependency(
                    "decision proposals contract".to_string(),
                    DECISION_PROPOSALS_CONTRACT_ID.to_string(Encoding::Base58),
                )
            })
    }

    /// Builds a proposal of `owner_id` asking what `input` says (see
    /// [`build_decision_proposal_document`]). The result is put like any document create; the
    /// owner pays the decision proposal fund on top of the fees.
    pub async fn build_decision_proposal(
        &self,
        owner_id: Identifier,
        input: &DecisionProposalInput,
    ) -> Result<DecisionProposalDocument, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let entropy = Bytes32::new(StdRng::from_os_rng().random());
        build_decision_proposal_document(&contract, owner_id, input, entropy)
    }

    /// The proposal with id `proposal_id`, `None` when there is none.
    pub async fn fetch_decision_proposal(
        &self,
        proposal_id: Identifier,
    ) -> Result<Option<Document>, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        let query = DocumentQuery::new(contract, PROPOSAL_DOCUMENT_TYPE_NAME)?
            .with_document_id(&proposal_id);
        Document::fetch(self, query).await
    }

    /// One page of the proposals whose end date is within `end_dates`, by end date, then by
    /// creation time. The open ones are those ending from now on. [`DocumentsPage::after`]
    /// gives the next page.
    pub async fn fetch_decision_proposals_by_end_date(
        &self,
        end_dates: DecisionProposalEndDates,
        page: DocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch_many(
            self,
            proposals_by_end_date_query(contract, end_dates, page)?,
        )
        .await
    }

    /// One page of the proposals of `owner_id`, by creation time. [`DocumentsPage::after`]
    /// gives the next page.
    pub async fn fetch_decision_proposals_by_owner(
        &self,
        owner_id: Identifier,
        page: DocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch_many(self, proposals_by_owner_query(contract, owner_id, page)?).await
    }

    /// One page of every proposal, newest first. [`DocumentsPage::after`] gives the next page.
    pub async fn fetch_decision_proposals_by_created_at(
        &self,
        page: DocumentsPage,
    ) -> Result<Documents, Error> {
        let contract = self.fetch_decision_proposals_contract().await?;
        Document::fetch_many(self, proposals_by_created_at_query(contract, page)?).await
    }

    /// A masternode's answer `vote_choice` on the proposal `proposal_id`, to put with
    /// [`PutVote`](crate::platform::transition::vote::PutVote). The vote names the proposal's
    /// poll by its id. A masternode may vote 5 times on a poll, but not give the answer it
    /// already gave; polls exist from protocol version 15.
    pub async fn decision_proposal_vote(
        &self,
        proposal_id: Identifier,
        vote_choice: YesNoAbstainVoteChoice,
    ) -> Result<Vote, Error> {
        self.yes_no_vote(
            &YesNoVotePollPurpose::DecisionProposal {
                document_id: proposal_id,
            },
            vote_choice,
        )
        .await
    }

    /// The id of the yes/no poll the proposal `proposal_id` opens, which the vote polls by end
    /// date listing and the poll queries name it by.
    pub async fn decision_proposal_vote_poll_id(
        &self,
        proposal_id: Identifier,
    ) -> Result<Identifier, Error> {
        self.yes_no_vote_poll_id(&YesNoVotePollPurpose::DecisionProposal {
            document_id: proposal_id,
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use dpp::decision_proposal::{
        decision_proposal_vote_poll_id, DecisionProposalVotePoll, DECISION_PROPOSAL_FUND_NAME,
    };
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;
    use dpp::voting::votes::yes_no_vote::v0::YesNoVoteV0;
    use dpp::voting::votes::yes_no_vote::YesNoVote;
    use drive::query::DriveDocumentQuery;

    const DAY_MS: u64 = 86_400_000;

    fn proposals_contract() -> Arc<DataContract> {
        Arc::new(
            load_system_data_contract(
                SystemDataContract::DecisionProposals,
                PlatformVersion::latest(),
            )
            .expect("the decision proposals contract loads at the latest version"),
        )
    }

    fn input() -> DecisionProposalInput {
        DecisionProposalInput {
            title: "Raise the block size?".to_string(),
            description: "Yes doubles it; no keeps it.".to_string(),
            url: Some("https://forum.dash.org/t/block-size".to_string()),
            pass_percent: 60,
            minimum_turnout_percent: 10,
            end_date: 1_700_000_000_000 + 14 * DAY_MS,
        }
    }

    /// The document `build_decision_proposal_document` builds, with the system fields the
    /// platform fills in when it stores the create.
    fn stored(document: &Document) -> Document {
        let mut document = document.clone();
        document.set_created_at(Some(1_700_000_000_000));
        document.set_updated_at(Some(1_700_000_000_000));
        document
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
    fn should_build_a_proposal_the_schema_accepts_and_the_poll_reads() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let owner_id = Identifier::from([3; 32]);
        let entropy = Bytes32::new([9; 32]);
        let built = build_decision_proposal_document(&contract, owner_id, &input(), entropy)
            .expect("builds");

        assert_eq!(built.entropy, entropy);
        assert_eq!(built.document.owner_id(), owner_id);
        assert_eq!(built.document.revision(), Some(INITIAL_REVISION));
        assert_eq!(
            built.document.id(),
            Document::generate_document_id_v0(
                &DECISION_PROPOSALS_CONTRACT_ID,
                &owner_id,
                PROPOSAL_DOCUMENT_TYPE_NAME,
                entropy.as_slice(),
            )
        );
        let document = stored(&built.document);
        let validation = contract
            .validate_document(PROPOSAL_DOCUMENT_TYPE_NAME, &document, platform_version)
            .expect("validates");
        assert!(validation.is_valid(), "{:?}", validation.errors);

        // The poll consensus opens is the one the proposal's id names
        let opened = DecisionProposalVotePoll::from_proposal(
            document.id(),
            document.properties(),
            platform_version,
        )
        .expect("active")
        .into_data()
        .expect("a valid rule");
        assert_eq!(opened.end_date, input().end_date);
        assert_eq!(opened.vote_poll.supermajority_numerator, 60);
        let proposal = DecisionProposal::from_document(document).expect("reads");
        assert_eq!(opened.vote_poll.purpose, proposal.vote_poll_purpose());

        // A create built from it states the decision proposal fund
        let document_type = contract
            .document_type_for_name(PROPOSAL_DOCUMENT_TYPE_NAME)
            .expect("the proposal type");
        assert_eq!(
            document_type
                .prefunded_voting_balance_for_document(&built.document, platform_version)
                .expect("active"),
            Some((DECISION_PROPOSAL_FUND_NAME.to_string(), 50_000_000_000))
        );
    }

    #[test]
    fn should_leave_out_a_url_not_given() {
        let platform_version = PlatformVersion::latest();
        let contract = proposals_contract();
        let built = build_decision_proposal_document(
            &contract,
            Identifier::from([3; 32]),
            &DecisionProposalInput {
                url: None,
                ..input()
            },
            Bytes32::new([9; 32]),
        )
        .expect("builds");
        assert!(built.document.get(properties::URL).is_none());
        let document = stored(&built.document);
        assert!(contract
            .validate_document(PROPOSAL_DOCUMENT_TYPE_NAME, &document, platform_version)
            .expect("validates")
            .is_valid());
        assert_eq!(
            DecisionProposal::from_document(document)
                .expect("reads")
                .url,
            None
        );
    }

    #[test]
    fn should_read_every_property_of_a_proposal() {
        let built = build_decision_proposal_document(
            &proposals_contract(),
            Identifier::from([3; 32]),
            &input(),
            Bytes32::new([9; 32]),
        )
        .expect("builds");
        let proposal = DecisionProposal::from_document(built.document.clone()).expect("reads");
        assert_eq!(
            proposal,
            DecisionProposal {
                document: built.document.clone(),
                title: input().title,
                description: input().description,
                url: input().url,
                pass_percent: 60,
                minimum_turnout_percent: 10,
                end_date: input().end_date,
            }
        );
        assert_eq!(proposal.id(), built.document.id());
        assert_eq!(proposal.owner_id(), Identifier::from([3; 32]));

        let mut no_title = built.document;
        no_title.remove(properties::TITLE);
        assert!(DecisionProposal::from_document(no_title).is_err());
    }

    #[test]
    fn should_edit_only_the_text_and_move_to_the_next_revision() {
        let contract = proposals_contract();
        let proposal = stored(
            &build_decision_proposal_document(
                &contract,
                Identifier::from([3; 32]),
                &input(),
                Bytes32::new([9; 32]),
            )
            .expect("builds")
            .document,
        );

        let edited = build_decision_proposal_edit(
            &proposal,
            &DecisionProposalEdit {
                title: Some("Raise the block size to 4 MB?".to_string()),
                ..Default::default()
            },
        )
        .expect("edits");
        assert_eq!(edited.id(), proposal.id());
        assert_eq!(edited.revision(), Some(INITIAL_REVISION + 1));
        let read = DecisionProposal::from_document(edited).expect("reads");
        assert_eq!(read.title, "Raise the block size to 4 MB?");
        assert_eq!(read.description, input().description);
        assert_eq!(read.url, input().url);
        assert_eq!(
            (
                read.pass_percent,
                read.minimum_turnout_percent,
                read.end_date
            ),
            (60, 10, input().end_date)
        );

        let without_url = build_decision_proposal_edit(
            &proposal,
            &DecisionProposalEdit {
                description: Some("Yes doubles it.".to_string()),
                url: Some(None),
                ..Default::default()
            },
        )
        .expect("edits");
        let read = DecisionProposal::from_document(without_url).expect("reads");
        assert_eq!(read.description, "Yes doubles it.");
        assert_eq!(read.url, None);

        assert!(
            build_decision_proposal_edit(&proposal, &DecisionProposalEdit::default()).is_err(),
            "an edit that changes nothing is refused before it is paid for"
        );
    }

    #[test]
    fn should_serve_every_listing_through_the_index_the_schema_declares_for_it() {
        let contract = proposals_contract();
        let page = DocumentsPage {
            limit: Some(20),
            start_after: Some(Identifier::from([5; 32])),
        };
        let end_dates = [
            DecisionProposalEndDates::default(),
            DecisionProposalEndDates {
                from: Some(1_700_000_000_000),
                to: None,
            },
            DecisionProposalEndDates {
                from: None,
                to: Some(1_800_000_000_000),
            },
            DecisionProposalEndDates {
                from: Some(1_700_000_000_000),
                to: Some(1_800_000_000_000),
            },
        ];
        for end_dates in end_dates {
            let query =
                proposals_by_end_date_query(contract.clone(), end_dates, page).expect("builds");
            assert_eq!(served_through(&query), "byEndDate", "{end_dates:?}");
            assert_eq!(query.limit, 20);
        }
        assert_eq!(
            served_through(
                &proposals_by_owner_query(contract.clone(), Identifier::from([3; 32]), page)
                    .expect("builds")
            ),
            "byOwner"
        );
        let by_created_at =
            proposals_by_created_at_query(contract, DocumentsPage::default()).expect("builds");
        assert_eq!(served_through(&by_created_at), "byCreatedAt");
        assert_eq!(by_created_at.limit, 100);
        assert_eq!(by_created_at.start, None);
    }

    #[test]
    fn should_name_the_poll_a_vote_answers_by_the_proposal_id() {
        let platform_version = PlatformVersion::latest();
        let proposal_id = Identifier::from([4; 32]);
        let purpose = YesNoVotePollPurpose::DecisionProposal {
            document_id: proposal_id,
        };
        assert_eq!(
            decision_proposal_vote_poll_id(proposal_id, platform_version).expect("active"),
            purpose.unique_id(platform_version).expect("active")
        );
    }

    #[tokio::test]
    async fn should_build_a_vote_on_the_proposal_poll() {
        let sdk = crate::SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("a mock sdk");
        let proposal_id = Identifier::from([4; 32]);
        let vote = sdk
            .decision_proposal_vote(proposal_id, YesNoAbstainVoteChoice::No)
            .await
            .expect("builds");
        assert_eq!(
            sdk.decision_proposal_vote_poll_id(proposal_id)
                .await
                .expect("derives"),
            decision_proposal_vote_poll_id(proposal_id, PlatformVersion::latest()).expect("active")
        );
        assert_eq!(
            vote,
            Vote::YesNoVote(YesNoVote::V0(YesNoVoteV0 {
                vote_poll_id: decision_proposal_vote_poll_id(
                    proposal_id,
                    PlatformVersion::latest()
                )
                .expect("active"),
                vote_choice: YesNoAbstainVoteChoice::No,
            }))
        );
    }
}
