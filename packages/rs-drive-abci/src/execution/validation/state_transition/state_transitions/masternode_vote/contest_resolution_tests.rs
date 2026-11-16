//! The end of a contest whose contenders' stored documents do not decode, or whose winner's
//! document the state refuses. From protocol version 15 (`check_for_ended_vote_polls` 2) such a
//! contender cannot win and such a contest ends with no winner; protocol version 14
//! (`check_for_ended_vote_polls` 1) returns the error. Contract and document checks keep these
//! states from being reached through transitions, so the tests write them to the state directly.

use crate::error::Error;
use crate::execution::validation::state_transition::state_transitions::tests::{
    create_dpns_identity_name_contest, dpns_name_vote_poll, get_vote_states, perform_votes_multi,
    setup_identity,
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
use dpp::dash_to_credits;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::SecurityLevel;
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
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::votes::paths::vote_contested_resource_contract_documents_storage_path;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::error::drive::DriveError;
use drive::error::Error as DriveCrateError;
use drive::grovedb::Element;
use drive::query::DriveDocumentQuery;
use drive::util::object_size_info::DocumentInfo::DocumentRefInfo;
use drive::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use drive::util::test_helpers::{setup_contract, vote_poll_end_dates};
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};

/// A contest on `normalizedLabel` and `rank`, where `domain` sums `rank`
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

/// The contest ended in the block at height 900 with no winner: its record is back to not
/// started, so a new contest may open on its values
fn assert_ended_with_no_winner(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) {
    assert_eq!(
        stored_poll_status(platform, vote_poll, platform_version),
        (ContestedDocumentVotePollStatus::NotStarted, Some(900))
    );
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
async fn should_award_the_next_earliest_contender_when_the_earliest_does_not_decode() {
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
        1,
        "the contender whose document decodes must be awarded it"
    );
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

#[tokio::test]
async fn should_end_a_contest_with_no_winner_when_no_contender_decodes() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    for (_, document) in contender_documents(&platform, &dpns_contract, "quantum", platform_version)
    {
        truncate_contender_document(
            &platform,
            dpns_contract.id(),
            document.id(),
            platform_version,
        );
    }

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    assert_ended_with_no_winner(
        &platform,
        &dpns_name_vote_poll(&dpns_contract, "quantum"),
        platform_version,
    );
    for contender in [contender_1, contender_2] {
        assert!(awarded_domains(&platform, &dpns_contract, contender, platform_version).is_empty());
    }
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

#[tokio::test]
async fn should_award_the_contender_with_the_most_votes_among_those_that_decode() {
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

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    let finished = finished_vote_info(&platform, &dpns_contract, "quantum", platform_version)
        .expect("expected the contest to be finished");
    assert_eq!(finished.won_by_identity_id, Some(contender_2.to_vec()));
    assert_eq!(
        awarded_domains(&platform, &dpns_contract, contender_2, platform_version).len(),
        1
    );
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

#[tokio::test]
async fn should_lock_when_the_lock_beats_every_contender_that_decodes() {
    let platform_version = PlatformVersion::latest();
    let (mut platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    perform_votes_multi(
        &mut platform,
        &dpns_contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1), 3),
            (ResourceVoteChoice::Lock, 2),
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
async fn should_end_a_contest_with_no_winner_when_its_unique_index_entry_is_taken() {
    let platform_version = PlatformVersion::latest();
    let (platform, contender_1, contender_2, dpns_contract) =
        quantum_contest(platform_version).await;
    let (_, template) =
        contender_documents(&platform, &dpns_contract, "quantum", platform_version).remove(0);
    let holder = other_domain_like(&template, 0x51);
    insert_domain_directly(&platform, &dpns_contract, &holder, platform_version);

    end_first_due_vote_polls(&platform, platform_version)
        .expect("expected the contest to end with the block");

    assert_ended_with_no_winner(
        &platform,
        &dpns_name_vote_poll(&dpns_contract, "quantum"),
        platform_version,
    );
    for contender in [contender_1, contender_2] {
        assert!(awarded_domains(&platform, &dpns_contract, contender, platform_version).is_empty());
    }
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
    assert_contest_cleaned_up(&platform, &dpns_contract, "quantum", platform_version);
}

/// Opens a DPNS name contest on "quantum", then one on "coolio" that ends later, takes the
/// unique index entry of the contest at `refused` (0 or 1, in the order they end), and ends
/// both in one block at the latest protocol version. The award of that contest is refused, and
/// the other contest's award must be kept whichever of the two the block ends first.
async fn end_two_contests_in_one_block_with_one_award_refused(refused: usize) {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let (_, _, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        "quantum",
        platform_version,
    )
    .await;
    fast_forward_to_block(&platform, 500_000, 100, 42, 0, false);
    let platform_state = platform.state.load();
    create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        8,
        "coolio",
        platform_version,
    )
    .await;
    let names = ["quantum", "coolio"];
    let end_dates = vote_poll_end_dates(&platform.drive, platform_version);
    assert_eq!(
        end_dates.len(),
        2,
        "expected the contests to end at two dates"
    );
    let last_end_date = *end_dates
        .keys()
        .next_back()
        .expect("expected a vote poll end date");

    let (_, template) =
        contender_documents(&platform, &dpns_contract, names[refused], platform_version).remove(0);
    let holder = other_domain_like(&template, 0x51);
    insert_domain_directly(&platform, &dpns_contract, &holder, platform_version);
    let contenders = names.map(|name| {
        contender_documents(&platform, &dpns_contract, name, platform_version)
            .into_iter()
            .map(|(contender, _)| contender)
            .collect::<Vec<_>>()
    });

    end_vote_polls_due_at(&platform, last_end_date, platform_version)
        .expect("expected both contests to end with the block");

    for (index, name) in names.into_iter().enumerate() {
        if index == refused {
            assert_ended_with_no_winner(
                &platform,
                &dpns_name_vote_poll(&dpns_contract, name),
                platform_version,
            );
            for contender in &contenders[index] {
                assert!(
                    awarded_domains(&platform, &dpns_contract, *contender, platform_version)
                        .is_empty(),
                    "no contender of the refused contest may be awarded"
                );
            }
        } else {
            let finished = finished_vote_info(&platform, &dpns_contract, name, platform_version)
                .expect("expected the other contest to be finished");
            assert_eq!(
                finished.finished_vote_outcome,
                FinishedVoteOutcome::TowardsIdentity as i32
            );
            let winner = Identifier::from_bytes(
                &finished
                    .won_by_identity_id
                    .expect("expected the other contest to have a winner"),
            )
            .expect("expected the winner's id");
            assert!(contenders[index].contains(&winner));
            assert_eq!(
                awarded_domains(&platform, &dpns_contract, winner, platform_version).len(),
                1,
                "the other contest's winner must keep its award"
            );
        }
        assert_contest_cleaned_up(&platform, &dpns_contract, name, platform_version);
    }
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
async fn should_keep_an_earlier_award_of_the_block_when_a_later_award_is_refused() {
    end_two_contests_in_one_block_with_one_award_refused(1).await;
}

#[tokio::test]
async fn should_keep_a_later_award_of_the_block_when_an_earlier_award_is_refused() {
    end_two_contests_in_one_block_with_one_award_refused(0).await;
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

/// Opens a contest on `quantum` with `rank` 1 in [`SUMMED_RANK_CONTRACT`], after writing a
/// domain `big` whose `rank` is `i64::MAX`, then ends it at `platform_version`
async fn end_contest_over_a_full_sum(
    platform_version: &PlatformVersion,
) -> (
    TempPlatform<MockCoreRPCLike>,
    DataContract,
    Identifier,
    Result<(), Error>,
) {
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut rng = StdRng::seed_from_u64(0x5011_0000);
    let (identity, signer, key) = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contract_owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contract = setup_contract(
        &platform.drive,
        SUMMED_RANK_CONTRACT,
        None,
        Some(contract_owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(platform_version),
    );
    let domain = contract
        .document_type_for_name("domain")
        .expect("expected a domain document type");

    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = domain
        .random_document_with_identifier_and_entropy(
            &mut rng,
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

    let mut big = document.clone();
    big.set("label", "big".into());
    big.set("normalizedLabel", "big".into());
    big.set("rank", Value::I64(i64::MAX));
    let big = other_domain_like(&big, 0x61);
    insert_domain_directly(&platform, &contract, &big, platform_version);

    document.set("label", "quantum".into());
    document.set("normalizedLabel", "quantum".into());
    document.set("rank", Value::I64(1));
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
            &BlockInfo::default_with_time(5000),
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

    let result = end_first_due_vote_polls(&platform, platform_version);
    (platform, contract, identity.id(), result)
}

#[tokio::test]
async fn should_end_a_contest_with_no_winner_when_the_award_takes_a_sum_past_its_range() {
    let platform_version = PlatformVersion::latest();
    let (platform, contract, contender, result) =
        end_contest_over_a_full_sum(platform_version).await;
    result.expect("expected the contest to end with the block");

    assert!(
        awarded_domains(&platform, &contract, contender, platform_version).is_empty(),
        "nothing of the refused award may stay in the state"
    );
    assert!(vote_poll_end_dates(&platform.drive, platform_version).is_empty());
    assert_ended_with_no_winner(
        &platform,
        &ContestedDocumentResourceVotePoll {
            contract_id: contract.id(),
            document_type_name: "domain".to_string(),
            index_name: "parentNameAndLabel".to_string(),
            index_values: vec![
                Value::Text("dash".to_string()),
                Value::Text("quantum".to_string()),
                Value::I64(1),
            ],
        },
        platform_version,
    );
}

#[tokio::test]
async fn should_fail_on_an_award_taking_a_sum_past_its_range_at_protocol_version_14() {
    let (_, _, _, result) = end_contest_over_a_full_sum(protocol_version_14()).await;
    assert_matches!(result, Err(Error::Drive(DriveCrateError::GroveDB(_))));
}

/// Ends the same contest, with votes for both contenders and the lock, at the latest protocol
/// version with `check_for_ended_vote_polls` and
/// `remove_contested_resource_vote_poll_documents_operations` at the given generations, and
/// returns the state's root hash, the outcome and the awarded domains of the winner
async fn end_an_ordinary_contest_with_generations(
    (end_generation, clean_up_generation): (u16, u16),
) -> ([u8; 32], Option<FinishedVoteInfo>, Vec<Document>) {
    let mut platform_version = PlatformVersion::latest().clone();
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
        awarded_domains(&platform, &dpns_contract, contender_2, &platform_version),
    )
}

/// An ordinary contest ends with the same winner, records and state with the latest
/// generations as with the previous ones
#[tokio::test]
async fn should_end_an_ordinary_contest_as_the_previous_generation_does() {
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
    let previous = end_an_ordinary_contest_with_generations((1, 1)).await;
    let latest = end_an_ordinary_contest_with_generations(latest_generations).await;
    assert_eq!(
        latest
            .1
            .as_ref()
            .map(|finished| finished.finished_vote_outcome),
        Some(FinishedVoteOutcome::TowardsIdentity as i32)
    );
    assert_eq!(latest.2.len(), 1, "the winner must be awarded its domain");
    assert_eq!(latest, previous);
}
