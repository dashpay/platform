//! A contest is stored at the path of its index keys, and its poll is named by an id hashed from
//! its index values. An integer sent as `U64(5)` and as `U8(5)` has one key. From protocol
//! version 15 a contested create writes every index value the one way its property's type keys
//! it, so the contenders of one contest name one poll. A masternode vote names its poll by the
//! values it sends. A contest opened at an earlier protocol version keeps the poll it was
//! opened under: a contender joining it later writes its values as that version did.

use crate::config::PlatformConfig;
use crate::execution::validation::state_transition::state_transitions::tests::{
    setup_identity, setup_masternode_voting_identity,
};
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use assert_matches::assert_matches;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::dash_to_credits;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::{
    CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
};
use dpp::data_contract::DataContract;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, TimestampMillis};
use dpp::platform_value::{Bytes32, Value};
use dpp::prelude::Identifier;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::masternode_vote_transition::methods::MasternodeVoteTransitionMethodsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use dpp::voting::votes::resource_vote::ResourceVote;
use dpp::voting::votes::Vote;
use drive::drive::votes::paths::VotePollPaths;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use drive::util::test_helpers::{setup_contract, vote_poll_end_dates};
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// The DPNS-shaped fixture whose contested index ends in the integer property `rank`.
const INTEGER_CONTRACT: &str =
    "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-integer.json";
/// The same fixture with the contested index resolved without locking.
const NO_LOCKING_INTEGER_CONTRACT: &str = "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-no-locking-with-integer.json";
/// The same fixture with `rank` a number.
const NUMBER_CONTRACT: &str =
    "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-number.json";
const NAME: &str = "quantum";

type IdentityInfo = (Identity, SimpleSigner, IdentityPublicKey);

/// A platform starting at `protocol_version`, on mainnet: on testnet a contested create's poll
/// is not checked against its document before epoch 2080.
fn platform_at(protocol_version: u32) -> TempPlatform<MockCoreRPCLike> {
    TestPlatformBuilder::new()
        .with_config(PlatformConfig {
            network: Network::Mainnet,
            ..Default::default()
        })
        .with_initial_protocol_version(protocol_version)
        .build_with_mock_rpc()
        .with_no_core_masternodes()
        .set_genesis_state()
}

fn block(time_ms: TimestampMillis, epoch_index: u16) -> BlockInfo {
    BlockInfo {
        time_ms,
        height: 1,
        core_height: 42,
        epoch: Epoch::new(epoch_index).expect("expected an epoch"),
    }
}

fn poll(contract: &DataContract, rank: Value) -> ContestedDocumentResourceVotePoll {
    ContestedDocumentResourceVotePoll {
        contract_id: contract.id(),
        document_type_name: "domain".to_string(),
        index_name: "parentNameAndLabel".to_string(),
        index_values: vec![
            Value::Text("dash".to_string()),
            Value::Text(NAME.to_string()),
            rank,
        ],
    }
}

/// The serialized domain creation of a contender whose `rank` is sent as `rank`.
async fn contender_transition(
    contract: &DataContract,
    (identity, signer, key): &IdentityInfo,
    rank: Value,
    rng: &mut StdRng,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    let domain = contract
        .document_type_for_name("domain")
        .expect("expected domain document type");
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
    document
        .set_id_for_creation(domain, &entropy.0, 2, platform_version)
        .expect("expected to set the document id");
    document.set("parentDomainName", "dash".into());
    document.set("normalizedParentDomainName", "dash".into());
    document.set("label", NAME.into());
    document.set("normalizedLabel", NAME.into());
    document.set("records.identity", document.owner_id().into());
    document.set("subdomainRules.allowSubdomains", false.into());
    document.set("preorderSalt", [7u8; 32].into());
    document.set("rank", rank);
    BatchTransition::new_document_creation_transition_from_document(
        document,
        domain,
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
    .expect("expect to create domain batch transition")
    .serialize_to_bytes()
    .expect("serialize domain transition")
}

async fn vote_transition(
    vote_poll: ContestedDocumentResourceVotePoll,
    resource_vote_choice: ResourceVoteChoice,
    signer: &SimpleSigner,
    pro_tx_hash: Identifier,
    voting_key: &IdentityPublicKey,
    nonce: u64,
    platform_version: &PlatformVersion,
) -> Vec<u8> {
    let vote = Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
        vote_poll: VotePoll::ContestedDocumentResourceVotePoll(vote_poll),
        resource_vote_choice,
    }));
    MasternodeVoteTransition::try_from_vote_with_signer(
        vote,
        signer,
        pro_tx_hash,
        voting_key,
        nonce,
        platform_version,
        None,
    )
    .await
    .expect("expected to make transition vote")
    .serialize_to_bytes()
    .expect("expected to serialize the masternode vote")
}

/// Processes transitions in a block at `time_ms` of epoch `epoch_index` and returns how each
/// one ended.
fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transitions: &[Vec<u8>],
    time_ms: TimestampMillis,
    epoch_index: u16,
    platform_version: &PlatformVersion,
) -> Vec<StateTransitionExecutionResult> {
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    let processing_result = platform
        .platform
        .process_raw_state_transitions(
            transitions,
            &platform_state,
            &block(time_ms, epoch_index),
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
    processing_result.execution_results().to_vec()
}

fn process_valid_in_epoch(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    epoch_index: u16,
    platform_version: &PlatformVersion,
) {
    assert_matches!(
        process(
            platform,
            &[transition],
            time_ms,
            epoch_index,
            platform_version
        )
        .as_slice(),
        [StateTransitionExecutionResult::SuccessfulExecution { .. }]
    );
}

fn process_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    process_valid_in_epoch(platform, transition, time_ms, 0, platform_version);
}

/// `rank` 5 as the latest protocol version writes it in a poll, and in another integer form.
fn rank_forms(contract: &DataContract) -> (Value, Value) {
    let domain = contract
        .document_type_for_name("domain")
        .expect("expected domain document type");
    let rank = domain
        .indexes()
        .get("parentNameAndLabel")
        .expect("expected the contested index")
        .extract_contested_values(
            &BTreeMap::from([("rank".to_string(), Value::U64(5))]),
            domain.flattened_properties(),
            PlatformVersion::latest(),
        )
        .expect("expected the values of the contest")
        .pop()
        .expect("expected the rank");
    let alternate_rank = [Value::U8(5), Value::U64(5)]
        .into_iter()
        .find(|form| form != &rank)
        .expect("expected another form of the rank");
    (rank, alternate_rank)
}

/// Ends every poll due at `time_ms`, as the block at that time would.
fn end_polls_at(
    platform: &TempPlatform<MockCoreRPCLike>,
    time_ms: TimestampMillis,
    epoch_index: u16,
    platform_version: &PlatformVersion,
) {
    let mut platform_state = (**platform.state.load()).clone();
    let block_info = block(time_ms, epoch_index);
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

fn balance(
    platform: &TempPlatform<MockCoreRPCLike>,
    vote_poll: &ContestedDocumentResourceVotePoll,
    platform_version: &PlatformVersion,
) -> Option<Credits> {
    platform
        .drive
        .fetch_prefunded_specialized_balance(
            vote_poll
                .specialized_balance_id()
                .expect("expected a balance id")
                .to_buffer(),
            None,
            platform_version,
        )
        .expect("expected to fetch the prefunded balance")
}

/// Whether the vote of `pro_tx_hash` towards `contender` is stored in the contest.
fn vote_is_stored(
    contest: &Contest,
    contender: Identifier,
    pro_tx_hash: Identifier,
    platform_version: &PlatformVersion,
) -> bool {
    let path = contest
        .poll
        .resolve_with_provided_borrowed_contract(&contest.contract)
        .expect("expected to resolve the poll")
        .contender_voting_path(
            &ResourceVoteChoice::TowardsIdentity(contender),
            platform_version,
        )
        .expect("expected the voting path of the contender");
    let path: Vec<&[u8]> = path.iter().map(|part| part.as_slice()).collect();
    contest
        .platform
        .drive
        .grove
        .has_raw(
            path.as_slice(),
            pro_tx_hash.as_slice(),
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("expected to read the vote")
}

/// A contest two contenders joined, the first with `rank` as `U64(5)`, the second as `U8(5)`.
struct Contest {
    platform: TempPlatform<MockCoreRPCLike>,
    contract: DataContract,
    second_contender: Identifier,
    /// The poll the contenders name: `rank` as its property's type keys it
    poll: ContestedDocumentResourceVotePoll,
    /// The poll of the same contest with `rank` in another integer form
    alternate_poll: ContestedDocumentResourceVotePoll,
    /// What the poll's prefunded balance held once the first contender had joined
    balance_after_first_contender: Option<Credits>,
    /// The time of the block the second contender joined in
    time_ms: TimestampMillis,
}

async fn open_contest(platform_version: &PlatformVersion) -> Contest {
    let mut platform = platform_at(platform_version.protocol_version);
    let mut rng = StdRng::seed_from_u64(0x5EED_0F02);
    let owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contender_a = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let contender_b = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let contract = setup_contract(
        &platform.drive,
        INTEGER_CONTRACT,
        None,
        Some(owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(platform_version),
    );

    let (rank, alternate_rank) = rank_forms(&contract);
    let alternate_poll = poll(&contract, alternate_rank);
    let poll = poll(&contract, rank);
    assert_ne!(
        poll.unique_id().expect("expected a poll id"),
        alternate_poll.unique_id().expect("expected a poll id"),
    );

    let start: TimestampMillis = 1_000_000;
    let a = contender_transition(
        &contract,
        &contender_a,
        Value::U64(5),
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, a, start, platform_version);
    let balance_after_first_contender = balance(&platform, &poll, platform_version);
    let b = contender_transition(
        &contract,
        &contender_b,
        Value::U8(5),
        &mut rng,
        platform_version,
    )
    .await;
    process_valid(&platform, b, start + 1000, platform_version);

    Contest {
        platform,
        contract,
        second_contender: contender_b.0.id(),
        poll,
        alternate_poll,
        balance_after_first_contender,
        time_ms: start + 1000,
    }
}

#[tokio::test]
async fn should_name_one_poll_when_contenders_write_an_index_value_in_two_forms() {
    let platform_version = PlatformVersion::latest();
    let contest = open_contest(platform_version).await;

    let first = contest
        .balance_after_first_contender
        .expect("expected the first contender to fund the poll");
    let both = balance(&contest.platform, &contest.poll, platform_version)
        .expect("expected the poll to keep its balance");
    assert!(
        both > first,
        "the second contender funds the poll of the first: {first} then {both}"
    );
    assert_eq!(
        balance(&contest.platform, &contest.alternate_poll, platform_version),
        None
    );
}

#[tokio::test]
async fn should_refuse_a_vote_naming_the_poll_by_another_form_of_its_index_values() {
    let platform_version = PlatformVersion::latest();
    let mut contest = open_contest(platform_version).await;
    let (pro_tx_hash, _, signer, voting_key) =
        setup_masternode_voting_identity(&mut contest.platform, 0xA11CE, platform_version);

    let vote = vote_transition(
        contest.alternate_poll.clone(),
        ResourceVoteChoice::TowardsIdentity(contest.second_contender),
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await;
    // No balance is held under another poll of the contest
    assert_matches!(
        process(
            &contest.platform,
            &[vote],
            contest.time_ms + 1000,
            0,
            platform_version,
        )
        .as_slice(),
        [StateTransitionExecutionResult::UnpaidConsensusError(
            ConsensusError::StateError(StateError::PrefundedSpecializedBalanceNotFoundError(_))
        )]
    );
    assert!(!vote_is_stored(
        &contest,
        contest.second_contender,
        pro_tx_hash,
        platform_version
    ));

    let vote = vote_transition(
        contest.poll.clone(),
        ResourceVoteChoice::TowardsIdentity(contest.second_contender),
        &signer,
        pro_tx_hash,
        &voting_key,
        1,
        platform_version,
    )
    .await;
    process_valid(
        &contest.platform,
        vote,
        contest.time_ms + 2000,
        platform_version,
    );
    assert!(vote_is_stored(
        &contest,
        contest.second_contender,
        pro_tx_hash,
        platform_version
    ));
}

/// A contest opened at protocol version 14 with `rank` in a form the latest protocol version
/// writes another way is joined, after the upgrade, by a contender sending `rank` the same way,
/// then ended. The second contender funds the poll the contest was opened under, the contest
/// runs to the end of its poll, and nothing is left under either poll once it ended. A contest
/// resolved without locking runs to the end of its join window until a second contender joins.
async fn join_after_the_upgrade_a_contest_opened_at_protocol_version_14(
    contract_path: &str,
    no_locking: bool,
) {
    let version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let latest = PlatformVersion::latest();
    let mut platform = platform_at(14);
    let mut rng = StdRng::seed_from_u64(0x5EED_0F03);
    let owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contender_a = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let contender_b = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let contract = setup_contract(
        &platform.drive,
        contract_path,
        None,
        Some(owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(version_14),
    );

    let (rank, alternate_rank) = rank_forms(&contract);
    // The poll the contest is opened under at protocol version 14, which takes `rank` as sent
    let opening_poll = poll(&contract, alternate_rank.clone());
    // The poll the latest protocol version names for a contest it opens
    let latest_poll = poll(&contract, rank);
    let opening_poll_id = opening_poll.unique_id().expect("expected a poll id");

    let join_window = latest
        .dpp
        .validation
        .voting
        .allow_other_contenders_time_mainnet_ms;
    let poll_duration = latest
        .dpp
        .voting_versions
        .default_vote_poll_time_duration_mainnet_ms;

    let start: TimestampMillis = 1_000_000;
    let a = contender_transition(
        &contract,
        &contender_a,
        alternate_rank.clone(),
        &mut rng,
        version_14,
    )
    .await;
    process_valid_in_epoch(&platform, a, start, 0, version_14);
    let balance_after_first_contender = balance(&platform, &opening_poll, version_14)
        .expect("expected the first contender to fund the poll");
    let first_end = if no_locking {
        start + join_window
    } else {
        start + poll_duration
    };
    assert_eq!(
        vote_poll_end_dates(&platform.drive, version_14),
        BTreeMap::from([(first_end, BTreeSet::from([opening_poll_id]))])
    );

    // The first block of the latest protocol version, in the next epoch
    let platform_state = platform.state.load();
    let transaction = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &platform_state,
            &mut PlatformState::clone(&platform_state),
            &block(start + 500, 1),
            &transaction,
            14,
            latest,
        )
        .expect("expected the first block of the latest protocol version");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected to commit the transaction");
    let mut upgraded_state = platform_state.as_ref().clone();
    upgraded_state.set_current_protocol_version_in_consensus(latest.protocol_version);
    upgraded_state.set_next_epoch_protocol_version(latest.protocol_version);
    platform.state.store(Arc::new(upgraded_state));

    let b = contender_transition(&contract, &contender_b, alternate_rank, &mut rng, latest).await;
    process_valid_in_epoch(&platform, b, start + 1000, 1, latest);

    let balance_after_second_contender =
        balance(&platform, &opening_poll, latest).expect("expected the poll to keep its balance");
    assert!(
        balance_after_second_contender > balance_after_first_contender,
        "the second contender funds the poll the contest was opened under: \
         {balance_after_first_contender} then {balance_after_second_contender}"
    );
    assert_eq!(balance(&platform, &latest_poll, latest), None);
    assert_eq!(
        vote_poll_end_dates(&platform.drive, latest),
        BTreeMap::from([(start + poll_duration, BTreeSet::from([opening_poll_id]))])
    );

    end_polls_at(&platform, start + poll_duration, 1, latest);

    assert_eq!(balance(&platform, &opening_poll, latest), None);
    assert_eq!(balance(&platform, &latest_poll, latest), None);
    assert_eq!(
        vote_poll_end_dates(&platform.drive, latest),
        BTreeMap::new()
    );
}

#[tokio::test]
async fn should_keep_the_poll_of_a_contest_opened_before_the_upgrade() {
    join_after_the_upgrade_a_contest_opened_at_protocol_version_14(INTEGER_CONTRACT, false).await;
}

#[tokio::test]
async fn should_keep_the_poll_of_a_contest_without_locking_opened_before_the_upgrade() {
    join_after_the_upgrade_a_contest_opened_at_protocol_version_14(
        NO_LOCKING_INTEGER_CONTRACT,
        true,
    )
    .await;
}

/// Protocol version 14 compares the poll a contested create names with the poll its document
/// type expects as whole values, and a poll holding a NaN is not equal to itself: such a create
/// is refused with `DocumentContestIndexMismatchError`, and stays so.
#[tokio::test]
async fn should_refuse_a_contested_create_with_a_nan_index_value_as_protocol_version_14_does() {
    let version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
    let mut platform = platform_at(14);
    let mut rng = StdRng::seed_from_u64(0x5EED_0F04);
    let owner = setup_identity(&mut platform, rng.gen(), dash_to_credits!(0.5));
    let contender = setup_identity(&mut platform, rng.gen(), dash_to_credits!(1.0));
    let contract = setup_contract(
        &platform.drive,
        NUMBER_CONTRACT,
        None,
        Some(owner.0.id().to_buffer()),
        None::<fn(&mut DataContract)>,
        None,
        Some(version_14),
    );

    let create = contender_transition(
        &contract,
        &contender,
        Value::Float(f64::NAN),
        &mut rng,
        version_14,
    )
    .await;
    assert_matches!(
        process(&platform, &[create], 1_000_000, 0, version_14).as_slice(),
        [StateTransitionExecutionResult::PaidConsensusError {
            error: ConsensusError::StateError(StateError::DocumentContestIndexMismatchError(_)),
            ..
        }]
    );
}
