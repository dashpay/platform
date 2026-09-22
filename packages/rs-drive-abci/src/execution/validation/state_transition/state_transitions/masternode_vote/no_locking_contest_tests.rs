//! Contests on a unique index resolved without locking
//! (`ContestedIndexResolution::MasternodeVoteNoLocking`, protocol version 14): no Lock choice,
//! a single contender is awarded when the join window closes, a second contender opens the
//! vote window, and a tie goes to the earliest contender.

use crate::execution::validation::state_transition::state_transitions::tests::{
    create_dpns_identity_name_contest, get_vote_states, perform_vote, perform_votes_multi,
    setup_identity, setup_masternode_voting_identity,
};
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dapi_grpc::platform::v0::get_contested_resource_vote_state_request::get_contested_resource_vote_state_request_v0::ResultType;
use dapi_grpc::platform::v0::get_contested_resource_vote_state_response::get_contested_resource_vote_state_response_v0::{
    finished_vote_info, FinishedVoteInfo,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::state::voting::vote_choice_not_allowed_for_vote_poll_error::VoteChoiceNotAllowedForVotePollError;
use dpp::dash_to_credits;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, TimestampMillis};
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::util::hash::hash_double;
use dpp::util::strings::convert_to_homograph_safe_chars;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use drive::query::VotePollsByEndDateDriveQuery;
use drive::util::test_helpers::setup_contract;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use std::sync::Arc;

/// The DPNS-shaped fixture whose `parentNameAndLabel` index is resolved without locking.
const NO_LOCKING_CONTRACT: &str =
    "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-no-locking.json";
const NAME: &str = "quantum";

type IdentityInfo = (Identity, SimpleSigner, IdentityPublicKey);

fn block(time_ms: TimestampMillis, height: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        height,
        core_height: 42,
        epoch: Default::default(),
    }
}

fn setup() -> (
    TempPlatform<MockCoreRPCLike>,
    &'static PlatformVersion,
    DataContract,
    StdRng,
) {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let mut rng = StdRng::seed_from_u64(0x9010_C41A);
    let owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contract = setup_contract(
        &platform.drive,
        NO_LOCKING_CONTRACT,
        None,
        Some(owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(platform_version),
    );
    (platform, platform_version, contract, rng)
}

/// The join window and the poll duration the platform under test applies.
fn windows(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> (TimestampMillis, TimestampMillis) {
    match platform.config.network {
        Network::Mainnet => (
            platform_version
                .dpp
                .validation
                .voting
                .allow_other_contenders_time_mainnet_ms,
            platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_mainnet_ms,
        ),
        _ => (
            platform_version
                .dpp
                .validation
                .voting
                .allow_other_contenders_time_testing_ms,
            platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_test_network_ms,
        ),
    }
}

fn vote_poll(contract: &DataContract) -> VotePoll {
    VotePoll::ContestedDocumentResourceVotePoll(ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text(convert_to_homograph_safe_chars(NAME)),
        ],
    })
}

/// The serialized preorder and domain creations of one contender for `NAME`.
async fn contest_transitions(
    contract: &DataContract,
    (identity, signer, key): &IdentityInfo,
    salt_discriminator: u8,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> (Vec<u8>, Vec<u8>) {
    let preorder = contract
        .document_type_for_name("preorder")
        .expect("expected preorder document type");
    let domain = contract
        .document_type_for_name("domain")
        .expect("expected domain document type");
    let entropy = Bytes32::random_with_rng(rng);
    let mut preorder_document = preorder
        .random_document_with_identifier_and_entropy(
            rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random preorder document");
    preorder_document
        .set_id_for_creation(preorder, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    let mut domain_document = domain
        .random_document_with_identifier_and_entropy(
            rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            platform_version,
        )
        .expect("expected a random domain document");
    domain_document
        .set_id_for_creation(domain, &entropy.0, 3, platform_version)
        .expect("expected to set the document id");
    domain_document.set("parentDomainName", "dash".into());
    domain_document.set("normalizedParentDomainName", "dash".into());
    domain_document.set("label", NAME.into());
    domain_document.set(
        "normalizedLabel",
        convert_to_homograph_safe_chars(NAME).into(),
    );
    domain_document.set("records.identity", domain_document.owner_id().into());
    domain_document.set("subdomainRules.allowSubdomains", false.into());
    let mut salt: [u8; 32] = [0u8; 32];
    salt[31] = salt_discriminator;
    let mut salted_domain_buffer: Vec<u8> = vec![];
    salted_domain_buffer.extend(salt);
    salted_domain_buffer.extend((convert_to_homograph_safe_chars(NAME) + ".dash").as_bytes());
    preorder_document.set("saltedDomainHash", hash_double(salted_domain_buffer).into());
    domain_document.set("preorderSalt", salt.into());
    let preorder_transition = BatchTransition::new_document_creation_transition_from_document(
        preorder_document,
        preorder,
        entropy.0,
        key,
        2,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create preorder batch transition");
    let domain_transition = BatchTransition::new_document_creation_transition_from_document(
        domain_document,
        domain,
        entropy.0,
        key,
        3,
        0,
        None,
        signer,
        platform_version,
        None,
    )
    .await
    .expect("expect to create domain batch transition");
    (
        preorder_transition
            .serialize_to_bytes()
            .expect("serialize preorder transition"),
        domain_transition
            .serialize_to_bytes()
            .expect("serialize domain transition"),
    )
}

/// Processes transitions in a block at `time_ms` and expects every one of them to pass.
fn process_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    transitions: &[Vec<u8>],
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            transitions,
            &platform_state,
            &block(time_ms, 1),
            &transaction,
            platform_version,
            false,
            None,
        )
        .expect("expected to process state transitions");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");
    assert_eq!(
        processing_result.valid_count(),
        transitions.len(),
        "every transition should pass: {:?}",
        processing_result.execution_results()
    );
}

/// A contender joins the contest for `NAME`: the preorder at `time_ms`, the domain one second
/// later. Returns the domain's block time, which is when the contest starts or is joined.
async fn join(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    identity: &IdentityInfo,
    salt_discriminator: u8,
    time_ms: TimestampMillis,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> TimestampMillis {
    let (preorder, domain) = contest_transitions(
        contract,
        identity,
        salt_discriminator,
        rng,
        platform_version,
    )
    .await;
    process_valid(platform, &[preorder], time_ms, platform_version);
    process_valid(platform, &[domain], time_ms + 1000, platform_version);
    time_ms + 1000
}

fn end_dates(
    platform: &TempPlatform<MockCoreRPCLike>,
    platform_version: &PlatformVersion,
) -> Vec<(TimestampMillis, VotePoll)> {
    VotePollsByEndDateDriveQuery {
        start_time: None,
        end_time: None,
        limit: None,
        offset: None,
        order_ascending: true,
    }
    .execute_no_proof(&platform.drive, None, &mut vec![], platform_version)
    .expect("expected the end date entries")
    .into_iter()
    .flat_map(|(time, polls)| polls.into_iter().map(move |poll| (time, poll)))
    .collect()
}

/// Ends every poll due at `time_ms`, as the block at that time would.
fn end_polls_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    height: u64,
    platform_version: &PlatformVersion,
) {
    let mut platform_state = (**platform.state.load()).clone();
    let block_info = block(time_ms, height);
    platform_state.set_last_committed_block_info(Some(
        ExtendedBlockInfoV0 {
            basic_info: block_info,
            app_hash: platform
                .drive
                .grove
                .root_hash(None, &platform_version.drive.grove_version)
                .unwrap()
                .unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            proposer_pro_tx_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        }
        .into(),
    ));
    platform.state.store(Arc::new(platform_state));
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .check_for_ended_vote_polls(
            &platform_state,
            &platform_state,
            &block_info,
            Some(&transaction),
            platform_version,
        )
        .expect("expected to check for ended vote polls");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit transaction");
}

/// The identity the finished contest was awarded to, if it finished.
fn winner(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Option<Identifier> {
    let platform_state = platform.state.load();
    let (_, _, _, finished_vote_info) = get_vote_states(
        platform,
        &platform_state,
        contract,
        NAME,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    );
    finished_vote_info.map(
        |FinishedVoteInfo {
             finished_vote_outcome,
             won_by_identity_id,
             ..
         }| {
            assert_eq!(
                finished_vote_outcome,
                finished_vote_info::FinishedVoteOutcome::TowardsIdentity as i32,
                "a contest without locking always has a winner"
            );
            Identifier::from_vec(won_by_identity_id.expect("expected a winner"))
                .expect("expected an identifier")
        },
    )
}

#[tokio::test]
async fn should_refuse_a_lock_vote_and_accept_the_other_choices() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let alice = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let bob = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    join(
        &platform,
        &contract,
        &alice,
        1,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    join(
        &platform,
        &contract,
        &bob,
        2,
        20_000,
        &mut rng,
        platform_version,
    )
    .await;

    let (pro_tx_hash, _, signer, voting_key) =
        setup_masternode_voting_identity(&mut platform, 0x10c, platform_version);
    let platform_state = platform.state.load();
    let refused =
        VoteChoiceNotAllowedForVotePollError::new(vote_poll(&contract), ResourceVoteChoice::Lock)
            .to_string();
    perform_vote(
        &mut platform,
        &platform_state,
        &contract,
        ResourceVoteChoice::Lock,
        NAME,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        Some(&refused),
        platform_version,
    )
    .await;
    perform_vote(
        &mut platform,
        &platform_state,
        &contract,
        ResourceVoteChoice::Abstain,
        NAME,
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        None,
        platform_version,
    )
    .await;
    let (abstaining, locking, tallies) = tallies_of(&platform, &contract, platform_version);
    assert_eq!(abstaining, Some(1));
    assert_eq!(locking, Some(0));
    assert!(tallies.contains(&(alice.0.id(), Some(0))));
    assert!(tallies.contains(&(bob.0.id(), Some(0))));

    // The same masternode changes its vote: a changed vote replaces the previous one
    perform_vote(
        &mut platform,
        &platform_state,
        &contract,
        ResourceVoteChoice::TowardsIdentity(alice.0.id()),
        NAME,
        &signer,
        pro_tx_hash,
        &voting_key,
        2,
        None,
        platform_version,
    )
    .await;

    // The masternode changed its vote, so its abstain vote is gone and alice has it
    let (abstaining, locking, tallies) = tallies_of(&platform, &contract, platform_version);
    assert_eq!(abstaining, Some(0));
    assert_eq!(locking, Some(0));
    assert!(tallies.contains(&(alice.0.id(), Some(1))));
    assert!(tallies.contains(&(bob.0.id(), Some(0))));
}

/// The abstain and lock tallies and every contender's tally of the still-running contest.
fn tallies_of(
    platform: &TempPlatform<MockCoreRPCLike>,
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> (Option<u32>, Option<u32>, Vec<(Identifier, Option<u32>)>) {
    let platform_state = platform.state.load();
    let (contenders, abstaining, locking, finished) = get_vote_states(
        platform,
        &platform_state,
        contract,
        NAME,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    );
    assert!(finished.is_none());
    let tallies = contenders
        .iter()
        .map(|contender| (contender.identity_id(), contender.vote_tally()))
        .collect();
    (abstaining, locking, tallies)
}

#[tokio::test]
async fn should_award_a_single_contender_when_the_join_window_closes() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let (join_window, poll_duration) = windows(&platform, platform_version);
    let alice = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let start = join(
        &platform,
        &contract,
        &alice,
        1,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;

    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(start + join_window, vote_poll(&contract))],
        "the contest ends with its join window while it has one contender"
    );
    assert!(start + join_window < start + poll_duration);

    end_polls_at(&platform, start + join_window - 1, 10, platform_version);
    assert_eq!(winner(&platform, &contract, platform_version), None);

    end_polls_at(&platform, start + join_window, 11, platform_version);
    assert_eq!(
        winner(&platform, &contract, platform_version),
        Some(alice.0.id())
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

#[tokio::test]
async fn should_open_the_vote_window_when_a_second_contender_joins() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let (join_window, poll_duration) = windows(&platform, platform_version);
    let alice = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let bob = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let carol = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let start = join(
        &platform,
        &contract,
        &alice,
        1,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    join(
        &platform,
        &contract,
        &bob,
        2,
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(start + poll_duration, vote_poll(&contract))],
        "the second contender moves the end to the full poll duration"
    );

    // A third contender finds the end date there already
    join(
        &platform,
        &contract,
        &carol,
        3,
        start + 120_000,
        &mut rng,
        platform_version,
    )
    .await;
    assert_eq!(
        end_dates(&platform, platform_version),
        vec![(start + poll_duration, vote_poll(&contract))]
    );

    perform_votes_multi(
        &mut platform,
        &contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(bob.0.id()), 3),
            (ResourceVoteChoice::TowardsIdentity(alice.0.id()), 2),
            (ResourceVoteChoice::Abstain, 1),
        ],
        NAME,
        100,
        None,
        platform_version,
    )
    .await;

    end_polls_at(&platform, start + join_window, 10, platform_version);
    assert_eq!(
        winner(&platform, &contract, platform_version),
        None,
        "the join window closing no longer ends the contest"
    );

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        winner(&platform, &contract, platform_version),
        Some(bob.0.id()),
        "plurality"
    );
    assert!(end_dates(&platform, platform_version).is_empty());
}

#[tokio::test]
async fn should_award_a_tie_to_the_earliest_contender() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let (_, poll_duration) = windows(&platform, platform_version);
    let alice = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let bob = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let start = join(
        &platform,
        &contract,
        &alice,
        1,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    join(
        &platform,
        &contract,
        &bob,
        2,
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    perform_votes_multi(
        &mut platform,
        &contract,
        vec![
            (ResourceVoteChoice::TowardsIdentity(bob.0.id()), 2),
            (ResourceVoteChoice::TowardsIdentity(alice.0.id()), 2),
        ],
        NAME,
        100,
        None,
        platform_version,
    )
    .await;

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        winner(&platform, &contract, platform_version),
        Some(alice.0.id()),
        "alice joined first"
    );
}

#[tokio::test]
async fn should_award_the_earliest_contender_when_nobody_votes() {
    let (mut platform, platform_version, contract, mut rng) = setup();
    let (_, poll_duration) = windows(&platform, platform_version);
    let alice = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let bob = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let start = join(
        &platform,
        &contract,
        &alice,
        1,
        10_000,
        &mut rng,
        platform_version,
    )
    .await;
    join(
        &platform,
        &contract,
        &bob,
        2,
        start + 60_000,
        &mut rng,
        platform_version,
    )
    .await;

    end_polls_at(&platform, start + poll_duration, 11, platform_version);
    assert_eq!(
        winner(&platform, &contract, platform_version),
        Some(alice.0.id())
    );
}

/// The DPNS rule keeps its Lock choice, and from protocol version 14 its ties go to the
/// earliest contender too: two documents created in the same block tie on time and
/// heights, so the smaller document id wins.
#[tokio::test]
async fn should_award_a_dpns_tie_to_the_earliest_contender_from_version_14() {
    let platform_version = PlatformVersion::latest();
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
    let platform_state = platform.state.load();
    let (contender_1, contender_2, dpns_contract) = create_dpns_identity_name_contest(
        &mut platform,
        &platform_state,
        7,
        NAME,
        platform_version,
    )
    .await;
    perform_votes_multi(
        &mut platform,
        dpns_contract.as_ref(),
        vec![
            (ResourceVoteChoice::TowardsIdentity(contender_1.id()), 4),
            (ResourceVoteChoice::TowardsIdentity(contender_2.id()), 4),
        ],
        NAME,
        10,
        None,
        platform_version,
    )
    .await;
    let (contenders, _, _, _) = get_vote_states(
        &platform,
        &platform_state,
        dpns_contract.as_ref(),
        NAME,
        None,
        true,
        None,
        ResultType::DocumentsAndVoteTally,
        platform_version,
    );
    let earliest = contenders
        .iter()
        .map(|contender| {
            let document = contender
                .document()
                .as_ref()
                .expect("expected the contender's document");
            (document.id(), contender.identity_id())
        })
        .min()
        .expect("expected contenders")
        .1;

    let (_, poll_duration) = windows(&platform, platform_version);
    end_polls_at(&platform, poll_duration + 300_000, 11, platform_version);
    assert_eq!(
        winner(&platform, dpns_contract.as_ref(), platform_version),
        Some(earliest)
    );
}
