use crate::masternodes;
use crate::masternodes::{GenerateTestMasternodeUpdates, MasternodeListItemWithUpdates};
use crate::operations::FinalizeBlockOperation::IdentityAddKeys;
use crate::signer::SimpleSigner;
use crate::strategy::{
    ChainExecutionOutcome, ChainExecutionParameters, Strategy, StrategyRandomness,
    ValidatorVersionMigration,
};
use dashcore_rpc::dashcore::hashes::hex::ToHex;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{
    Bip9SoftforkInfo, Bip9SoftforkStatus, DMNStateDiff, ExtendedQuorumDetails, MasternodeListDiff,
    MasternodeListItem, QuorumInfoResult, QuorumType,
};
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use drive_abci::abci::mimic::MimicExecuteBlockOutcome;
use drive_abci::abci::AbciApplication;
use drive_abci::config::PlatformConfig;
use drive_abci::execution::fee_pools::epoch::{EpochInfo, EPOCH_CHANGE_TIME_MS};
use drive_abci::execution::test_quorum::TestQuorumInfo;
use drive_abci::platform::Platform;
use drive_abci::rpc::core::MockCoreRPCLike;
use drive_abci::test::fixture::abci::static_init_chain_request;
use rand::prelude::{SliceRandom, StdRng};
use rand::SeedableRng;
use std::collections::{BTreeMap, HashMap};
use tenderdash_abci::proto::abci::{ResponseInitChain, ValidatorSetUpdate};
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;

pub(crate) fn run_chain_for_strategy(
    platform: &mut Platform<MockCoreRPCLike>,
    block_count: u64,
    strategy: Strategy,
    config: PlatformConfig,
    seed: u64,
) -> ChainExecutionOutcome {
    let quorum_count = strategy.quorum_count; // We assume 24 quorums
    let quorum_size = config.quorum_size;

    let mut rng = StdRng::seed_from_u64(seed);

    let any_changes_in_strategy = strategy.proposer_strategy.any_is_set();
    let updated_proposers_in_strategy = strategy.proposer_strategy.updated_any_masternode_types();

    let (
        initial_masternodes_with_updates,
        initial_hpmns_with_updates,
        with_extra_masternodes_with_updates,
        with_extra_hpmns_with_updates,
    ) = if any_changes_in_strategy {
        let approximate_end_core_height =
            ((block_count as f64) * strategy.core_height_increase.average_event_count()) as u32;
        let end_core_height = approximate_end_core_height * 2; //let's be safe
        let generate_updates = if updated_proposers_in_strategy {
            Some(GenerateTestMasternodeUpdates {
                start_core_height: config.abci.genesis_core_height,
                end_core_height,
                update_masternode_frequency: &strategy.proposer_strategy.updated_mastenodes,
                update_hpmn_frequency: &strategy.proposer_strategy.updated_hpmns,
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
                        update_masternode_frequency: &strategy.proposer_strategy.updated_mastenodes,
                        update_hpmn_frequency: &strategy.proposer_strategy.updated_hpmns,
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

                all_masternodes.extend(extra_masternodes_by_block.clone());
                all_hpmns.extend(extra_hpmns_by_block.clone());
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

    let mut all_hpmns_with_updates = with_extra_hpmns_with_updates
        .iter()
        .max_by_key(|(key, _)| *key)
        .map(|(_, v)| v.clone())
        .unwrap_or(initial_hpmns_with_updates.clone());

    let total_quorums = if strategy.rotate_quorums {
        quorum_count * 10
    } else {
        quorum_count
    };

    let quorums = masternodes::generate_test_quorums(
        total_quorums as usize,
        initial_hpmns_with_updates
            .iter()
            .map(|hpmn| &hpmn.masternode),
        quorum_size as usize,
        &mut rng,
    );

    let quorums_clone: HashMap<QuorumHash, ExtendedQuorumDetails> = quorums
        .keys()
        .map(|quorum_hash| {
            (
                *quorum_hash,
                ExtendedQuorumDetails {
                    creation_height: 0,
                    quorum_index: None,
                    mined_block_hash: Default::default(),
                    num_valid_members: 0,
                    health_ratio: 0.0,
                },
            )
        })
        .collect();

    let start_core_height = platform.config.abci.genesis_core_height;

    platform
        .core_rpc
        .expect_get_fork_info()
        .returning(move |_| {
            Ok(Some(Bip9SoftforkInfo {
                status: Bip9SoftforkStatus::Active,
                bit: None,
                start_time: 0,
                timeout: 0,
                since: start_core_height, // block height 1
                statistics: None,
            }))
        });

    platform
        .core_rpc
        .expect_get_quorum_listextended()
        .returning(move |core_height: Option<u32>| {
            if !strategy.rotate_quorums {
                Ok(dashcore_rpc::dashcore_rpc_json::ExtendedQuorumListResult {
                    quorums_by_type: HashMap::from([(
                        QuorumType::Llmq100_67,
                        quorums_clone.clone(),
                    )]),
                })
            } else {
                let core_height = core_height.expect("expected a core height");
                // if we rotate quorums we shouldn't give back the same ones every time
                let start_range = core_height / 24;
                let end_range = start_range + quorum_count as u32;
                let start_range = start_range % total_quorums as u32;
                let end_range = end_range % total_quorums as u32;

                let quorums = if end_range > start_range {
                    quorums_clone
                        .iter()
                        .skip(start_range as usize)
                        .take((end_range - start_range) as usize)
                        .map(|(quorum_hash, quorum)| (quorum_hash.clone(), quorum.clone()))
                        .collect()
                } else {
                    let first_range = quorums_clone
                        .iter()
                        .skip(start_range as usize)
                        .take((total_quorums as u32 - start_range) as usize);
                    let second_range = quorums_clone.iter().take(end_range as usize);
                    first_range
                        .chain(second_range)
                        .map(|(quorum_hash, quorum)| (quorum_hash.clone(), quorum.clone()))
                        .collect()
                };

                Ok(dashcore_rpc::dashcore_rpc_json::ExtendedQuorumListResult {
                    quorums_by_type: HashMap::from([(QuorumType::Llmq100_67, quorums)]),
                })
            }
        });

    let quorums_info: HashMap<QuorumHash, QuorumInfoResult> = quorums
        .iter()
        .map(|(quorum_hash, test_quorum_info)| (*quorum_hash, test_quorum_info.into()))
        .collect();

    platform
        .core_rpc
        .expect_get_quorum_info()
        .returning(move |_, quorum_hash: &QuorumHash, _| {
            Ok(quorums_info
                .get(quorum_hash)
                .unwrap_or_else(|| panic!("expected to get quorum {}", quorum_hash.to_hex()))
                .clone())
        });

    let initial_all_masternodes: Vec<_> = initial_masternodes_with_updates
        .clone()
        .into_iter()
        .chain(initial_hpmns_with_updates.clone().into_iter())
        .collect();

    platform
        .core_rpc
        .expect_get_protx_diff_with_masternodes()
        .returning(move |base_block, block| {
            let diff = if base_block == 0 {
                MasternodeListDiff {
                    base_height: base_block,
                    block_height: block,
                    added_mns: initial_all_masternodes
                        .iter()
                        .map(|masternode_list_item| masternode_list_item.masternode.clone())
                        .collect(),
                    removed_mns: vec![],
                    updated_mns: vec![],
                }
            } else {
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
                        .map(|masternode_list_item| masternode_list_item.pro_tx_hash.clone())
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
                                        .map(|diff| (end_masternode.pro_tx_hash.clone(), diff))
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

                    let mut added_hpmns = end_hpmns
                        .iter()
                        .filter(|item| !start_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|a| (*a).clone())
                        .collect::<Vec<MasternodeListItem>>();

                    let mut removed_hpmns = start_hpmns
                        .iter()
                        .filter(|item| !end_pro_tx_hashes.contains(&&item.pro_tx_hash))
                        .map(|masternode_list_item| masternode_list_item.pro_tx_hash.clone())
                        .collect::<Vec<ProTxHash>>();

                    let mut updated_hpmns: Vec<(ProTxHash, DMNStateDiff)> = start_hpmns
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
                                        .map(|diff| (end_masternode.pro_tx_hash.clone(), diff))
                                })
                        })
                        .collect();

                    added_masternodes.extend(added_hpmns);
                    removed_masternodes.extend(removed_hpmns);
                    updated_masternodes.extend(updated_hpmns);

                    let diff = MasternodeListDiff {
                        base_height: base_block,
                        block_height: block,
                        added_mns: added_masternodes,
                        removed_mns: removed_masternodes,
                        updated_mns: updated_masternodes,
                    };
                    // dbg!(&diff);
                    diff
                }
            };

            Ok(diff)
        });

    start_chain_for_strategy(
        platform,
        block_count,
        all_hpmns_with_updates,
        quorums,
        strategy,
        config,
        rng,
    )
}

pub(crate) fn start_chain_for_strategy(
    platform: &Platform<MockCoreRPCLike>,
    block_count: u64,
    proposers_with_updates: Vec<MasternodeListItemWithUpdates>,
    quorums: BTreeMap<QuorumHash, TestQuorumInfo>,
    strategy: Strategy,
    config: PlatformConfig,
    mut rng: StdRng,
) -> ChainExecutionOutcome {
    let abci_application = AbciApplication::new(platform).expect("expected new abci application");

    let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

    let mut current_quorum_hash = **quorum_hashes
        .choose(&mut rng)
        .expect("expected quorums to be initialized");

    let current_quorum_with_test_info = quorums
        .get(&current_quorum_hash)
        .expect("expected a quorum to be found");

    // init chain
    let mut init_chain_request = static_init_chain_request();

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
                    pro_tx_hash: validator_in_quorum.pro_tx_hash.to_vec(),
                    node_address: "".to_string(),
                },
            )
            .collect(),
        threshold_public_key: Some(tenderdash_abci::proto::crypto::PublicKey {
            sum: Some(Bls12381(
                current_quorum_with_test_info.public_key.to_bytes().to_vec(),
            )),
        }),
        quorum_hash: current_quorum_hash.to_vec(),
    });

    abci_application.start_transaction();

    let binding = abci_application.transaction.read().unwrap();

    let transaction = binding.as_ref().expect("expected a transaction");

    let ResponseInitChain {
        initial_core_height,
        ..
    } = platform
        .init_chain(init_chain_request, transaction)
        .expect("should init chain");

    // initialization will change the current quorum hash
    current_quorum_hash = platform
        .state
        .read()
        .unwrap()
        .current_validator_set_quorum_hash;

    platform.create_mn_shares_contract(Some(transaction));

    drop(binding);

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
    mut strategy: Strategy,
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
        mut current_time_ms,
    } = chain_execution_parameters;
    let mut rng = match seed {
        StrategyRandomness::SeedEntropy(seed) => StdRng::seed_from_u64(seed),
        StrategyRandomness::RNGEntropy(rng) => rng,
    };
    let quorum_size = config.quorum_size;
    let quorum_rotation_block_count = config.validator_set_quorum_rotation_block_count as u64;
    let first_block_time = 0;
    let mut current_identities = vec![];
    let mut signer = SimpleSigner::default();
    let mut i = 0;

    let blocks_per_epoch = EPOCH_CHANGE_TIME_MS / config.block_spacing_ms;

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

    let mut current_quorum_with_test_info = quorums.get(&current_quorum_hash).unwrap();

    let mut next_quorum_hash = current_quorum_hash;

    for block_height in block_start..(block_start + block_count) {
        let needs_rotation_on_next_block = block_height % quorum_rotation_block_count == 0;
        if needs_rotation_on_next_block {
            let quorum_hashes: Vec<&QuorumHash> = quorums.keys().collect();

            next_quorum_hash = **quorum_hashes.choose(&mut rng).unwrap();
        }
        let epoch_info = EpochInfo::calculate(
            first_block_time,
            current_time_ms,
            platform
                .state
                .read()
                .expect("lock is poisoned")
                .last_committed_block_info
                .as_ref()
                .map(|block_info| block_info.time_ms),
        )
        .expect("should calculate epoch info");

        current_core_height += strategy.core_height_increase.events_if_hit(&mut rng) as u32;

        let block_info = BlockInfo {
            time_ms: current_time_ms,
            height: block_height,
            core_height: current_core_height,
            epoch: Epoch::new(epoch_info.current_epoch_index).unwrap(),
        };
        if current_quorum_with_test_info.quorum_hash != current_quorum_hash {
            current_quorum_with_test_info = quorums.get(&current_quorum_hash).unwrap();
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

        let proposed_version = proposer_versions
            .as_ref()
            .map(|proposer_versions| {
                let ValidatorVersionMigration {
                    current_protocol_version,
                    next_protocol_version,
                    change_block_height,
                } = proposer_versions
                    .get(&proposer.pro_tx_hash)
                    .expect("expected to have version");
                if &block_height >= change_block_height {
                    *next_protocol_version
                } else {
                    *current_protocol_version
                }
            })
            .unwrap_or(1);

        let MimicExecuteBlockOutcome {
            withdrawal_transactions: mut withdrawals_this_block,
            next_validator_set_hash,
        } = abci_app
            .mimic_execute_block(
                proposer.pro_tx_hash.into_inner(),
                current_quorum_with_test_info,
                proposed_version,
                block_info,
                false,
                state_transitions,
            )
            .expect("expected to execute a block");

        total_withdrawals.append(&mut withdrawals_this_block);

        for finalize_block_operation in finalize_block_operations {
            match finalize_block_operation {
                IdentityAddKeys(identifier, keys) => {
                    let identity = current_identities
                        .iter_mut()
                        .find(|identity| identity.id == identifier)
                        .expect("expected to find an identity");
                    identity
                        .public_keys
                        .extend(keys.into_iter().map(|key| (key.id, key)));
                }
            }
        }
        signer.commit_block_keys();

        current_time_ms += config.block_spacing_ms;

        let next_quorum_hash = QuorumHash::from_inner(next_validator_set_hash.try_into().unwrap());
        if current_quorum_hash != next_quorum_hash {
            current_quorum_hash = next_quorum_hash;
            i = 0;
        } else {
            i += 1;
            i %= quorum_size; //todo: this could be variable
        }
    }

    let masternode_identity_balances = platform
        .drive
        .fetch_identities_balances(
            &proposers_with_updates
                .iter()
                .map(|proposer| proposer.pro_tx_hash().into_inner())
                .collect(),
            None,
        )
        .expect("expected to get balances");

    let end_epoch_index = platform
        .state
        .read()
        .expect("lock is poisoned")
        .last_committed_block_info
        .as_ref()
        .unwrap()
        .epoch
        .index;

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
    }
}
