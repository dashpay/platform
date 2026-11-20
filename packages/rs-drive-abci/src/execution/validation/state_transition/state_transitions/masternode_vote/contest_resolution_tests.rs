//! The end of a contest whose winner cannot be awarded its document: a contender with the
//! highest tally holds a document that does not decode, or the state would refuse the winner's
//! document, or its type sums values past the limit a contested type is held to. From protocol version 15 (`check_for_ended_vote_polls` 2) such a contest is left
//! open: no one is awarded, nothing of it is recorded or removed, and it is listed under the
//! end date no block reaches. Protocol version 14 (`check_for_ended_vote_polls` 1) returns the
//! error. Contract and document checks keep these states from being reached through
//! transitions, so the tests write them to the state directly.

use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::tests::{
    create_dpns_identity_name_contest, dpns_name_vote_poll, first_time_check_tx_errors,
    get_vote_states, perform_votes, perform_votes_multi, serialized_dpns_name_vote, setup_identity,
    setup_masternode_voting_identity,
};
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::finished_vote_info::FinishedVoteOutcome;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::FinishedVoteInfo;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::fee::Credits;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, SecurityLevel};
use dpp::platform_value::{Bytes32, Identifier, Value};
use dpp::prelude::TimestampMillis;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::document_base_transition::DocumentBaseTransition;
use dpp::state_transition::batch_transition::document_create_transition::DocumentCreateTransitionV0;
use dpp::state_transition::batch_transition::{
    BatchTransition, BatchTransitionV0, DocumentCreateTransition,
};
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::{
    ContestedDocumentVotePollStatus, ContestedDocumentVotePollStoredInfoV0Getters,
};
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::drive::document::paths::document_history_path;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::votes::paths::{
    vote_contested_resource_contract_documents_storage_path, VOTE_POLLS_LEFT_OPEN_END_DATE,
};
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::error::drive::DriveError;
use drive::error::Error as DriveCrateError;
use drive::grovedb::Element;
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::test_helpers::{setup_contract, vote_poll_end_dates};
use rand::prelude::StdRng;
use simple_signer::signer::SimpleSigner;
use rand::{Rng, SeedableRng};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

/// A contest on `normalizedLabel` and `rank`, where `domain` sums `rank` and declares no bounds
/// on it: a type protocol version 13 and earlier registered
const SUMMED_RANK_CONTRACT: &str =
    "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-summed-integer.json";

fn protocol_version_14() -> &'static PlatformVersion {
    PlatformVersion::get(14).expect("expected protocol version 14")
}

/// Runs the ended vote poll check of a block at the first end date and returns its result,
/// committing the block when it succeeds
fn end_first_due_vote_polls(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    let time_ms: TimestampMillis = *vote_poll_end_dates(&platform.drive, platform_version)
        .keys()
        .next()
        .expect("expected a vote poll end date");
    end_vote_polls_due_at(platform, time_ms, platform_version)
}

/// Runs the ended vote poll check of a block at `time_ms`, which ends every vote poll due by
/// then, and returns its result, committing the block when it succeeds
fn end_vote_polls_due_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    fast_forward_to_block(platform, time_ms, 900, 42, 0, false);
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform.check_for_ended_vote_polls(
        &platform_state,
        &platform_state,
        &BlockInfo {
            time_ms,
            height: 900,
            core_height: 42,
            epoch: Default::default(),
        },
        Some(&transaction),
        platform_version,
    )?;
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the block");
    Ok(())
}

/// The documents of the DPNS name contest on `name`, by contender
fn contender_documents(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    name: &str,
    platform_version: &PlatformVersion,
) -> Vec<(Identifier, Document)> {
    let platform_state = platform.state.load();
    let (contenders, ..) = get_vote_states(
        platform,
        &platform_state,
        dpns_contract,
        name,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    );
    contenders
        .into_iter()
        .map(|contender| {
            (
                contender.identity_id(),
                contender
                    .document()
                    .clone()
                    .expect("expected the contender's document"),
            )
        })
        .collect()
}

fn finished_vote_info(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    name: &str,
    platform_version: &PlatformVersion,
) -> Option<FinishedVoteInfo> {
    let platform_state = platform.state.load();
    get_vote_states(
        platform,
        &platform_state,
        dpns_contract,
        name,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    )
    .3
}

/// The status the stored record of `vote_poll` holds, and the height of its last finalization
fn stored_poll_status(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> (ContestedDocumentVotePollStatus, Option<u64>) {
    let resolved_vote_poll = vote_poll
        .resolve(&platform.drive, None, platform_version)
        .expect("expected to resolve the vote poll");
    let (_, stored_info) = platform
        .drive
        .fetch_contested_document_vote_poll_stored_info(
            &resolved_vote_poll,
            None,
            None,
            platform_version,
        )
        .expect("expected to read the vote poll's record");
    let stored_info = stored_info.expect("expected the vote poll's record");
    (
        stored_info.vote_poll_status(),
        stored_info
            .last_finalization_block()
            .map(|block_info| block_info.height),
    )
}

/// What a contest holds in the state: its record's status and the height of its last
/// finalization, its prefunded balance, and what is stored for each of the given contenders'
/// documents
#[derive(Debug, PartialEq)]
struct ContestState {
    status: (ContestedDocumentVotePollStatus, Option<u64>),
    prefunded_balance: Credits,
    contender_documents: Vec<Option<Element>>,
}

fn contest_state(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePoll,
    document_ids: &[Identifier],
    platform_version: &PlatformVersion,
) -> ContestState {
    let path = vote_contested_resource_contract_documents_storage_path(
        vote_poll.contract_id.as_slice(),
        &vote_poll.document_type_name,
    );
    let balance_id = vote_poll
        .specialized_balance_id()
        .expect("expected the prefunded balance id");
    ContestState {
        status: stored_poll_status(platform, vote_poll, platform_version),
        prefunded_balance: platform
            .drive
            .fetch_prefunded_specialized_balance(balance_id.to_buffer(), None, platform_version)
            .expect("expected to read the prefunded balance")
            .unwrap_or_default(),
        contender_documents: document_ids
            .iter()
            .map(|document_id| {
                platform
                    .drive
                    .grove
                    .get_raw_optional(
                        path.as_ref().into(),
                        document_id.as_slice(),
                        None,
                        &platform_version.drive.grove_version,
                    )
                    .unwrap()
                    .expect("expected to read the contender's document")
            })
            .collect(),
    }
}

/// The contest holds what it held `before` its end, which is all an open contest holds: a
/// record that is started and never finalized, its prefunded balance and its contenders'
/// documents
fn assert_holds_what_it_held(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePoll,
    document_ids: &[Identifier],
    before: &ContestState,
    platform_version: &PlatformVersion,
) {
    assert_matches!(
        before.status,
        (ContestedDocumentVotePollStatus::Started(_), None)
    );
    assert!(before.prefunded_balance > 0);
    assert!(before.contender_documents.iter().all(Option::is_some));
    assert_eq!(
        &contest_state(platform, vote_poll, document_ids, platform_version),
        before,
        "a contest left open must hold what it held before its end"
    );
}

/// The end dates list nothing but `vote_polls`, under the left open end date
fn assert_only_left_open_listed(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_polls: &[&ContestedDocumentResourceVotePoll],
    platform_version: &PlatformVersion,
) {
    let unique_ids: BTreeSet<Identifier> = vote_polls
        .iter()
        .map(|vote_poll| vote_poll.unique_id().expect("expected the vote poll's id"))
        .collect();
    assert_eq!(
        vote_poll_end_dates(&platform.drive, platform_version),
        BTreeMap::from([(VOTE_POLLS_LEFT_OPEN_END_DATE, unique_ids)])
    );
}

/// Ends the DPNS name contest on "quantum" and checks that the block goes through and leaves
/// the contest open: it holds what it held, only the left open end date lists it, and neither
/// contender is awarded the name
fn assert_ending_leaves_the_quantum_contest_open(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    document_ids: &[Identifier],
    contenders: [Identifier; 2],
    platform_version: &PlatformVersion,
) {
    let vote_poll = dpns_name_vote_poll(dpns_contract, "quantum");
    let before = contest_state(platform, &vote_poll, document_ids, platform_version);

    end_first_due_vote_polls(platform, platform_version).expect("expected the block to go through");

    assert_holds_what_it_held(
        platform,
        &vote_poll,
        document_ids,
        &before,
        platform_version,
    );
    assert_only_left_open_listed(platform, &[&vote_poll], platform_version);
    for contender in contenders {
        assert!(
            awarded_domains(platform, dpns_contract, contender, platform_version).is_empty(),
            "no contender may be awarded a contest left open"
        );
    }
}

/// The ids of `documents`
fn document_ids(documents: &[(Identifier, Document)]) -> Vec<Identifier> {
    documents
        .iter()
        .map(|(_, document)| document.id())
        .collect()
}

/// Cuts the stored bytes of a contender's document short, keeping its storage flags, so they
/// no longer decode
fn truncate_contender_document(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract_id: Identifier,
    document_id: Identifier,
    platform_version: &PlatformVersion,
) {
    let grove_version = &platform_version.drive.grove_version;
    let path =
        vote_contested_resource_contract_documents_storage_path(contract_id.as_slice(), "domain");
    let element = platform
        .drive
        .grove
        .get_raw(
            path.as_ref().into(),
            document_id.as_slice(),
            None,
            grove_version,
        )
        .unwrap()
        .expect("expected the contender's stored document");
    let Element::Item(bytes, flags) = element else {
        panic!("expected the contender's document to be stored as an item");
    };
    platform
        .drive
        .grove
        .insert(
            path.as_ref(),
            document_id.as_slice(),
            Element::Item(bytes[..10].to_vec(), flags),
            None,
            None,
            grove_version,
        )
        .unwrap()
        .expect("expected to overwrite the contender's document");
}

/// Writes `document` to the state as a document of `contract`'s `domain` outside any contest,
/// without the checks of a transition
fn insert_domain_directly(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    document: &Document,
    platform_version: &PlatformVersion,
) {
    platform
        .drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((document, None)),
                    owner_id: Some(document.owner_id().to_buffer()),
                },
                contract,
                document_type: contract
                    .document_type_for_name("domain")
                    .expect("expected a domain document type"),
            },
            false,
            BlockInfo::default_with_time(4000),
            true,
            None,
            platform_version,
        )
        .expect("expected to write the domain");
}

/// `template` with another id and owner
fn other_domain_like(template: &Document, seed: u8) -> Document {
    let mut document = template.clone();
    let owner_id = Identifier::new([seed; 32]);
    document.set_id(Identifier::new([seed.wrapping_add(1); 32]));
    document.set_owner_id(owner_id);
    document.set("records.identity", owner_id.into());
    document
}

/// The DPNS name contest on "quantum", with two contenders and no votes, at `platform_version`
async fn quantum_contest(
    platform_version: &PlatformVersion,
) -> (
    TempPlatform<MockCoreRPCLike>,
    Identifier,
    Identifier,
    DataContract,
) {
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        "quantum",
        platform_version,
    )
    .await;
    (
        platform,
        contender_1.id(),
        contender_2.id(),
        dpns_contract.as_ref().clone(),
    )
}

/// The contest is gone: no end date, no prefunded balance, no contender left
fn assert_contest_cleaned_up(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    name: &str,
    platform_version: &PlatformVersion,
) {
    assert!(
        vote_poll_end_dates(&platform.drive, platform_version).is_empty(),
        "the contest's end date must be removed"
    );
    let balance_id = dpns_name_vote_poll(dpns_contract, name)
        .specialized_balance_id()
        .expect("expected the prefunded balance id");
    let balance = platform
        .drive
        .fetch_prefunded_specialized_balance(balance_id.to_buffer(), None, platform_version)
        .expect("expected to read the prefunded balance");
    assert_eq!(
        balance.unwrap_or_default(),
        0,
        "the contest's prefunded balance must be emptied"
    );
    let platform_state = platform.state.load();
    let (contenders, ..) = get_vote_states(
        platform,
        &platform_state,
        dpns_contract,
        name,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    );
    assert!(
        contenders
            .iter()
            .all(|contender| contender.document().is_none()),
        "the contenders' documents must be removed"
    );
}

/// The `domain` documents of `owner_id` in the state, outside any contest
fn awarded_domains(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    owner_id: Identifier,
    platform_version: &PlatformVersion,
) -> Vec<Document> {
    let query = DriveDocumentQuery::all_items_query(
        dpns_contract,
        dpns_contract
            .document_type_for_name("domain")
            .expect("expected a domain document type"),
        None,
    );
    let documents = platform
        .drive
        .query_documents(
            query,
            None,
            false,
            None,
            Some(platform_version.protocol_version),
        )
        .expect("expected to query the domains");
    documents
        .documents()
        .iter()
        .filter(|document| document.owner_id() == owner_id)
        .cloned()
        .collect()
}

#[tokio::test]
async fn should_leave_a_contest_open_when_the_earliest_of_the_top_contenders_does_not_decode() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    // With no votes the earliest contender, contender 2, would win
    let (_, earliest_document) = documents
        .iter()
        .find(|(contender, _)| *contender == contender_2)
        .expect("expected contender 2");
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        earliest_document.id(),
        platform_version,
    );

    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );
}

#[tokio::test]
async fn should_leave_a_contest_open_when_no_contender_decodes() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    for (_, document) in &documents {
        truncate_contender_document(
            &platform,
            dpns_contract.id(),
            document.id(),
            platform_version,
        );
    }

    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );
}

#[tokio::test]
async fn should_leave_a_contest_open_with_its_votes_when_the_contender_with_the_most_votes_does_not_decode(
) {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1), 3),
            (ResourceVoteChoice::TowardsIdentity(contender_2), 2),
            (ResourceVoteChoice::Lock, 1),
        ],
        "quantum",
        10,
        None,
        platform_version,
    )
    .await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let (_, leading_document) = documents
        .iter()
        .find(|(contender, _)| *contender == contender_1)
        .expect("expected contender 1");
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        leading_document.id(),
        platform_version,
    );

    // The contender with fewer votes, whose document decodes, is not awarded in its place
    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );

    let platform_state = platform.state.load();
    let (contenders, _, lock_tally, finished) = get_vote_states(
        &platform,
        &platform_state,
        &dpns_contract,
        "quantum",
        None,
        true,
        None,
        ResultType::VoteTally,
        platform_version,
    );
    let tallies: BTreeMap<Identifier, Option<u32>> = contenders
        .iter()
        .map(|contender| (contender.identity_id(), contender.vote_tally()))
        .collect();
    assert_eq!(
        tallies,
        BTreeMap::from([(contender_1, Some(3)), (contender_2, Some(2))]),
        "the votes must stay"
    );
    assert_eq!(lock_tally, Some(1));
    assert_eq!(finished, None);
}

/// A masternode votes on the DPNS name contest on "quantum", which its end left open: check_tx
/// and the block refuse the vote as one on a contest that takes no more votes
async fn assert_vote_on_the_quantum_contest_is_refused(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    choice: ResourceVoteChoice,
    (pro_tx_hash, _, signer, voting_key): &(Identifier, Identity, SimpleSigner, IdentityPublicKey),
    nonce: u64,
    platform_version: &PlatformVersion,
) {
    let serialized_vote = serialized_dpns_name_vote(
        dpns_contract,
        choice,
        "quantum",
        signer,
        *pro_tx_hash,
        voting_key,
        nonce,
        platform_version,
    )
    .await;
    let platform_state = platform.state.load();
    assert_matches!(
        first_time_check_tx_errors(
            platform,
            &platform_state,
            &serialized_vote,
            platform_version
        )
        .as_slice(),
        [ConsensusError::StateError(
            StateError::VotePollNotAvailableForVotingError(_)
        )]
    );
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized_vote],
            &platform_state,
            &BlockInfo::default(),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the vote");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the block");
    assert_matches!(
        processing_result.into_execution_results().remove(0),
        StateTransitionExecutionResult::UnpaidConsensusError(ConsensusError::StateError(
            StateError::VotePollNotAvailableForVotingError(_)
        ))
    );
}

#[tokio::test]
async fn should_refuse_a_vote_on_a_contest_left_open_and_take_votes_on_another() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let voters = perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1), 3),
            (ResourceVoteChoice::TowardsIdentity(contender_2), 2),
        ],
        "quantum",
        10,
        None,
        platform_version,
    )
    .await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let (_, leading_document) = documents
        .iter()
        .find(|(contender, _)| *contender == contender_1)
        .expect("expected contender 1");
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        leading_document.id(),
        platform_version,
    );
    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );

    // A masternode that has not voted, then one changing the vote it cast
    let new_voter = setup_masternode_voting_identity(&mut platform, 100, platform_version);
    assert_vote_on_the_quantum_contest_is_refused(
        &platform,
        &dpns_contract,
        ResourceVoteChoice::TowardsIdentity(contender_2),
        &new_voter,
        1,
        platform_version,
    )
    .await;
    let earlier_voter = &voters[&ResourceVoteChoice::TowardsIdentity(contender_1)][0];
    assert_vote_on_the_quantum_contest_is_refused(
        &platform,
        &dpns_contract,
        ResourceVoteChoice::TowardsIdentity(contender_2),
        earlier_voter,
        2,
        platform_version,
    )
    .await;
    let platform_state = platform.state.load();
    let (contenders, ..) = get_vote_states(
        &platform,
        &platform_state,
        &dpns_contract,
        "quantum",
        None,
        true,
        None,
        ResultType::VoteTally,
        platform_version,
    );
    let tallies: BTreeMap<Identifier, Option<u32>> = contenders
        .iter()
        .map(|contender| (contender.identity_id(), contender.vote_tally()))
        .collect();
    assert_eq!(
        tallies,
        BTreeMap::from([(contender_1, Some(3)), (contender_2, Some(2))]),
        "the tallies must stay what they were at the contest's end"
    );

    // A contest opened since takes votes as usual
    let (_, _, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        8,
        "cooldog",
        platform_version,
    )
    .await;
    perform_votes(
        &mut platform,
        &dpns_contract,
        ResourceVoteChoice::Lock,
        "cooldog",
        1,
        200,
        None,
        platform_version,
    )
    .await;
}

#[tokio::test]
async fn should_award_the_contender_with_the_most_votes_when_one_with_fewer_does_not_decode() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1), 3),
            (ResourceVoteChoice::TowardsIdentity(contender_2), 2),
        ],
        "quantum",
        10,
        None,
        platform_version,
    )
    .await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let (_, trailing_document) = documents
        .iter()
        .find(|(contender, _)| *contender == contender_2)
        .expect("expected contender 2");
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        trailing_document.id(),
        platform_version,
    );

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    let finished = finished_vote_info(&platform, &dpns_contract, "quantum", platform_version)
        .expect("expected the contest to be finished");
    assert_eq!(
        finished.finished_vote_outcome,
        FinishedVoteOutcome::TowardsIdentity as i32
    );
    assert_eq!(finished.won_by_identity_id, Some(contender_1.to_vec()));
    assert_eq!(
        awarded_domains(&platform, &dpns_contract, contender_1, platform_version).len(),
        1
    );
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

#[tokio::test]
async fn should_lock_when_the_lock_beats_a_top_contender_that_does_not_decode() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::Lock, 3),
            (ResourceVoteChoice::TowardsIdentity(contender_1), 2),
            (ResourceVoteChoice::TowardsIdentity(contender_2), 1),
        ],
        "quantum",
        10,
        None,
        platform_version,
    )
    .await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let (_, leading_document) = documents
        .iter()
        .find(|(contender, _)| *contender == contender_1)
        .expect("expected contender 1");
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        leading_document.id(),
        platform_version,
    );

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    let finished = finished_vote_info(&platform, &dpns_contract, "quantum", platform_version)
        .expect("expected the contest to be finished");
    assert_eq!(
        finished.finished_vote_outcome,
        FinishedVoteOutcome::Locked as i32
    );
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

#[tokio::test]
async fn should_fail_on_a_contender_that_does_not_decode_at_protocol_version_14() {
    let platform_version = protocol_version_14();
    let (platform, _, _, dpns_contract) = quantum_contest(platform_version).await;
    let (_, document) =
        contender_documents(&platform, &dpns_contract, "quantum", platform_version).remove(0);
    truncate_contender_document(
        &platform,
        dpns_contract.id(),
        document.id(),
        platform_version,
    );

    assert_matches!(
        end_first_due_vote_polls(&platform, platform_version),
        Err(Error::Protocol(_))
    );
}

#[tokio::test]
async fn should_leave_a_contest_open_when_its_unique_index_entry_is_taken() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let holder = other_domain_like(&documents[0].1, 0x51);
    insert_domain_directly(&platform, &dpns_contract, &holder, platform_version);

    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );
    assert_eq!(
        awarded_domains(
            &platform,
            &dpns_contract,
            holder.owner_id(),
            platform_version
        ),
        vec![holder],
        "the document holding the entry must stay as it was"
    );
}

#[tokio::test]
async fn should_fail_on_a_taken_unique_index_entry_at_protocol_version_14() {
    let platform_version = protocol_version_14();
    let (platform, _, _, dpns_contract) = quantum_contest(platform_version).await;
    let (_, template) =
        contender_documents(&platform, &dpns_contract, "quantum", platform_version).remove(0);
    insert_domain_directly(
        &platform,
        &dpns_contract,
        &other_domain_like(&template, 0x51),
        platform_version,
    );

    assert_matches!(
        end_first_due_vote_polls(&platform, platform_version),
        Err(Error::Drive(DriveCrateError::Drive(
            DriveError::CorruptedContractIndexes(_)
        )))
    );
}

/// Writes, for each contender of the contest on "quantum", a domain outside the contest with
/// the id of the contender's document, another owner and another label, so no unique index
/// value of a contender's document is held but the id the award writes it at is taken.
/// Returns the domains written.
fn take_contender_document_ids(
    platform: &TempPlatform<MockCoreRPCLike>,
    dpns_contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Vec<Document> {
    contender_documents(platform, dpns_contract, "quantum", platform_version)
        .into_iter()
        .zip([(0x51, "zebra"), (0x61, "yaks")])
        .map(|((_, contender_document), (seed, label))| {
            let mut holder = other_domain_like(&contender_document, seed);
            holder.set_id(contender_document.id());
            holder.set("label", label.into());
            holder.set("normalizedLabel", label.into());
            insert_domain_directly(platform, dpns_contract, &holder, platform_version);
            holder
        })
        .collect()
}

#[tokio::test]
async fn should_leave_a_contest_open_when_the_awarded_document_id_is_taken() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", platform_version);
    let holders = take_contender_document_ids(&platform, &dpns_contract, platform_version);

    assert_ending_leaves_the_quantum_contest_open(
        &platform,
        &dpns_contract,
        &document_ids(&documents),
        [contender_1, contender_2],
        platform_version,
    );
    for holder in holders {
        assert_eq!(
            awarded_domains(
                &platform,
                &dpns_contract,
                holder.owner_id(),
                platform_version
            ),
            vec![holder],
            "the document holding the id must stay as it was"
        );
    }
}

#[tokio::test]
async fn should_fail_on_a_taken_awarded_document_id_at_protocol_version_14() {
    let platform_version = protocol_version_14();
    let (platform, _, _, dpns_contract) = quantum_contest(platform_version).await;
    take_contender_document_ids(&platform, &dpns_contract, platform_version);

    assert_matches!(
        end_first_due_vote_polls(&platform, platform_version),
        Err(Error::Drive(DriveCrateError::Drive(
            DriveError::CorruptedDocumentAlreadyExists(_)
        )))
    );
}

/// What a test changes in the `domain` type of [`SUMMED_RANK_CONTRACT`]
#[derive(Clone, Copy, Default)]
struct SummedRankContract {
    /// `rank` declares the bounds registration holds the summed values of a contested type
    /// to from protocol version 14 (within 2^27 of 0)
    bounded_rank: bool,
    /// The type keeps the history of its documents, which registration refuses on a
    /// contested type from protocol version 15
    keeps_history: bool,
    /// `rank` is left out of the contested index, declared between 0 and 1000 and listed as
    /// transient, which registration refuses on a type summing it from protocol version 15
    transient_rank: bool,
    /// The type sums `stats.rank`, an integer between 0 and 1000 in a required `stats` object,
    /// instead of `rank`: a dotted name, which registration refuses from protocol version 14
    dotted_summed_name: bool,
    /// The type sums `rank` through its `identityId` index alone instead of `documentsSummable`
    rank_summed_by_an_index: bool,
}

/// A platform at `platform_version` with [`SUMMED_RANK_CONTRACT`] changed as `changes` say,
/// written to the state without the checks of a registration
fn platform_with_summed_rank_contract(
    changes: SummedRankContract,
    platform_version: &PlatformVersion,
) -> (TempPlatform<MockCoreRPCLike>, DataContract, StdRng) {
    let mut schema: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(SUMMED_RANK_CONTRACT).expect("expected to read the contract"),
    )
    .expect("expected the contract to be JSON");
    let domain = &mut schema["documentSchemas"]["domain"];
    if changes.bounded_rank {
        domain["properties"]["rank"]["minimum"] = (-(1i64 << 27)).into();
        domain["properties"]["rank"]["maximum"] = (1i64 << 27).into();
    }
    if changes.keeps_history {
        domain["documentsKeepHistory"] = true.into();
    }
    if changes.transient_rank {
        domain["indices"][0]["properties"] = serde_json::json!([
            { "normalizedParentDomainName": "asc" },
            { "normalizedLabel": "asc" },
        ]);
        domain["properties"]["rank"]["minimum"] = 0.into();
        domain["properties"]["rank"]["maximum"] = 1000.into();
        domain["transient"] = serde_json::json!(["rank"]);
    }
    if changes.dotted_summed_name {
        domain["properties"]["stats"] = serde_json::json!({
            "type": "object",
            "properties": {
                "rank": { "type": "integer", "minimum": 0, "maximum": 1000, "position": 0 },
            },
            "required": ["rank"],
            "additionalProperties": false,
            "position": 8,
        });
        domain["required"]
            .as_array_mut()
            .expect("expected the required list")
            .push("stats".into());
        domain["documentsSummable"] = "stats.rank".into();
    }
    if changes.rank_summed_by_an_index {
        domain
            .as_object_mut()
            .expect("expected the domain schema")
            .remove("documentsSummable");
        domain["indices"][1]["summable"] = "rank".into();
    }
    let contract_file = tempfile::Builder::new()
        .suffix(".json")
        .tempfile()
        .expect("expected a file for the contract");
    fs::write(contract_file.path(), schema.to_string()).expect("expected to write the contract");

    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut rng = StdRng::seed_from_u64(0x5011_0000);
    let contract_owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contract = setup_contract(
        &platform.drive,
        contract_file
            .path()
            .to_str()
            .expect("expected the contract's path"),
        None,
        Some(contract_owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(platform_version),
    );
    (platform, contract, rng)
}

/// Writes a domain with `label` and `rank` to the state outside any contest, owned by an
/// identity of its own, without the checks of a transition; returns it
fn insert_summed_rank_domain(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    rng: &mut StdRng,
    label: &str,
    rank: i64,
    platform_version: &PlatformVersion,
) -> Document {
    let mut domain = contract
        .document_type_for_name("domain")
        .expect("expected a domain document type")
        .random_document_with_rng(rng, platform_version)
        .expect("expected a random domain document");
    domain.set("parentDomainName", "dash".into());
    domain.set("normalizedParentDomainName", "dash".into());
    domain.set("subdomainRules.allowSubdomains", false.into());
    domain.set("preorderSalt", rng.gen::<[u8; 32]>().into());
    domain.set("label", label.into());
    domain.set("normalizedLabel", label.into());
    domain.set("rank", Value::I64(rank));
    let domain = other_domain_like(&domain, rng.gen());
    insert_domain_directly(platform, contract, &domain, platform_version);
    domain
}

/// A new identity joins a contest on `label` with `rank` in [`SUMMED_RANK_CONTRACT`] in a block
/// at `time_ms`, opening it; returns the identity's id and its document's
async fn join_summed_rank_contest(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    rng: &mut StdRng,
    label: &str,
    rank: i64,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> (Identifier, Identifier) {
    let (identity, signer, key) = setup_identity(platform, rng.gen(), dash_to_credits!(0.5));
    let domain = contract
        .document_type_for_name("domain")
        .expect("expected a domain document type");
    let entropy = Bytes32::random_with_rng(rng);
    let mut document = domain
        .random_document_with_identifier_and_entropy(
            rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random domain document");
    document.set("parentDomainName", "dash".into());
    document.set("normalizedParentDomainName", "dash".into());
    document.set("records.identity", document.owner_id().into());
    document.set("subdomainRules.allowSubdomains", false.into());
    document.set("preorderSalt", rng.gen::<[u8; 32]>().into());
    document.set("label", label.into());
    document.set("normalizedLabel", label.into());
    document.set("rank", Value::I64(rank));
    // A type summing `stats.rank` holds the rank there too
    if domain.properties().contains_key("stats") {
        document.set("stats.rank", Value::I64(rank));
    }
    document
        .set_id_for_creation(domain, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    let create_transition: DocumentCreateTransition = DocumentCreateTransitionV0 {
        base: DocumentBaseTransition::from_document(
            &document,
            domain,
            None,
            2,
            platform_version,
            None,
        )
        .expect("expected a base transition"),
        entropy: entropy.0,
        data: document.clone().properties_consumed(),
        prefunded_voting_balance: Some((
            "parentNameAndLabel".to_string(),
            platform_version
                .fee_version
                .vote_resolution_fund_fees
                .contested_document_vote_resolution_fund_required_amount,
        )),
    }
    .into();
    let batch: BatchTransition = BatchTransitionV0 {
        owner_id: identity.id(),
        transitions: vec![create_transition.into()],
        user_fee_increase: 0,
        signature_public_key_id: 0,
        signature: Default::default(),
    }
    .into();
    let mut transition: StateTransition = batch.into();
    transition
        .sign_external(&key, &signer, Some(|_, _| Ok(SecurityLevel::HIGH)))
        .await
        .expect("expected to sign");
    let serialized_transition = transition
        .serialize_to_bytes()
        .expect("expected to serialize the transition");
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized_transition],
            &platform_state,
            &BlockInfo::default_with_time(time_ms),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process the state transition");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");
    assert_matches!(
        processing_result.into_execution_results().remove(0),
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "the contender must join the contest"
    );
    (identity.id(), document.id())
}

/// The contest on `label` with `rank` in [`SUMMED_RANK_CONTRACT`]
fn summed_rank_vote_poll(
    contract: &DataContract,
    label: &str,
    rank: i64,
) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text(label.to_string()),
            Value::I64(rank),
        ],
    }
}

#[tokio::test]
async fn should_leave_a_contest_open_on_a_type_summing_values_past_the_contested_limit() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) =
        platform_with_summed_rank_contract(SummedRankContract::default(), platform_version);
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;
    let vote_poll = summed_rank_vote_poll(&contract, "quantum", 1);
    let before = contest_state(&platform, &vote_poll, &[document_id], platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the block to go through");

    assert_holds_what_it_held(
        &platform,
        &vote_poll,
        &[document_id],
        &before,
        platform_version,
    );
    assert_only_left_open_listed(&platform, &[&vote_poll], platform_version);
    assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
}

/// A contested create stores its document without the values of the type's transient
/// properties, so the winner's document of a type summing one does not hold the value the
/// type's sums read. Registration refuses such a type from protocol version 15; one stored
/// earlier is left open at the end of its contest, as any winner whose document cannot be
/// written is.
#[tokio::test]
async fn should_leave_a_contest_open_when_the_winner_does_not_hold_its_summed_value() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            transient_rank: true,
            ..Default::default()
        },
        platform_version,
    );
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;
    let vote_poll = ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text("quantum".to_string()),
        ],
    };
    let before = contest_state(&platform, &vote_poll, &[document_id], platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the block to go through");

    assert_holds_what_it_held(
        &platform,
        &vote_poll,
        &[document_id],
        &before,
        platform_version,
    );
    assert_only_left_open_listed(&platform, &[&vote_poll], platform_version);
    assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
}

/// A type summing its transient property through an index alone, without `documentsSummable`,
/// is left open as well: Drive reads the value of every index the document enters.
#[tokio::test]
async fn should_leave_a_contest_open_when_the_winner_does_not_hold_the_value_an_index_sums() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            transient_rank: true,
            rank_summed_by_an_index: true,
            ..Default::default()
        },
        platform_version,
    );
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;
    let vote_poll = ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text("quantum".to_string()),
        ],
    };
    let before = contest_state(&platform, &vote_poll, &[document_id], platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the block to go through");

    assert_holds_what_it_held(
        &platform,
        &vote_poll,
        &[document_id],
        &before,
        platform_version,
    );
    assert_only_left_open_listed(&platform, &[&vote_poll], platform_version);
    assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
}

/// Drive reads a summed value from the top level of a document, so the winner's document of a
/// type summing a property named by a dotted path holds no value its sums read either. Such a
/// contest is left open too.
#[tokio::test]
async fn should_leave_a_contest_open_on_a_type_summing_a_dotted_name() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            bounded_rank: true,
            dotted_summed_name: true,
            ..Default::default()
        },
        platform_version,
    );
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;
    let vote_poll = summed_rank_vote_poll(&contract, "quantum", 1);
    let before = contest_state(&platform, &vote_poll, &[document_id], platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the block to go through");

    assert_holds_what_it_held(
        &platform,
        &vote_poll,
        &[document_id],
        &before,
        platform_version,
    );
    assert_only_left_open_listed(&platform, &[&vote_poll], platform_version);
    assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
}

#[tokio::test]
async fn should_award_a_contest_on_a_type_summing_values_within_the_contested_limit() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            bounded_rank: true,
            ..Default::default()
        },
        platform_version,
    );
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    assert_eq!(
        contest_state(
            &platform,
            &summed_rank_vote_poll(&contract, "quantum", 1),
            &[document_id],
            platform_version
        ),
        ContestState {
            status: (
                ContestedDocumentVotePollStatus::Awarded(contender),
                Some(900)
            ),
            prefunded_balance: 0,
            contender_documents: vec![None],
        }
    );
    assert_eq!(
        awarded_domains(&platform, &contract, contender, platform_version).len(),
        1
    );
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());
}

#[tokio::test]
async fn should_fail_on_an_award_taking_a_sum_past_its_range_at_protocol_version_14() {
    let platform_version = protocol_version_14();
    let (mut platform, contract, mut rng) =
        platform_with_summed_rank_contract(SummedRankContract::default(), platform_version);
    insert_summed_rank_domain(
        &platform,
        &contract,
        &mut rng,
        "big",
        i64::MAX,
        platform_version,
    );
    join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;

    assert_matches!(
        end_first_due_vote_polls(&platform, platform_version),
        Err(Error::Drive(DriveCrateError::GroveDB(_)))
    );
}

#[tokio::test]
async fn should_leave_a_contest_open_when_a_deleted_document_retains_the_awarded_id() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            bounded_rank: true,
            keeps_history: true,
            ..Default::default()
        },
        platform_version,
    );
    let (contender, document_id) = join_summed_rank_contest(
        &mut platform,
        &contract,
        &mut rng,
        "quantum",
        1,
        5000,
        platform_version,
    )
    .await;
    // The tree of the revisions a document of this id retains, with no document under the id
    let grove_version = &platform_version.drive.grove_version;
    let mut history_trees_path = document_history_path(
        contract.id_ref().as_bytes(),
        "domain",
        document_id.as_slice(),
    );
    let history_tree_key = history_trees_path
        .pop()
        .expect("expected the document's history tree key");
    let history_trees_key = history_trees_path
        .pop()
        .expect("expected the history trees key");
    for (path, key) in [
        (&history_trees_path[..], &history_trees_key),
        (
            &[&history_trees_path[..], &[history_trees_key.clone()]].concat()[..],
            &history_tree_key,
        ),
    ] {
        platform
            .drive
            .grove
            .insert_if_not_exists(path, key, Element::empty_tree(), None, grove_version)
            .unwrap()
            .expect("expected to write the history tree");
    }
    let vote_poll = summed_rank_vote_poll(&contract, "quantum", 1);
    let before = contest_state(&platform, &vote_poll, &[document_id], platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the block to go through");

    assert_holds_what_it_held(
        &platform,
        &vote_poll,
        &[document_id],
        &before,
        platform_version,
    );
    assert_only_left_open_listed(&platform, &[&vote_poll], platform_version);
    assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
}

/// Opens a contest whose winner's unique index entry another document holds (`quantum`,
/// `rank` 1) and one awarded as usual (`zebra`, `rank` 0), the one at `left_open` (0 or 1)
/// opening first, at `times`, and ends both in one block at the latest protocol version. The
/// first is left open and the other is awarded and cleaned up, whichever of the two the block
/// ends first and whether or not they share an end date.
async fn end_two_contests_in_one_block_with_one_left_open(
    left_open: usize,
    times: [TimestampMillis; 2],
) {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            bounded_rank: true,
            ..Default::default()
        },
        platform_version,
    );
    let left_open_contest = ("quantum", 1);
    let awarded_contest = ("zebra", 0);
    let contests = if left_open == 0 {
        [left_open_contest, awarded_contest]
    } else {
        [awarded_contest, left_open_contest]
    };
    let mut contenders = vec![];
    for ((label, rank), time_ms) in contests.into_iter().zip(times) {
        contenders.push(
            join_summed_rank_contest(
                &mut platform,
                &contract,
                &mut rng,
                label,
                rank,
                time_ms,
                platform_version,
            )
            .await,
        );
    }
    let (label, rank) = left_open_contest;
    insert_summed_rank_domain(
        &platform,
        &contract,
        &mut rng,
        label,
        rank,
        platform_version,
    );
    let end_dates = vote_poll_end_dates(&platform.drive, platform_version);
    assert_eq!(
        end_dates.len(),
        BTreeSet::from(times).len(),
        "expected the contests to end at as many dates as they opened at"
    );
    let last_end_date = *end_dates
        .keys()
        .next_back()
        .expect("expected a vote poll end date");
    let left_open_vote_poll = summed_rank_vote_poll(&contract, label, rank);
    let (_, left_open_document_id) = contenders[left_open];
    let before = contest_state(
        &platform,
        &left_open_vote_poll,
        &[left_open_document_id],
        platform_version,
    );

    end_vote_polls_due_at(&platform, last_end_date, platform_version)
        .expect("expected the block to go through");

    assert_only_left_open_listed(&platform, &[&left_open_vote_poll], platform_version);
    for (index, ((label, rank), (contender, document_id))) in
        contests.into_iter().zip(contenders).enumerate()
    {
        let vote_poll = summed_rank_vote_poll(&contract, label, rank);
        if index == left_open {
            assert_holds_what_it_held(
                &platform,
                &vote_poll,
                &[document_id],
                &before,
                platform_version,
            );
            assert!(awarded_domains(&platform, &contract, contender, platform_version).is_empty());
        } else {
            assert_eq!(
                contest_state(&platform, &vote_poll, &[document_id], platform_version),
                ContestState {
                    status: (
                        ContestedDocumentVotePollStatus::Awarded(contender),
                        Some(900)
                    ),
                    prefunded_balance: 0,
                    contender_documents: vec![None],
                },
                "the other contest must be awarded and cleaned up"
            );
            assert_eq!(
                awarded_domains(&platform, &contract, contender, platform_version).len(),
                1
            );
        }
    }
}

#[tokio::test]
async fn should_award_an_earlier_contest_of_the_block_when_a_later_one_is_left_open() {
    end_two_contests_in_one_block_with_one_left_open(1, [5000, 500_000]).await;
}

#[tokio::test]
async fn should_award_a_later_contest_of_the_block_when_an_earlier_one_is_left_open() {
    end_two_contests_in_one_block_with_one_left_open(0, [5000, 500_000]).await;
}

#[tokio::test]
async fn should_award_a_contest_and_leave_open_another_sharing_its_end_date() {
    end_two_contests_in_one_block_with_one_left_open(0, [5000, 5000]).await;
}

#[tokio::test]
async fn should_list_contests_left_open_under_one_end_date_and_not_end_them_again() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contract, mut rng) = platform_with_summed_rank_contract(
        SummedRankContract {
            bounded_rank: true,
            ..Default::default()
        },
        platform_version,
    );
    let mut contests = vec![];
    for ((label, rank), time_ms) in [("quantum", 1), ("zebra", 2)]
        .into_iter()
        .zip([5000, 500_000])
    {
        let (_, document_id) = join_summed_rank_contest(
            &mut platform,
            &contract,
            &mut rng,
            label,
            rank,
            time_ms,
            platform_version,
        )
        .await;
        // Another document holds the winner's unique index entry
        insert_summed_rank_domain(
            &platform,
            &contract,
            &mut rng,
            label,
            rank,
            platform_version,
        );
        contests.push((summed_rank_vote_poll(&contract, label, rank), document_id));
    }
    let before: Vec<ContestState> = contests
        .iter()
        .map(|(vote_poll, document_id)| {
            contest_state(&platform, vote_poll, &[*document_id], platform_version)
        })
        .collect();
    let end_dates: Vec<TimestampMillis> = vote_poll_end_dates(&platform.drive, platform_version)
        .into_keys()
        .collect();
    assert_eq!(
        end_dates.len(),
        2,
        "expected the contests to end at two dates"
    );

    // Each contest is left open in a block of its own, then a later block finds nothing due
    for time_ms in [end_dates[0], end_dates[1], end_dates[1] + 500_000] {
        end_vote_polls_due_at(&platform, time_ms, platform_version)
            .expect("expected the block to go through");
    }

    let vote_polls: Vec<&ContestedDocumentResourceVotePoll> =
        contests.iter().map(|(vote_poll, _)| vote_poll).collect();
    assert_only_left_open_listed(&platform, &vote_polls, platform_version);
    for ((vote_poll, document_id), before) in contests.iter().zip(&before) {
        assert_holds_what_it_held(
            &platform,
            vote_poll,
            &[*document_id],
            before,
            platform_version,
        );
    }
}

/// Ends the same contest, with votes for both contenders and the lock, at the latest protocol
/// version with `check_for_ended_vote_polls` and
/// `remove_contested_resource_vote_poll_documents_operations` at the given generations, the
/// tally reading at most `contenders_considered` contenders when given, and returns the state's
/// root hash, the outcome and the awarded domains of both contenders
async fn end_an_ordinary_contest_with_generations(
    (end_generation, clean_up_generation): (u16, u16),
    contenders_considered: Option<u16>,
) -> ([u8; 32], Option<FinishedVoteInfo>, [Vec<Document>; 2]) {
    let mut platform_version = PlatformVersion::latest().clone();
    if let Some(contenders_considered) = contenders_considered {
        platform_version
            .drive_abci
            .validation_and_processing
            .event_constants
            .maximum_contenders_to_consider = contenders_considered;
    }
    platform_version
        .drive_abci
        .methods
        .voting
        .check_for_ended_vote_polls = end_generation;
    platform_version
        .drive
        .methods
        .vote
        .cleanup
        .remove_contested_resource_vote_poll_documents_operations = clean_up_generation;
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(&platform_version).await;
    perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1), 2),
            (ResourceVoteChoice::TowardsIdentity(contender_2), 3),
            (ResourceVoteChoice::Lock, 1),
        ],
        "quantum",
        10,
        None,
        &platform_version,
    )
    .await;
    end_first_due_vote_polls(&platform, &platform_version)
        .expect("expected the contest to end with the block");
    let root_hash = platform
        .drive
        .grove
        .root_hash(None, &platform_version.drive.grove_version)
        .unwrap()
        .expect("expected the root hash");
    (
        root_hash,
        finished_vote_info(&platform, &dpns_contract, "quantum", &platform_version),
        [contender_1, contender_2].map(|contender| {
            awarded_domains(&platform, &dpns_contract, contender, &platform_version)
        }),
    )
}

/// The generations of `check_for_ended_vote_polls` and
/// `remove_contested_resource_vote_poll_documents_operations` the latest protocol version
/// selects
fn latest_end_of_contest_generations() -> (u16, u16) {
    let latest_version = PlatformVersion::latest();
    let latest_generations = (
        latest_version
            .drive_abci
            .methods
            .voting
            .check_for_ended_vote_polls,
        latest_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_contested_resource_vote_poll_documents_operations,
    );
    assert_eq!(latest_generations, (2, 2));
    latest_generations
}

/// An ordinary contest ends with the same winner, records and state with the latest
/// generations as with the previous ones
#[tokio::test]
async fn should_end_an_ordinary_contest_as_the_previous_generation_does() {
    let previous = end_an_ordinary_contest_with_generations((1, 1), None).await;
    let latest =
        end_an_ordinary_contest_with_generations(latest_end_of_contest_generations(), None).await;
    assert_eq!(
        latest
            .1
            .as_ref()
            .map(|finished| finished.finished_vote_outcome),
        Some(FinishedVoteOutcome::TowardsIdentity as i32)
    );
    assert_eq!(
        latest.2[1].len(),
        1,
        "the winner must be awarded its domain"
    );
    assert_eq!(latest, previous);
}

/// A contest opened before protocol version 14 may hold more contenders than the tally reads.
/// Its end leaves a contender the tally did not read whole, its document with its reference,
/// so the contest's vote state stays readable (`end_an_ordinary_contest_with_generations`
/// reads it). The previous generation removes that contender's document and keeps the
/// reference to it.
#[tokio::test]
async fn should_keep_whole_a_contender_the_tally_did_not_read() {
    let platform_version = PlatformVersion::latest();
    let (_, finished, awarded) =
        end_an_ordinary_contest_with_generations(latest_end_of_contest_generations(), Some(1))
            .await;
    assert_eq!(
        finished.map(|finished| finished.finished_vote_outcome),
        Some(FinishedVoteOutcome::TowardsIdentity as i32)
    );
    assert_eq!(
        awarded.iter().map(Vec::len).sum::<usize>(),
        1,
        "the contender the tally read must be awarded its domain"
    );
    // The same contest again, to read what it holds after its end
    let mut capped_version = platform_version.clone();
    capped_version
        .drive_abci
        .validation_and_processing
        .event_constants
        .maximum_contenders_to_consider = 1;
    let (platform, _, _, dpns_contract) = quantum_contest(&capped_version).await;
    let documents = contender_documents(&platform, &dpns_contract, "quantum", &capped_version);
    end_first_due_vote_polls(&platform, &capped_version)
        .expect("expected the contest to end with the block");
    let after = contest_state(
        &platform,
        &dpns_name_vote_poll(&dpns_contract, "quantum"),
        &document_ids(&documents),
        &capped_version,
    );
    assert_eq!(
        after
            .contender_documents
            .iter()
            .filter(|document| document.is_some())
            .count(),
        1,
        "the document of the contender the tally did not read must stay"
    );
    let platform_state = platform.state.load();
    let (contenders, ..) = get_vote_states(
        &platform,
        &platform_state,
        &dpns_contract,
        "quantum",
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        &capped_version,
    );
    assert_eq!(
        contenders
            .iter()
            .filter(|contender| contender.document().is_some())
            .count(),
        1,
        "the contender the tally did not read must stay readable with its document"
    );
}
