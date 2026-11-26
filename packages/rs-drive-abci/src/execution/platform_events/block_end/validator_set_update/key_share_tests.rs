use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0, BlockExecutionContextV0Getters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
use crate::execution::types::block_state_info::BlockStateInfo;
use crate::mimic::test_quorum::TestQuorumInfo;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::platform_types::withdrawal::unsigned_withdrawal_txs::v0::UnsignedWithdrawalTxs;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
use dpp::block::block_info::BlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::bls::PublicKey as BlsPublicKey;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::{BlockHash, ProTxHash, QuorumHash, Txid};
use dpp::dashcore_rpc::dashcore_rpc_json::{
    DMNState, ExtendedQuorumDetails, ExtendedQuorumListResult, MasternodeListDiff,
    MasternodeListItem, MasternodeType, QuorumInfoResult,
};
use dpp::version::PlatformVersion;
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;

/// The Core height the quorum validating now was mined at.
const CURRENT_QUORUM_HEIGHT: u32 = 90;
/// The Core height the quorum the chain rotates into was mined at.
const NEXT_QUORUM_HEIGHT: u32 = 100;

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
    let next_answers = Arc::new(AtomicUsize::new(0));
    platform
        .core_rpc
        .expect_get_quorum_info()
        .returning(move |_, quorum_hash, _| {
            if *quorum_hash == current_hash {
                Ok(current_info.clone())
            } else if next_answers.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(next_info_first.clone())
            } else {
                Ok(next_info_later.clone())
            }
        });
    platform
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
    block_state.set_last_committed_block_info(Some(
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
        .into(),
    ));
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
    let mut rng = StdRng::seed_from_u64(1);
    let current = quorum(CURRENT_QUORUM_HEIGHT, 0xa0, 1, &mut rng);
    let next = quorum(NEXT_QUORUM_HEIGHT, 0xb0, 4, &mut rng);
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

    let sent_keys: BTreeMap<Vec<u8>, Option<Vec<u8>>> = update
        .validator_updates
        .iter()
        .map(|validator| {
            let key = validator.pub_key.as_ref().map(|key| match &key.sum {
                Some(Bls12381(bytes)) => bytes.clone(),
                other => panic!("expected a BLS key share, got {other:?}"),
            });
            (validator.pro_tx_hash.clone(), key)
        })
        .collect();
    let expected_keys: BTreeMap<Vec<u8>, Option<Vec<u8>>> = next
        .validator_set
        .iter()
        .map(|member| {
            (
                member.pro_tx_hash.to_byte_array().to_vec(),
                Some(member.public_key.to_bytes().to_vec()),
            )
        })
        .collect();
    assert_eq!(
        sent_keys, expected_keys,
        "Tenderdash must get every member's key share Core has now"
    );

    // The blocks after this one keep them, so a restart or a later rotation does not lose them.
    let kept_keys: BTreeMap<ProTxHash, Option<BlsPublicKey>> = context
        .block_platform_state()
        .validator_sets()
        .get(&next.quorum_hash)
        .expect("expected the next quorum to stay")
        .members()
        .iter()
        .map(|(pro_tx_hash, validator)| (*pro_tx_hash, validator.public_key))
        .collect();
    let expected_kept: BTreeMap<ProTxHash, Option<BlsPublicKey>> = next
        .validator_set
        .iter()
        .map(|member| (member.pro_tx_hash, Some(member.public_key)))
        .collect();
    assert_eq!(kept_keys, expected_kept);
}
