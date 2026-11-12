//! A contest is stored at the path of its index keys, and its poll is named by an id hashed from
//! its index values. An integer sent as `U64(5)` and as `U8(5)` has one key. From protocol
//! version 15 a contested create writes every index value the one way its property's type keys
//! it, so the contenders of one contest name one poll. A masternode vote names its poll by the
//! values it sends.

use crate::execution::validation::state_transition::state_transitions::tests::{
    setup_identity, setup_masternode_voting_identity,
};
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::dash_to_credits;
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
use drive::util::test_helpers::setup_contract;
use platform_version::version::PlatformVersion;
use rand::prelude::StdRng;
use rand::{Rng, SeedableRng};
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;

/// The DPNS-shaped fixture whose contested index ends in the integer property `rank`.
const INTEGER_CONTRACT: &str =
    "tests/supporting_files/contract/dpns/dpns-contract-contested-unique-index-with-integer.json";
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

/// Processes transitions in a block at `time_ms` and returns how each one ended.
fn process(
    platform: &TempPlatform<MockCoreRPCLike>,
    transitions: &[Vec<u8>],
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) -> Vec<String> {
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
    processing_result
        .execution_results()
        .iter()
        .map(|result| format!("{:?}", result))
        .collect()
}

fn process_valid(
    platform: &TempPlatform<MockCoreRPCLike>,
    transition: Vec<u8>,
    time_ms: TimestampMillis,
    platform_version: &PlatformVersion,
) {
    let results = process(platform, &[transition], time_ms, platform_version);
    assert!(
        results[0].starts_with("SuccessfulExecution"),
        "the transition should pass: {}",
        results[0]
    );
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
    let mut platform = TestPlatformBuilder::new()
        .with_latest_protocol_version()
        .build_with_mock_rpc()
        .set_genesis_state();
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
            platform_version,
        )
        .expect("expected the values of the contest")
        .pop()
        .expect("expected the rank");
    let alternate_rank = [Value::U8(5), Value::U64(5)]
        .into_iter()
        .find(|form| form != &rank)
        .expect("expected another form of the rank");
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
    let result = process(
        &contest.platform,
        &[vote],
        contest.time_ms + 1000,
        platform_version,
    )
    .remove(0);
    assert!(
        result.starts_with("UnpaidConsensusError")
            && result.contains("PrefundedSpecializedBalanceNotFoundError"),
        "no balance is held under another id of the contest: {result}"
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
