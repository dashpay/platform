use crate::masternodes;
use crate::masternodes::{GenerateTestMasternodeUpdates, MasternodeListItemWithUpdates};
use crate::query::ProofVerification;
use crate::strategy::{
    ChainExecutionOutcome, ChainExecutionParameters, CoreHeightIncrease, NetworkStrategy,
    StrategyRandomness, ValidatorVersionMigration,
};
use crate::verify_state_transitions::verify_state_transitions_were_or_were_not_executed;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{BlockHash, ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{
    Bip9SoftforkInfo, Bip9SoftforkStatus, DMNStateDiff, ExtendedQuorumDetails, MasternodeListDiff,
    MasternodeListItem, QuorumInfoResult, QuorumType, SoftforkType,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0Getters;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use strategy_tests::operations::FinalizeBlockOperation::IdentityAddKeys;

use dashcore_rpc::json::{ExtendedQuorumListResult, SoftforkInfo};
use dpp::bls_signatures::PrivateKey;
use dpp::dashcore::consensus::Encodable;
use dpp::dashcore::hashes::{sha256d, HashEngine};
use dpp::dashcore::{ChainLock, QuorumSigningRequestId, VarInt};
use drive_abci::abci::AbciApplication;
use drive_abci::config::PlatformConfig;
use drive_abci::mimic::test_quorum::TestQuorumInfo;
use drive_abci::mimic::{MimicExecuteBlockOptions, MimicExecuteBlockOutcome};
use drive_abci::platform_types::epoch_info::v0::EpochInfoV0;
use drive_abci::platform_types::platform::Platform;
use drive_abci::platform_types::platform_state::v0::PlatformStateV0Methods;
use drive_abci::rpc::core::{CoreRPCLike, MockCoreRPCLike};
use drive_abci::test::fixture::abci::static_init_chain_request;
use platform_version::version::PlatformVersion;
use rand::prelude::{SliceRandom, StdRng};
use rand::{Rng, SeedableRng};
use std::collections::{BTreeMap, HashMap};
use tenderdash_abci::proto::abci::{ResponseInitChain, ValidatorSetUpdate};
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::serializers::timestamp::FromMilis;
use tenderdash_abci::Application;

pub(crate) fn run_chain_for_strategy(
    platform: &mut Platform<MockCoreRPCLike>,
    block_count: u64,
    strategy: NetworkStrategy,
    config: PlatformConfig,
    seed: u64,
) -> ChainExecutionOutcome {
    let validator_quorum_count = strategy.validator_quorum_count; // In most tests 24 quorums
    let chain_lock_quorum_count = strategy.chain_lock_quorum_count; // In most tests 4 quorums when not the same as validator
    let validator_set_quorum_size = config.validator_set_quorum_size;
    let chain_lock_quorum_size = config.chain_lock_quorum_size;

    let mut rng = StdRng::seed_from_u64(seed);

    let any_changes_in_strategy = strategy.proposer_strategy.any_is_set();
    let updated_proposers_in_strategy = strategy.proposer_strategy.any_kind_of_update_is_set();

    let core_height_increase = strategy.core_height_increase.clone();

    let max_core_height =
        core_height_increase.max_core_height(block_count, strategy.initial_core_height);

    let chain_lock_quorum_type = config.chain_lock_quorum_type();

    let sign_chain_locks = strategy.sign_chain_locks;

    let mut core_blocks = BTreeMap::new();
    let mut block_rng = StdRng::seed_from_u64(rng.gen()); // so we don't need to regenerate tests

    for x in strategy.initial_core_height..max_core_height + 1 {
        let block_hash: [u8; 32] = block_rng.gen();
        core_blocks.insert(x, block_hash);
    }

    let (
        initial_masternodes_with_updates,
        initial_hpmns_with_updates,
        with_extra_masternodes_with_updates,
        with_extra_hpmns_with_updates,
    ) = if any_changes_in_strategy {
        let end_core_height = strategy
            .core_height_increase
            .max_core_height(block_count, strategy.initial_core_height);
        let generate_updates = if updated_proposers_in_strategy {
            Some(GenerateTestMasternodeUpdates {
                start_core_height: config.abci.genesis_core_height,
                end_core_height,
                update_masternode_keys_frequency: &strategy.proposer_strategy.updated_masternodes,
                update_hpmn_keys_frequency: &strategy.proposer_strategy.updated_hpmns,
                ban_masternode_frequency: &strategy.proposer_strategy.banned_masternodes,
                ban_hpmn_frequency: &strategy.proposer_strategy.banned_hpmns,
                unban_masternode_frequency: &strategy.proposer_strategy.unbanned_masternodes,
                unban_hpmn_frequency: &strategy.proposer_strategy.unbanned_hpmns,
                change_masternode_ip_frequency: &strategy.proposer_strategy.changed_ip_masternodes,
                change_hpmn_ip_frequency: &strategy.proposer_strategy.changed_ip_hpmns,
                change_hpmn_p2p_port_frequency: &strategy.proposer_strategy.changed_p2p_port_hpmns,
                change_hpmn_http_port_frequency: &strategy
                    .proposer_strategy
                    .changed_http_port_hpmns,
            })
        } else {
            None
        };

        let (initial_masternodes, initial_hpmns) = masternodes::generate_test_masternodes(
            strategy.extra_normal_mns,
            strategy.total_hpmns,
            generate_updates,
            &mut rng,
        );

        let mut all_masternodes = initial_masternodes.clone();
        let mut all_hpmns = initial_hpmns.clone();

        let (extra_masternodes_by_block, extra_hpmns_by_block): (
            BTreeMap<u32, Vec<MasternodeListItemWithUpdates>>,
            BTreeMap<u32, Vec<MasternodeListItemWithUpdates>>,
        ) = (config.abci.genesis_core_height..end_core_height)
            .map(|height| {
                let new_masternodes = strategy
                    .proposer_strategy
                    .new_masternodes
                    .events_if_hit(&mut rng);
                let new_hpmns = strategy.proposer_strategy.new_hpmns.events_if_hit(&mut rng);
                let generate_updates = if updated_proposers_in_strategy {
                    Some(GenerateTestMasternodeUpdates {
                        start_core_height: height + 1,
                        end_core_height,
                        update_masternode_keys_frequency: &strategy
                            .proposer_strategy
                            .updated_masternodes,
                        update_hpmn_keys_frequency: &strategy.proposer_strategy.updated_hpmns,
                        ban_masternode_frequency: &strategy.proposer_strategy.banned_masternodes,
                        ban_hpmn_frequency: &strategy.proposer_strategy.banned_hpmns,
                        unban_masternode_frequency: &strategy
                            .proposer_strategy
                            .unbanned_masternodes,
                        unban_hpmn_frequency: &strategy.proposer_strategy.unbanned_hpmns,
                        change_masternode_ip_frequency: &strategy
                            .proposer_strategy
                            .changed_ip_masternodes,
                        change_hpmn_ip_frequency: &strategy.proposer_strategy.changed_ip_hpmns,
                        change_hpmn_p2p_port_frequency: &strategy
                            .proposer_strategy
                            .changed_p2p_port_hpmns,
                        change_hpmn_http_port_frequency: &strategy
                            .proposer_strategy
                            .changed_http_port_hpmns,
                    })
                } else {
                    None
                };

                let (extra_masternodes_by_block, extra_hpmns_by_block) =
                    masternodes::generate_test_masternodes(
                        new_masternodes,
                        new_hpmns,
                        generate_updates,
                        &mut rng,
                    );

                if strategy.proposer_strategy.removed_masternodes.is_set() {
                    let removed_masternodes_count = strategy
                        .proposer_strategy
                        .removed_masternodes
                        .events_if_hit(&mut rng);
                    let removed_count =
                        std::cmp::min(removed_masternodes_count as usize, all_masternodes.len());
                    all_masternodes.drain(0..removed_count);
                }

                if strategy.proposer_strategy.removed_hpmns.is_set() {
                    let removed_hpmns_count = strategy
                        .proposer_strategy
                        .removed_hpmns
                        .events_if_hit(&mut rng);
                    let removed_count =
                        std::cmp::min(removed_hpmns_count as usize, all_hpmns.len());
                    all_hpmns.drain(0..removed_count);
                }

                all_masternodes.extend(extra_masternodes_by_block);
                all_hpmns.extend(extra_hpmns_by_block);
                (
                    (height, all_masternodes.clone()),
                    (height, all_hpmns.clone()),
                )
            })
            .unzip();
        (
            initial_masternodes,
            initial_hpmns,
            extra_masternodes_by_block,
            extra_hpmns_by_block,
        )
    } else {
        let (initial_masternodes, initial_hpmns) = masternodes::generate_test_masternodes(
            strategy.extra_normal_mns,
            strategy.total_hpmns,
            None,
            &mut rng,
        );
        (
            initial_masternodes,
            initial_hpmns,
            BTreeMap::new(),
            BTreeMap::new(),
        )
    };

    let initial_all_masternodes: Vec<_> = initial_masternodes_with_updates
        .into_iter()
        .chain(initial_hpmns_with_updates.clone())
        .collect();

    let all_hpmns_with_updates = with_extra_hpmns_with_updates
        .iter()
        .max_by_key(|(key, _)| *key)
        .map(|(_, v)| v.clone())
        .unwrap_or(initial_hpmns_with_updates.clone());

    let total_validator_quorums = if strategy.rotate_quorums {
        validator_quorum_count * 10
    } else {
        validator_quorum_count
    };

    let validator_quorums = masternodes::generate_test_quorums(
        total_validator_quorums as usize,
        initial_hpmns_with_updates
            .iter()
            .map(|hpmn| &hpmn.masternode),
        validator_set_quorum_size as usize,
        &mut rng,
    );

    let mut quorums_details: Vec<(QuorumHash, ExtendedQuorumDetails)> = validator_quorums
        .keys()
        .map(|quorum_hash| {
            (
                *quorum_hash,
                ExtendedQuorumDetails {
                    creation_height: 0,
                    quorum_index: None,
                    mined_block_hash: BlockHash::all_zeros(),
                    num_valid_members: 0,
                    health_ratio: 0.0,
                },
            )
        })
        .collect();

    quorums_details.shuffle(&mut rng);

    let (chain_lock_quorums, chain_lock_quorums_details) =
        if config.validator_set_quorum_type != config.chain_lock_quorum_type {
            let total_chain_lock_quorums = if strategy.rotate_quorums {
                chain_lock_quorum_count * 10
            } else {
                chain_lock_quorum_count
            };

            let chain_lock_quorums = masternodes::generate_test_quorums(
                total_chain_lock_quorums as usize,
                initial_all_masternodes
                    .iter()
                    .map(|masternode| &masternode.masternode),
                chain_lock_quorum_size as usize,
                &mut rng,
            );

            let mut chain_lock_quorums_details: Vec<(QuorumHash, ExtendedQuorumDetails)> =
                chain_lock_quorums
                    .keys()
                    .map(|quorum_hash| {
                        (
                            *quorum_hash,
                            ExtendedQuorumDetails {
                                creation_height: 0,
                                quorum_index: None,
                                mined_block_hash: BlockHash::all_zeros(),
                                num_valid_members: 0,
                                health_ratio: 0.0,
                            },
                        )
                    })
                    .collect();

            chain_lock_quorums_details.shuffle(&mut rng);

            (chain_lock_quorums, chain_lock_quorums_details)
        } else {
            (BTreeMap::new(), vec![])
        };

    let start_core_height = platform.config.abci.genesis_core_height;

    platform
        .core_rpc
        .expect_get_fork_info()
        .returning(move |_| {
            // Ok(Some(Bip9SoftforkInfo {
            //     status: Bip9SoftforkStatus::Active,
            //     bit: None,
            //     start_time: 0,
            //     timeout: 0,
            //     since: start_core_height, // block height 1
            //     statistics: None,
            // }))
            Ok(Some(SoftforkInfo {
                softfork_type: SoftforkType::Bip9,
                active: true,
                height: Some(start_core_height), // block height 1
                bip9: Some(Bip9SoftforkInfo {
                    status: Bip9SoftforkStatus::Active,
                    bit: None,
                    start_time: 0,
                    timeout: 0,
                    since: start_core_height, // block height 1
                    statistics: None,
                }),
            }))
        });

    platform
        .core_rpc
        .expect_verify_instant_lock()
        .returning(|_, _| Ok(true));

    platform
        .core_rpc
        .expect_get_quorum_listextended()
        .returning(move |core_height: Option<u32>| {
            let validator_set_extended_info = if !strategy.rotate_quorums {
                quorums_details.clone().into_iter().collect()
            } else {
                let core_height = core_height.expect("expected a core height");
                // if we rotate quorums we shouldn't give back the same ones every time
                let start_range = core_height / 24;
                let end_range = start_range + validator_quorum_count as u32;
                let start_range = start_range % total_validator_quorums as u32;
                let end_range = end_range % total_validator_quorums as u32;

                if end_range > start_range {
                    quorums_details
                        .iter()
                        .skip(start_range as usize)
                        .take((end_range - start_range) as usize)
                        .map(|(quorum_hash, quorum)| (*quorum_hash, quorum.clone()))
                        .collect()
                } else {
                    let first_range = quorums_details
                        .iter()
                        .skip(start_range as usize)
                        .take((total_validator_quorums as u32 - start_range) as usize);
                    let second_range = quorums_details.iter().take(end_range as usize);
                    first_range
                        .chain(second_range)
                        .map(|(quorum_hash, quorum)| (*quorum_hash, quorum.clone()))
                        .collect()
                }
            };

            let mut quorums_by_type =
                HashMap::from([(QuorumType::Llmq100_67, validator_set_extended_info)]);

            if !chain_lock_quorums_details.is_empty() {
                quorums_by_type.insert(
                    QuorumType::Llmq400_60,
                    chain_lock_quorums_details.clone().into_iter().collect(),
                );
            }

            let result = ExtendedQuorumListResult { quorums_by_type };

            Ok(result)
        });

    let all_quorums_info: HashMap<QuorumHash, QuorumInfoResult> = validator_quorums
        .iter()
        .chain(chain_lock_quorums.iter())
        .map(|(quorum_hash, test_quorum_info)| (*quorum_hash, test_quorum_info.into()))
        .collect();

    platform
        .core_rpc
        .expect_get_quorum_info()
        .returning(move |_, quorum_hash: &QuorumHash, _| {
            Ok(all_quorums_info
                .get::<QuorumHash>(quorum_hash)
                .unwrap_or_else(|| panic!("expected to get quorum {}", hex::encode(quorum_hash)))
                .clone())
        });

    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |base_block, block| {
            let diff = if base_block.is_none() {
                MasternodeListDiff {
                    base_height: 0,
                    block_height: block,
                    added_mns: initial_all_masternodes
                        .iter()
                        .map(|masternode_list_item| masternode_list_item.masternode.clone())
                        .collect(),
                    removed_mns: vec![],
                    updated_mns: vec![],
                }
            } else {
                let base_block = base_block.unwrap();
                if !any_changes_in_strategy {
                    // no changes
                    MasternodeListDiff {
                        base_height: base_block,
                        block_height: block,
                        added_mns: vec![],
                        removed_mns: vec![],
                        updated_mns: vec![],
                    }
                } else {
                    // we need to figure out the difference of proposers between two heights
                    // we need to figure out the difference of proposers between two heights
                    let start_masternodes = with_extra_masternodes_with_updates
                        .get(&base_block)
                        .expect("expected start proposers")
                        .iter()
                        .map(|masternode| masternode.get_state_at_height(base_block))
                        .collect::<Vec<_>>();
                    let end_masternodes = with_extra_masternodes_with_updates
                        .get(&block)
                        .expect("expected end proposers")
                        .iter()
                        .map(|masternode| masternode.get_state_at_height(block))
                        .collect::<Vec<_>>();

                    let start_pro_tx_hashes: Vec<&ProTxHash> = start_masternodes
                        .iter()
                        .map(|item| &item.pro_tx_hash)
                        .collect();
                    let end_pro_tx_hashes: Vec<&ProTxHash> = end_masternodes
                        .iter()
                        .map(|item| &item.pro_tx_hash)
                        .collect();

                    let mut added_masternodes = end_masternodes
                        .iter()
                        .filter(|item| !start_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|a| (*a).clone())
                        .collect::<Vec<MasternodeListItem>>();

                    let mut removed_masternodes = start_masternodes
                        .iter()
                        .filter(|item| !end_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|masternode_list_item| masternode_list_item.pro_tx_hash)
                        .collect::<Vec<ProTxHash>>();

                    let mut updated_masternodes: Vec<(ProTxHash, DMNStateDiff)> = start_masternodes
                        .iter()
                        .filter_map(|start_masternode| {
                            end_masternodes
                                .iter()
                                .find(|end_masternode| {
                                    start_masternode.pro_tx_hash == end_masternode.pro_tx_hash
                                })
                                .and_then(|end_masternode| {
                                    start_masternode
                                        .state
                                        .compare_to_newer_dmn_state(&end_masternode.state)
                                        .map(|diff| (end_masternode.pro_tx_hash, diff))
                                })
                        })
                        .collect();

                    let start_hpmns = with_extra_hpmns_with_updates
                        .get(&base_block)
                        .expect("expected start proposers")
                        .iter()
                        .map(|masternode| masternode.get_state_at_height(base_block))
                        .collect::<Vec<_>>();
                    let end_hpmns = with_extra_hpmns_with_updates
                        .get(&block)
                        .expect("expected end proposers")
                        .iter()
                        .map(|masternode| masternode.get_state_at_height(block))
                        .collect::<Vec<_>>();
                    let start_pro_tx_hashes: Vec<&ProTxHash> =
                        start_hpmns.iter().map(|item| &item.pro_tx_hash).collect();
                    let end_pro_tx_hashes: Vec<&ProTxHash> =
                        end_hpmns.iter().map(|item| &item.pro_tx_hash).collect();

                    let added_hpmns = end_hpmns
                        .iter()
                        .filter(|item| !start_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|a| (*a).clone())
                        .collect::<Vec<MasternodeListItem>>();

                    let removed_hpmns = start_hpmns
                        .iter()
                        .filter(|item| !end_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|masternode_list_item| masternode_list_item.pro_tx_hash)
                        .collect::<Vec<ProTxHash>>();

                    let updated_hpmns: Vec<(ProTxHash, DMNStateDiff)> = start_hpmns
                        .iter()
                        .filter_map(|start_masternode| {
                            end_hpmns
                                .iter()
                                .find(|end_masternode| {
                                    start_masternode.pro_tx_hash == end_masternode.pro_tx_hash
                                })
                                .and_then(|end_masternode| {
                                    start_masternode
                                        .state
                                        .compare_to_newer_dmn_state(&end_masternode.state)
                                        .map(|diff| (end_masternode.pro_tx_hash, diff))
                                })
                        })
                        .collect();

                    added_masternodes.extend(added_hpmns);
                    removed_masternodes.extend(removed_hpmns);
                    updated_masternodes.extend(updated_hpmns);

                    // dbg!(&diff);
                    MasternodeListDiff {
                        base_height: base_block,
                        block_height: block,
                        added_mns: added_masternodes,
                        removed_mns: removed_masternodes,
                        updated_mns: updated_masternodes,
                    }
                }
            };

            Ok(diff)
        });

    let mut core_height = strategy.initial_core_height;

    let mut core_height_increase = strategy.core_height_increase.clone();
    let mut core_height_rng = rng.clone();

    let chain_lock_quorums_private_keys: BTreeMap<QuorumHash, [u8; 32]> = chain_lock_quorums
        .iter()
        .map(|(quorum_hash, info)| {
            let bytes = info.private_key.to_bytes();
            let fixed_bytes: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .expect("Expected a byte array of length 32");
            (quorum_hash.clone(), fixed_bytes)
        })
        .collect();

    platform
        .core_rpc
        .expect_get_best_chain_lock()
        .returning(move || {
            let block_height = match &mut core_height_increase {
                CoreHeightIncrease::NoCoreHeightIncrease
                | CoreHeightIncrease::RandomCoreHeightIncrease(_) => core_height,
                CoreHeightIncrease::KnownCoreHeightIncreases(core_block_heights) => {
                    if core_block_heights.len() == 1 {
                        *core_block_heights.first().unwrap()
                    } else {
                        core_block_heights.remove(0)
                    }
                }
            };

            let block_hash = *core_blocks
                .get(&block_height)
                .expect(format!("expected a block hash to be known for {}", core_height).as_str());

            let chain_lock = if sign_chain_locks {
                // From DIP 8: https://github.com/dashpay/dips/blob/master/dip-0008.md#finalization-of-signed-blocks
                // The request id is SHA256("clsig", blockHeight) and the message hash is the block hash of the previously successful attempt.

                let mut engine = QuorumSigningRequestId::engine();

                // Prefix
                let prefix_len = VarInt("clsig".len() as u64);
                prefix_len
                    .consensus_encode(&mut engine)
                    .expect("expected to encode the prefix");

                engine.input("clsig".as_bytes());
                engine.input(core_height.to_le_bytes().as_slice());

                let request_id = QuorumSigningRequestId::from_engine(engine);

                // Based on the deterministic masternode list at the given height, a quorum must be selected that was active at the time this block was mined

                let quorum = Platform::<MockCoreRPCLike>::choose_quorum_thread_safe(
                    chain_lock_quorum_type,
                    &chain_lock_quorums_private_keys,
                    request_id.as_ref(),
                    PlatformVersion::latest(), //it should be okay to use latest here
                )
                .expect("expected a quorum");

                let (quorum_hash, quorum_private_key) = quorum.expect("expected to find a quorum");

                // The signature must verify against the quorum public key and SHA256(llmqType, quorumHash, SHA256(height), blockHash). llmqType and quorumHash must be taken from the quorum selected in 1.

                let mut engine = sha256d::Hash::engine();

                engine.input(&[chain_lock_quorum_type as u8]);
                engine.input(quorum_hash.as_slice());
                engine.input(request_id.as_byte_array());
                engine.input(&block_hash);

                let message_digest = sha256d::Hash::from_engine(engine);

                let quorum_private_key =
                    PrivateKey::from_bytes(quorum_private_key.as_slice(), false)
                        .expect("expected to have a valid private key");
                let signature = quorum_private_key.sign(message_digest.as_byte_array());
                let chain_lock = ChainLock {
                    block_height: core_height,
                    block_hash: BlockHash::from_byte_array(block_hash),
                    signature: (*signature.to_bytes()).into(),
                };

                Ok(chain_lock)
            } else {
                let chain_lock = ChainLock {
                    block_height: core_height,
                    block_hash: BlockHash::from_byte_array(block_hash),
                    signature: [2; 96].into(),
                };

                Ok(chain_lock)
            };

            if let CoreHeightIncrease::RandomCoreHeightIncrease(core_height_increase) =
                &core_height_increase
            {
                core_height += core_height_increase.events_if_hit(&mut core_height_rng) as u32;
            };

            chain_lock
        });

    platform
        .core_rpc
        .expect_submit_chain_lock()
        .returning(move |chain_lock: &ChainLock| return Ok(chain_lock.block_height));

    create_chain_for_strategy(
        platform,
        block_count,
        all_hpmns_with_updates,
        validator_quorums,
        strategy,
        config,
        rng,
    )
}

pub(crate) fn create_chain_for_strategy(
    platform: &Platform<MockCoreRPCLike>,
    block_count: u64,
    proposers_with_updates: Vec<MasternodeListItemWithUpdates>,
    quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    strategy: NetworkStrategy,
    config: PlatformConfig,
    rng: StdRng,
) -> ChainExecutionOutcome {
    let abci_application = AbciApplication::new(platform).expect("expected new abci application");
    let seed = strategy
        .failure_testing
        .as_ref()
        .and_then(|strategy| strategy.deterministic_start_seed)
        .map(StrategyRandomness::SeedEntropy)
        .unwrap_or(StrategyRandomness::RNGEntropy(rng));
    start_chain_for_strategy(
        abci_application,
        block_count,
        proposers_with_updates,
        quorums,
        strategy,
        config,
        seed,
    )
}

pub(crate) fn start_chain_for_strategy(
    abci_application: AbciApplication<MockCoreRPCLike>,
    block_count: u64,
    proposers_with_updates: Vec<MasternodeListItemWithUpdates>,
    quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    strategy: NetworkStrategy,
    config: PlatformConfig,
    seed: StrategyRandomness,
) -> ChainExecutionOutcome {
    let mut rng = match seed {
        StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
        StrategyRandomness::RNGEntropy(rng) => rng,
    };

    let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

    let mut current_quorum_hash = **quorum_hashes
        .choose(&mut rng)
        .expect("expected quorums to be initialized");

    let current_quorum_with_test_info = quorums
        .get::<QuorumHash>(&current_quorum_hash)
        .expect("expected a quorum to be found");

    // init chain
    let mut init_chain_request = static_init_chain_request(&config);

    init_chain_request.initial_core_height = config.abci.genesis_core_height;
    init_chain_request.validator_set = Some(ValidatorSetUpdate {
        validator_updates: current_quorum_with_test_info
            .validator_set
            .iter()
            .map(
                |validator_in_quorum| tenderdash_abci::proto::abci::ValidatorUpdate {
                    pub_key: Some(tenderdash_abci::proto::crypto::PublicKey {
                        sum: Some(Bls12381(validator_in_quorum.public_key.to_bytes().to_vec())),
                    }),
                    power: 100,
                    pro_tx_hash: validator_in_quorum.pro_tx_hash.to_byte_array().to_vec(),
                    node_address: "".to_string(),
                },
            )
            .collect(),
        threshold_public_key: Some(tenderdash_abci::proto::crypto::PublicKey {
            sum: Some(Bls12381(
                current_quorum_with_test_info.public_key.to_bytes().to_vec(),
            )),
        }),
        quorum_hash: current_quorum_hash.to_byte_array().to_vec(),
    });

    let ResponseInitChain {
        initial_core_height,
        ..
    } = abci_application
        .init_chain(init_chain_request)
        .expect("should init chain");

    // initialization will change the current quorum hash
    current_quorum_hash = abci_application
        .platform
        .state
        .read()
        .unwrap()
        .current_validator_set_quorum_hash();

    continue_chain_for_strategy(
        abci_application,
        ChainExecutionParameters {
            block_start: 1,
            core_height_start: initial_core_height,
            block_count,
            proposers: proposers_with_updates,
            quorums,
            current_quorum_hash,
            current_proposer_versions: None,
            start_time_ms: 1681094380000,
            current_time_ms: 1681094380000,
        },
        strategy,
        config,
        StrategyRandomness::RNGEntropy(rng),
    )
}

pub(crate) fn continue_chain_for_strategy(
    abci_app: AbciApplication<MockCoreRPCLike>,
    chain_execution_parameters: ChainExecutionParameters,
    mut strategy: NetworkStrategy,
    config: PlatformConfig,
    seed: StrategyRandomness,
) -> ChainExecutionOutcome {
    let platform = abci_app.platform;
    let ChainExecutionParameters {
        block_start,
        core_height_start,
        block_count,
        proposers: proposers_with_updates,
        quorums,
        mut current_quorum_hash,
        current_proposer_versions,
        start_time_ms,
        mut current_time_ms,
    } = chain_execution_parameters;
    let mut rng = match seed {
        StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
        StrategyRandomness::RNGEntropy(rng) => rng,
    };
    let quorum_size = config.validator_set_quorum_size;
    let first_block_time = start_time_ms;
    let mut current_identities = vec![];
    let mut signer = strategy.strategy.signer.clone().unwrap_or_default();
    let mut i = 0;

    let blocks_per_epoch = config.execution.epoch_time_length_s * 1000 / config.block_spacing_ms;

    let proposer_versions = current_proposer_versions.unwrap_or(
        strategy.upgrading_info.as_ref().map(|upgrading_info| {
            upgrading_info.apply_to_proposers(
                proposers_with_updates
                    .iter()
                    .map(|masternode_list_item| masternode_list_item.pro_tx_hash())
                    .collect(),
                blocks_per_epoch,
                &mut rng,
            )
        }),
    );

    let mut current_core_height = core_height_start;

    let mut total_withdrawals = vec![];

    let mut current_quorum_with_test_info =
        quorums.get::<QuorumHash>(&current_quorum_hash).unwrap();

    let mut validator_set_updates = BTreeMap::new();

    let mut state_transitions_per_block = BTreeMap::new();
    let mut state_transition_results_per_block = BTreeMap::new();

    for block_height in block_start..(block_start + block_count) {
        let state = platform.state.read().expect("lock is poisoned");
        let epoch_info = EpochInfoV0::calculate(
            first_block_time,
            current_time_ms,
            state
                .last_committed_block_info()
                .as_ref()
                .map(|block_info| block_info.basic_info().time_ms),
            config.execution.epoch_time_length_s,
        )
        .expect("should calculate epoch info");

        current_core_height = state.last_committed_core_height();

        drop(state);

        let block_info = BlockInfo {
            time_ms: current_time_ms,
            height: block_height,
            core_height: current_core_height,
            epoch: Epoch::new(epoch_info.current_epoch_index).unwrap(),
        };
        if current_quorum_with_test_info.quorum_hash != current_quorum_hash {
            current_quorum_with_test_info =
                quorums.get::<QuorumHash>(&current_quorum_hash).unwrap();
        }

        let proposer = current_quorum_with_test_info
            .validator_set
            .get(i as usize)
            .unwrap();
        let (state_transitions, finalize_block_operations) = strategy
            .state_transitions_for_block_with_new_identities(
                platform,
                &block_info,
                &mut current_identities,
                &mut signer,
                &mut rng,
            );

        state_transitions_per_block.insert(block_height, state_transitions.clone());

        let proposed_version = proposer_versions
            .as_ref()
            .map(|proposer_versions| {
                let ValidatorVersionMigration {
                    current_protocol_version,
                    next_protocol_version,
                    change_block_height,
                } = proposer_versions
                    .get::<ProTxHash>(&proposer.pro_tx_hash)
                    .expect("expected to have version");
                if &block_height >= change_block_height {
                    *next_protocol_version
                } else {
                    *current_protocol_version
                }
            })
            .unwrap_or(1);

        let rounds = strategy
            .failure_testing
            .as_ref()
            .map(|failure_testing| {
                failure_testing
                    .rounds_before_successful_block
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let expected_validation_errors = strategy
            .failure_testing
            .as_ref()
            .map(|failure_strategy| {
                let mut codes = failure_strategy
                    .expect_every_block_errors_with_codes
                    .clone();
                if let Some(expected_block_error_codes) = failure_strategy
                    .expect_specific_block_errors_with_codes
                    .get(&block_height)
                {
                    codes.extend(expected_block_error_codes);
                }
                codes
            })
            .unwrap_or_default();

        let mut block_execution_outcome = None;
        for round in 0..=rounds {
            block_execution_outcome = Some(
                abci_app
                    .mimic_execute_block(
                        proposer.pro_tx_hash.into(),
                        current_quorum_with_test_info,
                        proposed_version,
                        block_info.clone(),
                        round,
                        expected_validation_errors.as_slice(),
                        false,
                        state_transitions.clone(),
                        MimicExecuteBlockOptions {
                            dont_finalize_block: strategy.dont_finalize_block(),
                            rounds_before_finalization: strategy.failure_testing.as_ref().and_then(
                                |failure_testing| failure_testing.rounds_before_successful_block,
                            ),
                            max_tx_bytes_per_block: strategy.max_tx_bytes_per_block,
                            independent_process_proposal_verification: strategy
                                .independent_process_proposal_verification,
                        },
                    )
                    .expect("expected to execute a block"),
            );
        }

        let MimicExecuteBlockOutcome {
            state_transaction_results,
            withdrawal_transactions: mut withdrawals_this_block,
            validator_set_update,
            next_validator_set_hash,
            root_app_hash,
            state_id,
            block_id_hash: block_hash,
            signature,
            app_version,
        } = block_execution_outcome.unwrap();

        if let Some(validator_set_update) = validator_set_update {
            validator_set_updates.insert(block_height, validator_set_update);
        }

        if strategy.dont_finalize_block() {
            continue;
        }

        let platform_state = platform.state.read().expect("lock is poisoned");

        let platform_version = platform_state.current_platform_version().unwrap();

        total_withdrawals.append(&mut withdrawals_this_block);

        for finalize_block_operation in finalize_block_operations {
            match finalize_block_operation {
                IdentityAddKeys(identifier, keys) => {
                    let identity = current_identities
                        .iter_mut()
                        .find(|identity| identity.id() == identifier)
                        .expect("expected to find an identity");
                    identity
                        .public_keys_mut()
                        .extend(keys.into_iter().map(|key| (key.id(), key)));
                }
            }
        }
        signer.commit_block_keys();

        current_time_ms += config.block_spacing_ms;

        if strategy.verify_state_transition_results {
            //we need to verify state transitions
            verify_state_transitions_were_or_were_not_executed(
                &abci_app,
                &root_app_hash,
                &state_transaction_results,
                &block_info,
                &expected_validation_errors,
                platform_version,
            );
        }

        state_transition_results_per_block.insert(block_height, state_transaction_results);

        if let Some(query_strategy) = &strategy.query_testing {
            query_strategy.query_chain_for_strategy(
                &ProofVerification {
                    quorum_hash: &current_quorum_with_test_info.quorum_hash.into(),
                    quorum_type: config.validator_set_quorum_type(),
                    app_version,
                    chain_id: drive_abci::mimic::CHAIN_ID.to_string(),
                    core_chain_locked_height: state_id.core_chain_locked_height,
                    height: state_id.height as i64,
                    block_hash: &block_hash,
                    app_hash: &root_app_hash,
                    time: Timestamp::from_milis(state_id.time),
                    signature: &signature,
                    public_key: &current_quorum_with_test_info.public_key,
                },
                &current_identities,
                &abci_app,
                StrategyRandomness::RNGEntropy(rng.clone()),
                platform_version,
            )
        }

        let next_quorum_hash =
            QuorumHash::from_byte_array(next_validator_set_hash.try_into().unwrap());
        if current_quorum_hash != next_quorum_hash {
            current_quorum_hash = next_quorum_hash;
            i = 0;
        } else {
            i += 1;
            i %= quorum_size; //todo: this could be variable
        }
    } // for block_height

    let masternode_identity_balances = if strategy.dont_finalize_block() && i == 0 {
        BTreeMap::new()
    } else {
        platform
            .drive
            .fetch_identities_balances(
                &proposers_with_updates
                    .iter()
                    .map(|proposer| proposer.pro_tx_hash().into())
                    .collect(),
                None,
            )
            .expect("expected to get balances")
    };

    let end_epoch_index = if strategy.dont_finalize_block() && i == 0 {
        0
    } else {
        platform
            .state
            .read()
            .expect("lock is poisoned")
            .last_committed_block_info()
            .as_ref()
            .unwrap()
            .basic_info()
            .epoch
            .index
    };

    ChainExecutionOutcome {
        abci_app,
        masternode_identity_balances,
        identities: current_identities,
        proposers: proposers_with_updates,
        quorums,
        current_quorum_hash,
        current_proposer_versions: proposer_versions,
        end_epoch_index,
        end_time_ms: current_time_ms,
        strategy,
        withdrawals: total_withdrawals,
        validator_set_updates,
        state_transition_results_per_block,
    }
}
