use crate::config::{PlatformConfig, PlatformTestConfig};
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0, BlockExecutionContextV0Getters, BlockExecutionContextV0OwnedGetters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
use crate::execution::types::block_state_info::BlockStateInfo;
use crate::mimic::test_quorum::TestQuorumInfo;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use crate::platform_types::validator_set::{ValidatorSet, ValidatorSetExt};
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::bls::PublicKey as BlsPublicKey;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{BlockHash, ProTxHash, QuorumHash, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNState, ExtendedQuorumDetails, ExtendedQuorumListResult, MasternodeListDiff,
    MasternodeListItem, MasternodeType, QuorumInfoResult,
};
use dpp::dashcore_rpc::jsonrpc::error::Error as JsonRpcError;
use dpp::dashcore_rpc::Error as CoreRpcError;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;

/// The Core height the quorum validating now was mined at.
const CURRENT_QUORUM_HEIGHT: u32 = 90;
/// The Core height the quorum the chain rotates into was mined at.
const NEXT_QUORUM_HEIGHT: u32 = 100;
/// The chain locked height of the blocks the dispatcher tests run.
const BLOCK_CORE_HEIGHT: u32 = 110;

/// Core's answer to `quorum info`, by quorum hash.
type CoreAnswer = Arc<dyn Fn(&QuorumHash) -> Result<QuorumInfoResult, CoreRpcError> + Send + Sync>;

/// An evonode Platform can make a validator of: it has a platform node id and ports.
fn evonode(pro_tx_hash: ProTxHash) -> MasternodeListItem {
    let index = pro_tx_hash.to_byte_array()[0];
    #[allow(deprecated)]
    MasternodeListItem {
        node_type: MasternodeType::Evo,
        pro_tx_hash,
        collateral_hash: Txid::from_byte_array([index; 32]),
        collateral_index: 0,
        collateral_address: Some([index; 20]),
        operator_reward: 0.0,
        state: DMNState {
            service: SocketAddr::from(([192, 0, 2, index], 9999)),
            registered_height: 1,
            pose_revived_height: None,
            pose_ban_height: None,
            revocation_reason: 0,
            owner_address: Some([index; 20]),
            voting_address: [index; 20],
            payout_address: Some([index; 20]),
            payouts: None,
            pub_key_operator: vec![index; 48],
            operator_payout_address: None,
            platform_node_id: Some([index; 20]),
            legacy_platform_p2p_port: Some(26656),
            legacy_platform_http_port: Some(443),
            addresses: None,
        },
    }
}

/// A quorum of three members named by `first_member`, `first_member + 1` and `first_member + 2`.
fn quorum(
    core_height: u32,
    quorum_hash_byte: u8,
    first_member: u8,
    rng: &mut StdRng,
) -> TestQuorumInfo {
    TestQuorumInfo::from_quorum_hash_and_pro_tx_hashes(
        core_height,
        QuorumHash::from_byte_array([quorum_hash_byte; 32]),
        None,
        (first_member..first_member + 3)
            .map(|index| ProTxHash::from_byte_array([index; 32]))
            .collect(),
        rng,
    )
}

/// The quorum validating now and the quorum the chain rotates into.
fn current_and_next() -> (TestQuorumInfo, TestQuorumInfo) {
    let mut rng = StdRng::seed_from_u64(1);
    let current = quorum(CURRENT_QUORUM_HEIGHT, 0xa0, 1, &mut rng);
    let next = quorum(NEXT_QUORUM_HEIGHT, 0xb0, 4, &mut rng);
    (current, next)
}

/// Core's `quorum info` for `quorum` when Core has its verification vector.
fn info_with_key_shares(quorum: &TestQuorumInfo) -> QuorumInfoResult {
    let mut info = QuorumInfoResult::from(quorum);
    info.height = quorum.core_height;
    info
}

/// Core's `quorum info` for `quorum` when Core has not built its verification vector: every
/// member is valid, and none has a public key share.
fn info_without_key_shares(quorum: &TestQuorumInfo) -> QuorumInfoResult {
    let mut info = info_with_key_shares(quorum);
    for member in &mut info.members {
        member.pub_key_share = None;
    }
    info
}

fn extended_details(creation_height: u32) -> ExtendedQuorumDetails {
    ExtendedQuorumDetails {
        creation_height,
        quorum_index: None,
        mined_block_hash: BlockHash::all_zeros(),
        num_valid_members: 3,
        health_ratio: 1.0,
    }
}

/// Core answers both of Drive's `quorum info` calls with `answer`.
fn expect_quorum_info(platform: &mut TempPlatform<MockCoreRPCLike>, answer: CoreAnswer) {
    let first_sight = Arc::clone(&answer);
    platform
        .core_rpc
        .expect_get_quorum_info()
        .returning(move |_, quorum_hash, _| first_sight(quorum_hash));
    platform
        .core_rpc
        .expect_get_quorum_info_with_short_timeout()
        .returning(move |_, quorum_hash| answer(quorum_hash));
}

/// A platform whose Core lists `current` and `next` as validator quorums at every height,
/// reports no masternode list changes and Core V24 active. Core answers `quorum info` for
/// `current` with its key shares, and for `next` without them the first time it is asked and
/// with them after that.
fn platform_with_core(
    platform_version: &PlatformVersion,
    current: &TestQuorumInfo,
    next: &TestQuorumInfo,
) -> TempPlatform<MockCoreRPCLike> {
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    // Without the default answers, which would shadow the ones set here.
    platform.core_rpc.checkpoint();

    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(|base_height, block_height| {
            Ok(MasternodeListDiff {
                base_height: base_height.unwrap_or_default(),
                block_height,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![],
            })
        });
    platform
        .core_rpc
        .expect_get_coinbase_payload_version()
        .returning(|_| Ok(Some(4)));

    let validator_quorum_type = platform.config.validator_set.quorum_type;
    let other_quorum_types = [
        platform.config.chain_lock.quorum_type,
        platform.config.instant_lock.quorum_type,
    ];
    let listed = HashMap::from([
        (current.quorum_hash, extended_details(current.core_height)),
        (next.quorum_hash, extended_details(next.core_height)),
    ]);
    platform
        .core_rpc
        .expect_get_quorum_listextended()
        .returning(move |_| {
            let mut quorums_by_type: HashMap<_, HashMap<QuorumHash, ExtendedQuorumDetails>> =
                other_quorum_types
                    .iter()
                    .map(|quorum_type| (*quorum_type, HashMap::new()))
                    .collect();
            quorums_by_type.insert(validator_quorum_type, listed.clone());
            Ok(ExtendedQuorumListResult { quorums_by_type })
        });

    let current_hash = current.quorum_hash;
    let current_info = info_with_key_shares(current);
    let next_info_first = info_without_key_shares(next);
    let next_info_later = info_with_key_shares(next);
    let next_answers = AtomicUsize::new(0);
    expect_quorum_info(
        &mut platform,
        Arc::new(move |quorum_hash| {
            if *quorum_hash == current_hash {
                Ok(current_info.clone())
            } else if next_answers.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(next_info_first.clone())
            } else {
                Ok(next_info_later.clone())
            }
        }),
    );
    platform
}

/// A platform at `platform_version` that stores its state and whose Core answers `quorum info`
/// with `answer`.
fn platform_answering(
    platform_version: &PlatformVersion,
    answer: CoreAnswer,
) -> TempPlatform<MockCoreRPCLike> {
    let mut platform = TestPlatformBuilder::new()
        .with_config(PlatformConfig {
            testing_configs: PlatformTestConfig {
                store_platform_state: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_initial_protocol_version(platform_version.protocol_version)
        .build_with_mock_rpc()
        .set_genesis_state();
    platform.core_rpc.checkpoint();
    expect_quorum_info(&mut platform, answer);
    platform
}

/// A Core whose connection fails with `kind`.
fn unreachable_core(kind: std::io::ErrorKind) -> CoreRpcError {
    CoreRpcError::JsonRpc(JsonRpcError::Transport(Box::new(std::io::Error::new(
        kind,
        "Core did not answer",
    ))))
}

fn extended_block_info(
    height: u64,
    core_height: u32,
    quorum_hash: QuorumHash,
    proposer: ProTxHash,
) -> ExtendedBlockInfo {
    ExtendedBlockInfoV0 {
        basic_info: BlockInfo {
            height,
            core_height,
            ..Default::default()
        },
        app_hash: [0; 32],
        quorum_hash: quorum_hash.to_byte_array(),
        block_id_hash: [0; 32],
        proposer_pro_tx_hash: proposer.to_byte_array(),
        signature: [0; 96],
        round: 0,
    }
    .into()
}

/// Moves the state to a block whose previous block was committed at `core_height`, validated
/// by `quorum_hash` and proposed by `proposer`.
fn commit_block(
    block_state: &mut PlatformState,
    core_height: u32,
    quorum_hash: QuorumHash,
    proposer: ProTxHash,
) {
    let height = block_state.last_committed_block_height() + 1;
    block_state.set_last_committed_block_info(Some(extended_block_info(
        height,
        core_height,
        quorum_hash,
        proposer,
    )));
}

/// Updates the core info of a block at chain locked height `core_height`, as
/// `run_block_proposal` does.
fn update_core_info(
    platform: &TempPlatform<MockCoreRPCLike>,
    block_state: &mut PlatformState,
    core_height: u32,
    platform_version: &PlatformVersion,
) {
    let transaction = platform.drive.grove.start_transaction();
    platform
        .update_core_info(
            None,
            block_state,
            core_height,
            false,
            &BlockInfo {
                height: block_state.last_committed_block_height() + 1,
                core_height,
                ..Default::default()
            },
            &transaction,
            platform_version,
        )
        .expect("expected to update the core info");
}

fn block_execution_context(
    block_platform_state: PlatformState,
    proposer: ProTxHash,
    core_height: u32,
) -> BlockExecutionContext {
    BlockExecutionContext::V0(BlockExecutionContextV0 {
        block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
            height: block_platform_state.last_committed_block_height() + 1,
            round: 0,
            block_time_ms: 1_000_000,
            previous_block_time_ms: None,
            proposer_pro_tx_hash: proposer.to_byte_array(),
            core_chain_locked_height: core_height,
            block_hash: None,
            app_hash: None,
        }),
        epoch_info: EpochInfo::V0(EpochInfoV0 {
            current_epoch_index: 0,
            previous_epoch_index: None,
            is_epoch_change: false,
        }),
        unsigned_withdrawal_transactions: UnsignedWithdrawalTxs::default(),
        block_address_balance_changes: BTreeMap::new(),
        block_platform_state,
        proposer_results: None,
    })
}

/// The validator set Drive keeps for `quorum`, with or without the members' key shares.
fn validator_set(quorum: &TestQuorumInfo, with_key_shares: bool) -> ValidatorSet {
    let mut validator_set = ValidatorSetV0::from(quorum);
    if !with_key_shares {
        for validator in validator_set.members.values_mut() {
            validator.public_key = None;
        }
    }
    ValidatorSet::V0(validator_set)
}

/// A committed state validated by `current`, whose last block `current`'s first member
/// proposed, that also holds `next`, ordered newest first as Drive orders them. It is stored, so
/// nothing in it is left to store.
fn committed_state(
    platform: &TempPlatform<MockCoreRPCLike>,
    current: &TestQuorumInfo,
    next: &TestQuorumInfo,
    next_with_key_shares: bool,
) -> PlatformState {
    let mut state = platform.state.load().as_ref().clone();
    state.set_validator_sets(IndexMap::from([
        (next.quorum_hash, validator_set(next, next_with_key_shares)),
        (current.quorum_hash, validator_set(current, true)),
    ]));
    state.set_current_validator_set_quorum_hash(current.quorum_hash);
    state.set_last_committed_block_info(Some(extended_block_info(
        5,
        BLOCK_CORE_HEIGHT,
        current.quorum_hash,
        current.validator_set[0].pro_tx_hash,
    )));
    let platform_version = PlatformVersion::get(state.current_protocol_version_in_consensus())
        .expect("expected a known protocol version");
    platform
        .store_platform_state(&state, None, platform_version)
        .expect("expected to store the committed state");
    state.mark_saved();
    state
}

/// Runs the end of a block proposed by `proposer` with `block_state` on top of `committed`:
/// the validator set update it sends Tenderdash and the block state it leaves.
fn run_block_end(
    platform: &TempPlatform<MockCoreRPCLike>,
    committed: &PlatformState,
    block_state: PlatformState,
    proposer: ProTxHash,
    platform_version: &PlatformVersion,
) -> (Option<ValidatorSetUpdate>, BlockExecutionContext) {
    let mut context = block_execution_context(block_state, proposer, BLOCK_CORE_HEIGHT);
    let update = platform
        .validator_set_update(
            proposer.to_byte_array(),
            committed,
            &mut context,
            platform_version,
        )
        .expect("expected the validator set update to succeed");
    (update, context)
}

/// The key share each validator of `update` is sent with, by proTxHash.
fn sent_keys(update: &ValidatorSetUpdate) -> BTreeMap<Vec<u8>, Option<Vec<u8>>> {
    update
        .validator_updates
        .iter()
        .map(|validator| {
            let key = validator.pub_key.as_ref().map(|key| match &key.sum {
                Some(Bls12381(bytes)) => bytes.clone(),
                other => panic!("expected a BLS key share, got {other:?}"),
            });
            (validator.pro_tx_hash.clone(), key)
        })
        .collect()
}

/// Every member of `quorum` with its key share, or without one, as `sent_keys` lists them.
fn quorum_keys(
    quorum: &TestQuorumInfo,
    with_key_shares: bool,
) -> BTreeMap<Vec<u8>, Option<Vec<u8>>> {
    quorum
        .validator_set
        .iter()
        .map(|member| {
            (
                member.pro_tx_hash.to_byte_array().to_vec(),
                with_key_shares.then(|| member.public_key.to_bytes().to_vec()),
            )
        })
        .collect()
}

/// The key share each member of `quorum_hash`'s set in `state` holds.
fn held_keys(
    state: &PlatformState,
    quorum_hash: &QuorumHash,
) -> BTreeMap<ProTxHash, Option<BlsPublicKey>> {
    state
        .validator_sets()
        .get(quorum_hash)
        .expect("expected the quorum's validator set")
        .members()
        .iter()
        .map(|(pro_tx_hash, validator)| (*pro_tx_hash, validator.public_key))
        .collect()
}

/// Every member of `quorum` with its key share, or without one, as `held_keys` lists them.
fn quorum_held_keys(
    quorum: &TestQuorumInfo,
    with_key_shares: bool,
) -> BTreeMap<ProTxHash, Option<BlsPublicKey>> {
    quorum
        .validator_set
        .iter()
        .map(|member| {
            (
                member.pro_tx_hash,
                with_key_shares.then_some(member.public_key),
            )
        })
        .collect()
}

/// A Core that has not built a quorum's verification vector answers `quorum info` without a
/// public key share for any member: a member that missed contributions during the DKG until it
/// recovers the vector from the others, or a node outside the quorum until it syncs the vector.
/// Tenderdash verifies votes with the members' key shares, and takes a quorum's keys from the
/// set Drive sends when the chain rotates into the quorum.
///
/// Drive first sees the next quorum while its Core has no key shares for it; two chain locked
/// heights later Core has them. The block that rotates into that quorum must send Tenderdash
/// every member's key share, and keep them for the blocks after it.
#[test]
fn should_send_the_key_shares_core_has_now_when_rotating_into_a_quorum_first_seen_without_them() {
    let platform_version = PlatformVersion::latest();
    let (current, next) = current_and_next();
    let platform = platform_with_core(platform_version, &current, &next);

    let mut block_state = platform.state.load().as_ref().clone();
    for member in current.validator_set.iter().chain(&next.validator_set) {
        block_state.insert_masternode(evonode(member.pro_tx_hash));
    }
    block_state.set_current_validator_set_quorum_hash(current.quorum_hash);
    let first_member = current.validator_set[0].pro_tx_hash;
    let last_member = current.validator_set[2].pro_tx_hash;

    // The block that first sees both quorums, while Core has no key shares for the next one.
    commit_block(
        &mut block_state,
        NEXT_QUORUM_HEIGHT,
        current.quorum_hash,
        first_member,
    );
    update_core_info(
        &platform,
        &mut block_state,
        NEXT_QUORUM_HEIGHT + 1,
        platform_version,
    );
    let first_seen = block_state
        .validator_sets()
        .get(&next.quorum_hash)
        .expect("expected the validator set of the listed quorum");
    assert!(
        first_seen
            .members()
            .values()
            .all(|validator| validator.public_key.is_none()),
        "Core had no key share for any member when the quorum was first seen"
    );

    // Core has since built the verification vector; the chain locked height moves on.
    commit_block(
        &mut block_state,
        NEXT_QUORUM_HEIGHT + 1,
        current.quorum_hash,
        first_member,
    );
    update_core_info(
        &platform,
        &mut block_state,
        NEXT_QUORUM_HEIGHT + 2,
        platform_version,
    );

    // The current quorum's last member proposes: the chain rotates into the next quorum.
    let core_height = NEXT_QUORUM_HEIGHT + 3;
    commit_block(
        &mut block_state,
        NEXT_QUORUM_HEIGHT + 2,
        current.quorum_hash,
        first_member,
    );
    let last_committed_state = block_state.clone();
    update_core_info(&platform, &mut block_state, core_height, platform_version);
    let mut context = block_execution_context(block_state, last_member, core_height);

    let update = platform
        .validator_set_update(
            last_member.to_byte_array(),
            &last_committed_state,
            &mut context,
            platform_version,
        )
        .expect("expected the validator set update to succeed")
        .expect("expected a rotation after the current quorum's last member proposed");
    assert_eq!(
        update.quorum_hash,
        next.quorum_hash.to_byte_array().to_vec(),
        "expected the rotation to go to the next quorum"
    );

    assert_eq!(
        sent_keys(&update),
        quorum_keys(&next, true),
        "Tenderdash must get every member's key share Core has now"
    );

    // The blocks after this one keep them, so a restart or a later rotation does not lose them.
    assert_eq!(
        held_keys(context.block_platform_state(), &next.quorum_hash),
        quorum_held_keys(&next, true)
    );
}

/// Whether Core had the key shares changes only the keys sent, at every validator set update
/// generation: the rotation, the quorum and the validators sent are the same, and after the
/// block is committed the next block sends no update on either node. The filled keys are part
/// of the state the next block compares against, so they do not read as a changed set.
#[test]
fn should_decide_the_blocks_alike_whether_or_not_core_had_the_key_shares() {
    // Protocol versions 1, 4 and 15 select validator set update generations 0, 1 and 2.
    for protocol_version in [1, 4, 15] {
        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected a known protocol version");
        let (current, next) = current_and_next();
        let last_member = current.validator_set[2].pro_tx_hash;
        let next_proposer = next.validator_set[0].pro_tx_hash;

        let with_shares = info_with_key_shares(&next);
        let without_shares = info_without_key_shares(&next);
        let core_with_shares =
            platform_answering(platform_version, Arc::new(move |_| Ok(with_shares.clone())));
        let core_without_shares = platform_answering(
            platform_version,
            Arc::new(move |_| Ok(without_shares.clone())),
        );

        let mut outcomes = Vec::new();
        for platform in [&core_with_shares, &core_without_shares] {
            let committed = committed_state(platform, &current, &next, false);
            let (update, context) = run_block_end(
                platform,
                &committed,
                committed.clone(),
                last_member,
                platform_version,
            );
            let update = update.unwrap_or_else(|| {
                panic!("protocol version {protocol_version}: expected a rotation")
            });

            let transaction = platform.drive.grove.start_transaction();
            platform
                .update_state_cache(
                    extended_block_info(6, BLOCK_CORE_HEIGHT, current.quorum_hash, last_member),
                    context.block_platform_state_owned(),
                    &transaction,
                    platform_version,
                )
                .expect("expected to commit the block");
            let committed = platform.state.load().as_ref().clone();
            assert_eq!(
                committed.current_validator_set_quorum_hash(),
                next.quorum_hash,
                "protocol version {protocol_version}"
            );
            // What a restart reads back holds the same keys as the committed state.
            let stored = Platform::<MockCoreRPCLike>::fetch_platform_state(
                &platform.drive,
                Some(&transaction),
                platform_version,
            )
            .expect("expected to read the stored state")
            .expect("expected a stored state");
            assert_eq!(
                held_keys(&stored, &next.quorum_hash),
                held_keys(&committed, &next.quorum_hash),
                "protocol version {protocol_version}: the stored set must hold the committed keys"
            );

            let (next_update, _) = run_block_end(
                platform,
                &committed,
                committed.clone(),
                next_proposer,
                platform_version,
            );
            assert_eq!(
                next_update, None,
                "protocol version {protocol_version}: the block after the rotation sends no update"
            );
            outcomes.push((update, committed));
        }

        let (filled, filled_state) = &outcomes[0];
        let (unfilled, unfilled_state) = &outcomes[1];
        assert_eq!(filled.quorum_hash, unfilled.quorum_hash);
        assert_eq!(filled.threshold_public_key, unfilled.threshold_public_key);
        assert_eq!(
            sent_keys(filled).keys().collect::<Vec<_>>(),
            sent_keys(unfilled).keys().collect::<Vec<_>>()
        );
        assert_eq!(
            sent_keys(filled),
            quorum_keys(&next, true),
            "protocol version {protocol_version}"
        );
        assert_eq!(
            sent_keys(unfilled),
            quorum_keys(&next, false),
            "protocol version {protocol_version}"
        );
        assert_eq!(
            held_keys(filled_state, &next.quorum_hash),
            quorum_held_keys(&next, true),
            "protocol version {protocol_version}"
        );
        assert_eq!(
            held_keys(unfilled_state, &next.quorum_hash),
            quorum_held_keys(&next, false),
            "protocol version {protocol_version}"
        );
    }
}

/// An update for the quorum Tenderdash holds, here sent because a validator's address changed,
/// is left as the versioned method made it, and Core is not asked. Tenderdash merges such an
/// update into the set it has, keeping that set's keys for validators the update leaves out, so
/// filling it could mix validators with and without keys.
#[test]
fn should_leave_an_update_for_the_quorum_tenderdash_holds_as_it_is() {
    let platform_version = PlatformVersion::latest();
    let (current, next) = current_and_next();
    let with_shares = info_with_key_shares(&next);
    let asked = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&asked);
    let platform = platform_answering(
        platform_version,
        Arc::new(move |_| {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(with_shares.clone())
        }),
    );

    // The next quorum validates now, without keys; its first member proposed the last block.
    let mut committed = committed_state(&platform, &current, &next, false);
    committed.set_current_validator_set_quorum_hash(next.quorum_hash);
    committed.set_last_committed_block_info(Some(extended_block_info(
        5,
        BLOCK_CORE_HEIGHT,
        next.quorum_hash,
        next.validator_set[0].pro_tx_hash,
    )));
    let moved = next.validator_set[1].pro_tx_hash;
    let mut block_state = committed.clone();
    block_state
        .validator_set_mut(&next.quorum_hash)
        .expect("expected the next quorum")
        .members_mut()
        .get_mut(&moved)
        .expect("expected the member")
        .node_ip = "198.51.100.7".to_string();
    let expected = block_state
        .validator_sets()
        .get(&next.quorum_hash)
        .expect("expected the next quorum")
        .to_update();

    let (update, context) =
        run_block_end(&platform, &committed, block_state, moved, platform_version);

    assert_eq!(update, Some(expected));
    assert_eq!(
        held_keys(context.block_platform_state(), &next.quorum_hash),
        quorum_held_keys(&next, false)
    );
    assert_eq!(asked.load(Ordering::SeqCst), 0, "Core is not asked");
}

/// Whatever keeps Core from giving every member's key share leaves the update as the versioned
/// method made it and the block state untouched, and the block goes on: the shares are not
/// consensus data, so their absence must never fail a block.
#[test]
fn should_leave_everything_unchanged_when_core_cannot_give_every_key_share() {
    let platform_version = PlatformVersion::latest();
    let (current, next) = current_and_next();
    let mut rng = StdRng::seed_from_u64(2);
    // The same quorum hash and members, with another quorum public key.
    let other_quorum = quorum(NEXT_QUORUM_HEIGHT, 0xb0, 4, &mut rng);

    let mut partial = info_with_key_shares(&next);
    partial.members[1].pub_key_share = None;
    let mut invalid_share = info_with_key_shares(&next);
    invalid_share.members[0].pub_key_share = Some(vec![1; 10]);
    let mut other_height = info_with_key_shares(&next);
    other_height.height += 1;
    let cases: Vec<(&str, Option<QuorumInfoResult>)> = vec![
        ("Core unreachable or timed out", None),
        (
            "Core without the verification vector",
            Some(info_without_key_shares(&next)),
        ),
        ("a share missing", Some(partial)),
        ("a share that is not a public key", Some(invalid_share)),
        (
            "another quorum public key",
            Some(info_with_key_shares(&other_quorum)),
        ),
        ("another quorum height", Some(other_height)),
    ];

    for (case, answer) in cases {
        let platform = platform_answering(
            platform_version,
            Arc::new(move |_| {
                answer
                    .clone()
                    .ok_or_else(|| unreachable_core(std::io::ErrorKind::TimedOut))
            }),
        );
        let committed = committed_state(&platform, &current, &next, false);

        let (update, context) = run_block_end(
            &platform,
            &committed,
            committed.clone(),
            current.validator_set[2].pro_tx_hash,
            platform_version,
        );

        let update = update.unwrap_or_else(|| panic!("{case}: expected a rotation"));
        assert_eq!(
            update,
            validator_set(&next, false).to_update(),
            "{case}: the update must be the one the versioned method made"
        );
        assert_eq!(
            held_keys(context.block_platform_state(), &next.quorum_hash),
            quorum_held_keys(&next, false),
            "{case}"
        );
        assert!(
            context
                .block_platform_state()
                .validator_set_changes
                .is_empty(),
            "{case}: no stored validator set may be rewritten"
        );
        assert!(
            platform.quorum_key_shares.get(&next.quorum_hash).is_none(),
            "{case}: nothing may be kept"
        );
    }
}

/// FinalizeBlock executes the committed block again when it was not the last one processed,
/// while Tenderdash keeps the update the first execution sent. The second execution must send
/// and keep the same keys even when Core cannot answer then.
#[test]
fn should_send_the_same_key_shares_when_the_block_is_executed_again() {
    let platform_version = PlatformVersion::latest();
    let (current, next) = current_and_next();
    let with_shares = info_with_key_shares(&next);
    let asked = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&asked);
    let platform = platform_answering(
        platform_version,
        Arc::new(move |_| {
            if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(with_shares.clone())
            } else {
                Err(unreachable_core(std::io::ErrorKind::ConnectionRefused))
            }
        }),
    );
    let committed = committed_state(&platform, &current, &next, false);
    let proposer = current.validator_set[2].pro_tx_hash;

    let (first, _) = run_block_end(
        &platform,
        &committed,
        committed.clone(),
        proposer,
        platform_version,
    );
    let (again, context) = run_block_end(
        &platform,
        &committed,
        committed.clone(),
        proposer,
        platform_version,
    );

    let first = first.expect("expected a rotation");
    let again = again.expect("expected the same rotation");
    assert_eq!(sent_keys(&first), quorum_keys(&next, true));
    assert_eq!(again, first);
    assert_eq!(
        held_keys(context.block_platform_state(), &next.quorum_hash),
        quorum_held_keys(&next, true)
    );
    assert_eq!(asked.load(Ordering::SeqCst), 1, "Core is asked only once");
}

/// An update that already has every key is sent as the versioned method made it, without
/// asking Core.
#[test]
fn should_not_ask_core_when_the_update_has_every_key_share() {
    let platform_version = PlatformVersion::latest();
    let (current, next) = current_and_next();
    let mut platform = TestPlatformBuilder::new()
        .build_with_mock_rpc()
        .set_genesis_state();
    platform.core_rpc.checkpoint();
    platform.core_rpc.expect_get_quorum_info().never();
    platform
        .core_rpc
        .expect_get_quorum_info_with_short_timeout()
        .never();
    let committed = committed_state(&platform, &current, &next, true);

    let (update, context) = run_block_end(
        &platform,
        &committed,
        committed.clone(),
        current.validator_set[2].pro_tx_hash,
        platform_version,
    );

    assert_eq!(update, Some(validator_set(&next, true).to_update()));
    assert!(context
        .block_platform_state()
        .validator_set_changes
        .is_empty());
}
