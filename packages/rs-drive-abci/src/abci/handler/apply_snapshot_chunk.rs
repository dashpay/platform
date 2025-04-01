use crate::abci::app::{SnapshotManagerApplication, StateSyncApplication};
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::core_based_updates::update_quorum_info::v0::QuorumSetType;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::{PlatformStateV0, PlatformStateV0Methods};
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::signature_verification_quorum_set::{
    SignatureVerificationQuorumSet, SignatureVerificationQuorumSetV0Methods, VerificationQuorum,
};
use crate::platform_types::validator_set::v0::ValidatorSetMethodsV0;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore_rpc_json::{
    ExtendedQuorumListResult, MasternodeListDiff, MasternodeType, QuorumType,
};
use dpp::block::extended_block_info::v0::{ExtendedBlockInfoV0, ExtendedBlockInfoV0Getters};
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::core_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use dpp::core_types::validator_set::ValidatorSet as CoreValidatorSet;
use dpp::dashcore::BlockHash;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;

pub fn apply_snapshot_chunk<'a, 'db: 'a, A, C>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike + 'db,
{
    if tracing::enabled!(tracing::Level::TRACE) {
        let chunk_id = if request.chunk_id.len() > 8 {
            &(request.chunk_id[0..8])
        } else {
            &request.chunk_id
        };
        tracing::trace!(
            chunk_id,
            "[state_sync] api apply_snapshot_chunk chunk_id:{}",
            hex::encode(chunk_id)
        );
    }

    let platform_version = app.platform().state.load().current_platform_version()?;

    // Lock first the RwLock
    let mut session_write_guard = app.snapshot_fetching_session().write().map_err(|_| {
        AbciError::StateSyncInternalError(
            "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
        )
    })?;
    {
        let session = session_write_guard
            .as_mut()
            .ok_or(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk unable to lock session".to_string(),
            ))?;

        let next_chunk_ids = session
            .state_sync_info
            .apply_chunk(
                &request.chunk_id,
                &request.chunk,
                platform_version.drive_abci.state_sync.protocol_version,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| {
                tracing::error!(
                    chunk_id = ?request.chunk_id,
                    chunk = ?request.chunk,
                    "state_sync apply_chunk_error",
                );
                AbciError::StateSyncInternalError(format!(
                    "apply_snapshot_chunk unable to apply chunk:{}",
                    e
                ))
            })?;
        if !next_chunk_ids.is_empty() && session.state_sync_info.is_sync_completed() {
            Err(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk sessions is completed but next_chunk_ids is not empty"
                    .to_string(),
            ))?;
        }
        tracing::info!("state_sync completed");
        if !session.state_sync_info.is_sync_completed() {
            return Ok(proto::ResponseApplySnapshotChunk {
                result: proto::response_apply_snapshot_chunk::Result::Accept.into(),
                refetch_chunks: vec![], // TODO: Check when this is needed
                reject_senders: vec![], // TODO: Check when this is needed
                next_chunks: next_chunk_ids,
            });
        }
    }
    {
        // State sync is completed, consume session and commit it
        let session = session_write_guard
            .take()
            .ok_or(AbciError::StateSyncInternalError(
                "apply_snapshot_chunk unable to lock session (poisoned)".to_string(),
            ))?;
        let state_sync_info = session.state_sync_info;
        app.platform()
            .drive
            .grove
            .commit_session(state_sync_info)
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "apply_snapshot_chunk unable to commit session: {}",
                    e
                ))
            })?;
        tracing::trace!("[state_sync] state sync completed. verifying");
        let incorrect_hashes = app
            .platform()
            .drive
            .grove
            .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "apply_snapshot_chunk unable to verify grovedb: {}",
                    e
                ))
            })?;
        if !incorrect_hashes.is_empty() {
            Err(AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk grovedb verification failed with {} incorrect hashes",
                incorrect_hashes.len()
            )))?;
        }

        reconstruct_platform_state(app, session.app_hash.as_slice(), platform_version)?;

        let drive_app_hash = app
            .platform()
            .drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "apply_snapshot_chunk unable to get app hash: {}",
                    e
                ))
            })
            .unwrap()?;

        if drive_app_hash.to_vec() != session.app_hash {
            tracing::error!(
                state_sync_app_hash = ?hex::encode(session.app_hash),
                drive_app_hash = ?hex::encode(drive_app_hash),
                "state_sync: grovedb verification failed with incorrect app hash"
            );
            Err(AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk grovedb verification failed with incorrect app hash: {}",
                hex::encode(drive_app_hash)
            )))?;
        }

        Ok(proto::ResponseApplySnapshotChunk {
            result: proto::response_apply_snapshot_chunk::Result::CompleteSnapshot.into(),
            refetch_chunks: vec![],
            reject_senders: vec![],
            next_chunks: vec![],
        })
    }
}
/// Reconstructs the platform state from the snapshot data.
///
/// ## Expected state
///
/// Given that we run state sync using snapshots created at height H, we expect the following state of
/// the system before entering this function:
///
/// 1. [Platform::<C>::fetch_reduced_platform_state] returns platform state after processing majority of `run_block_proposal`,
///    with an exception to validator_set_update().
fn reconstruct_platform_state<'a, 'db: 'a, A, C>(
    app: &'a A,
    app_hash: &[u8],
    platform_version: &PlatformVersion,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike + 'db,
{
    let config = &app.platform().config;
    let platform = app.platform();
    let drive = &platform.drive;

    log_apphash(app.platform(), "begin of reconstruct_platform_state")?;

    let reduced_platform_state =
        Platform::<C>::fetch_reduced_platform_state(drive, None, platform_version)?.ok_or_else(
            || AbciError::StateSyncInternalError("reduced_platform_state".to_string()),
        )?;

    let saved = match reduced_platform_state {
        ReducedPlatformStateForSaving::V0(v0) => v0,
    };

    // Restore platform state.
    //
    // Platform state is saved in [`Platform::run_block_proposal`], after executing validator rotation
    // and before finalizing the block.
    //
    // At this point, we should have validator sets loaded for next height.
    let mut platform_state = PlatformState::V0(PlatformStateV0 {
        genesis_block_info: None,
        last_committed_block_info: None,
        current_protocol_version_in_consensus: saved.current_protocol_version_in_consensus,
        next_epoch_protocol_version: saved.next_epoch_protocol_version,
        current_validator_set_quorum_hash: BlockHash::from_byte_array(
            saved.current_validator_set_quorum_hash.to_buffer(),
        ),
        next_validator_set_quorum_hash: saved
            .next_validator_set_quorum_hash
            .map(|x| BlockHash::from_byte_array(x.to_buffer())),
        patched_platform_version: None,
        validator_sets: Default::default(),
        chain_lock_validating_quorums: SignatureVerificationQuorumSet::new(
            &config.chain_lock,
            PlatformVersion::get(saved.current_protocol_version_in_consensus)?,
        )?,
        instant_lock_validating_quorums: SignatureVerificationQuorumSet::new(
            &config.instant_lock,
            PlatformVersion::get(saved.current_protocol_version_in_consensus)?,
        )?,
        full_masternode_list: BTreeMap::new(),
        hpmn_masternode_list: BTreeMap::new(),
        previous_fee_versions: saved
            .previous_fee_versions
            .into_keys()
            .map(|epoch_index| (epoch_index, FeeVersion::first()))
            .collect(),
    });
    // We need an ExtendedBlockInfoV0 with app_hash filled; we don't have it as we save during proposal processing
    // TODO: we leave signature and block hash set to [0u8;32], implement them as Option<> to avoid misuse of not
    // initialized values
    let current_block_info: ExtendedBlockInfo = match saved.last_committed_block_info {
        Some(ExtendedBlockInfo::V0(block_info)) => ExtendedBlockInfoV0 {
            app_hash: app_hash.try_into().map_err(|_| {
                AbciError::StateSyncBadRequest(format!(
                    "app_hash {:?} has invalid legth",
                    &app_hash
                ))
            })?,
            ..block_info
        }
        .into(),
        None => {
            return Err(Error::from(AbciError::StateSyncInternalError(
                "reduced_platform_state: last_committed_block_info must be set".to_string(),
            )))
        }
    };

    log_apphash(app.platform(), "before update_core_info")?;

    // Update the masternode list and create masternode identities and also update the active quorums
    let transaction = drive.grove.start_transaction();
    app.platform().update_core_info(
        None,
        &mut platform_state,
        saved.proposed_core_chain_locked_height, // new one, to be used in currently finalized block
        true,
        current_block_info.basic_info(),
        &transaction,
        platform_version,
    )?;

    log_apphash(app.platform(), "before sort_quorums")?;
    sort_quorums(platform_state.validator_sets_mut(), &saved.quorum_positions);

    let block_height = platform_state.last_committed_block_height();

    log_apphash(app.platform(), "before store_platform_state")?;
    platform.store_platform_state(&platform_state, Some(&transaction), platform_version)?;

    log_apphash(app.platform(), "before update_state_cache")?;

    // Now, we must advance to new height in state

    platform.update_state_cache(
        current_block_info,
        platform_state,
        &transaction,
        platform_version,
    )?;

    // platform.state.store(Arc::new(platform_state));

    log_apphash(app.platform(), "before commit_transaction")?;
    drive
        .grove
        .commit_transaction(transaction)
        .map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk unable to commit transaction: {}",
                e
            ))
        })
        .value?;

    if tracing::enabled!(tracing::Level::TRACE) {
        let platform_state_for_logging = platform.state.load();
        tracing::trace!(block_height, platform_state = ?platform_state_for_logging, "state_sync_finalize");
    }

    log_apphash(app.platform(), "at the end of reconstruct_platform_state")?;

    Ok(())
}

/// Logs the application hash of the platform state and returns it.
fn log_apphash<'db, C>(platform: &Platform<C>, desc: &str) -> Result<[u8; 32], Error>
where
    C: CoreRPCLike + 'db,
{
    let grove_version = &platform
        .state
        .load()
        .current_platform_version()
        .unwrap()
        .drive
        .grove_version;

    let app_hash = platform
        .drive
        .grove
        .root_hash(None, grove_version)
        .unwrap()?;

    tracing::trace!(
        app_ash = ?hex::encode(app_hash),
        "state_sync: app hash {}", desc,
    );

    Ok(app_hash)
}

fn build_masternode_lists<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    platform_state: &mut PlatformState,
    core_block_height: u32,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let mn_list_diff = app
        .platform()
        .core_rpc
        .get_protx_diff_with_masternodes(Some(1), core_block_height)?;

    let MasternodeListDiff { added_mns, .. } = &mn_list_diff;

    let added_hpmns = added_mns.iter().filter_map(|masternode| {
        if masternode.node_type == MasternodeType::Evo {
            Some((masternode.pro_tx_hash, masternode.clone()))
        } else {
            None
        }
    });

    let added_masternodes = added_mns
        .iter()
        .map(|masternode| (masternode.pro_tx_hash, masternode.clone()));

    platform_state
        .full_masternode_list_mut()
        .extend(added_masternodes);
    platform_state
        .hpmn_masternode_list_mut()
        .extend(added_hpmns);

    Ok(())
}

fn build_quorum_verification_set<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    extended_quorum_list: &ExtendedQuorumListResult,
    quorum_set_type: QuorumSetType,
    quorum_set: &mut SignatureVerificationQuorumSet,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let quorums_list: BTreeMap<_, _> = extended_quorum_list
        .quorums_by_type
        .get(&quorum_set_type.quorum_type())
        .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
            format!(
                "expected quorums {}, but did not receive any from Dash Core",
                quorum_set_type
            ),
        )))?
        .iter()
        .map(|(quorum_hash, extended_quorum_details)| {
            (quorum_hash, extended_quorum_details.quorum_index)
        })
        .collect();

    // Fetch quorum info and their keys from the RPC for new quorums
    // and then create VerificationQuorum instances
    let new_quorums = quorums_list
        .into_iter()
        .map(|(quorum_hash, index)| {
            let quorum_info = app.platform().core_rpc.get_quorum_info(
                quorum_set_type.quorum_type(),
                quorum_hash,
                None,
            )?;

            let public_key = match BlsPublicKey::try_from(quorum_info.quorum_public_key.as_slice())
                .map_err(ExecutionError::BlsErrorFromDashCoreResponse)
            {
                Ok(public_key) => public_key,
                Err(e) => return Err(e.into()),
            };

            Ok((*quorum_hash, VerificationQuorum { public_key, index }))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    quorum_set.current_quorums_mut().extend(new_quorums);

    Ok(())
}

fn build_validators_list<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    platform_state: &mut PlatformState,
    extended_quorum_list: &mut ExtendedQuorumListResult,
    quorums_order: &Vec<Vec<u8>>,
    validator_set_quorum_type: QuorumType,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let validator_quorums_list: BTreeMap<_, _> = extended_quorum_list
        .quorums_by_type
        .remove(&validator_set_quorum_type)
        .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
            format!(
                "expected quorums of type {}, but did not receive any from Dash Core",
                validator_set_quorum_type
            ),
        )))?
        .into_iter()
        .collect();

    // Fetch quorum info and their keys from the RPC for new quorums
    let mut quorum_infos = validator_quorums_list
        .into_keys()
        .map(|key| {
            let quorum_info_result =
                app.platform()
                    .core_rpc
                    .get_quorum_info(validator_set_quorum_type, &key, None)?;
            Ok((key, quorum_info_result))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // Sort by height and then by hash
    quorum_infos.sort_by(|a, b| {
        let height_cmp = a.1.height.cmp(&b.1.height);
        if height_cmp == std::cmp::Ordering::Equal {
            a.0.cmp(&b.0) // Compare hashes if heights are equal
        } else {
            height_cmp
        }
    });

    // Map to validator sets
    let mut new_validator_sets = quorum_infos
        .into_iter()
        .map(|(quorum_hash, info_result)| {
            let validator_set = CoreValidatorSet::V0(ValidatorSetV0::try_from_quorum_info_result(
                info_result,
                &platform_state,
            )?);
            Ok((quorum_hash, validator_set))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    // Validator sets generated by previous algorithm, just to double-check

    let old_validator_sets = old_sort_quorums(new_validator_sets.clone())
        .into_iter()
        .collect::<Vec<_>>();
    /*
    sort_quorums(&mut new_validator_sets, quorums_order);
    */
    platform_state
        .validator_sets_mut()
        .extend(new_validator_sets.clone());

    if !new_validator_sets.eq(&old_validator_sets) {
        let new_vals = new_validator_sets
            .iter()
            .enumerate()
            .map(|(i, (hash, _))| format!("{}=>{}", i, hex::encode(hash)))
            .collect::<Vec<_>>()
            .join(", ");

        let old_vals = old_validator_sets
            .iter()
            .enumerate()
            .map(|(i, (hash, _))| format!("{}=>{}", i, hex::encode(hash)))
            .collect::<Vec<_>>()
            .join(", ");

        tracing::debug!(
            new_validator_sets = new_vals,
            old_validator_sets = old_vals,
            "state_sync: validator sets sort gives different order"
        );
    };
    // end of old valset check

    Ok(())
}

// Old sorting function, only for testing
// TODO: only for testing, all that old_validator_sets should be removed when new algorithm is stable
fn old_sort_quorums(
    valsets: Vec<(BlockHash, CoreValidatorSet)>,
) -> IndexMap<BlockHash, CoreValidatorSet> {
    let mut old_valset_indexmap: IndexMap<BlockHash, CoreValidatorSet> = IndexMap::new();
    old_valset_indexmap.extend(valsets);

    // Sort all validator sets into deterministic order by core block height of creation
    old_valset_indexmap.sort_by(|_, quorum_a, _, quorum_b| {
        let primary_comparison = quorum_b.core_height().cmp(&quorum_a.core_height());
        if primary_comparison == std::cmp::Ordering::Equal {
            quorum_b
                .quorum_hash()
                .cmp(quorum_a.quorum_hash())
                .then_with(|| quorum_b.core_height().cmp(&quorum_a.core_height()))
        } else {
            primary_comparison
        }
    });

    old_valset_indexmap
}
/// Ensure quorums are in the order described by `order`.
///
/// Modifies `quorums` in place.
fn sort_quorums(quorums: &mut IndexMap<BlockHash, CoreValidatorSet>, order: &[Vec<u8>]) {
    let lookup_table = BTreeMap::from_iter(order.iter().enumerate().map(|x| (x.1 as &[u8], x.0)));

    quorums.sort_by(|a, _, b, _| {
        let a_hash = a.as_byte_array().as_slice();
        let b_hash = b.as_byte_array().as_slice();

        let a_index = lookup_table.get(a_hash).unwrap_or(&usize::MAX);
        let b_index = lookup_table.get(b_hash).unwrap_or(&usize::MAX);

        a_index.cmp(b_index)
    });
    // let ordered = order
    //     .iter()
    //     .map(|quorum_hash| {
    //         let (key, value) = quorums
    //             .iter()
    //             .find(|(hash, _)| hash.to_byte_array().as_slice() == quorum_hash)
    //             .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
    //                 format!(
    //                     "expected quorum {} not found in the list",
    //                     hex::encode(quorum_hash)
    //                 ),
    //             )))?;

    //         Result::<_, Error>::Ok((*key, value.clone()))
    //         // key
    //     })
    //     .collect::<Result<_, Error>>()?;
}

fn update_validators_list<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    platform_state: &mut PlatformState,
    proposer_pro_tx_hash: [u8; 32],
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + StateSyncApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let mut perform_rotation = false;

    if let Some(validator_set) = platform_state
        .validator_sets()
        .get(&platform_state.current_validator_set_quorum_hash())
    {
        if let Some((last_member_pro_tx_hash, _)) = validator_set.members().last_key_value() {
            // we should also perform a rotation if the validator set went through all quorum members
            // this means we are at the last member of the quorum
            if last_member_pro_tx_hash.as_byte_array() == &proposer_pro_tx_hash {
                perform_rotation = true;
            }
        } else {
            // the validator set has no members, very weird, but let's just perform a rotation
            perform_rotation = true;
        }

        // We should also perform a rotation if there are more than one quorum in the system
        // and that the new proposer is on the same quorum and the last proposer but is before
        // them in the list of proposers.
        // This only works if Tenderdash goes through proposers properly
        if &platform_state.last_committed_quorum_hash()
            == platform_state
                .current_validator_set_quorum_hash()
                .as_byte_array()
            && platform_state.last_committed_block_proposer_pro_tx_hash() > proposer_pro_tx_hash
            && platform_state.validator_sets().len() > 1
        {
            // 1 - We haven't changed quorums
            // 2 - The new proposer is before the old proposer
            // 3 - There are more than one quorum in the system
            perform_rotation = true;
        }
    } else {
        // we also need to perform a rotation if the validator set is being removed
        perform_rotation = true;
    }

    //todo: (maybe) perform a rotation if quorum health is low

    if perform_rotation {
        // get the index of the previous quorum
        let mut index = platform_state
            .validator_sets()
            .get_index_of(&platform_state.current_validator_set_quorum_hash())
            .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                format!("perform_rotation: current validator set quorum hash {} not in current known validator sets [{}] processing block {}", platform_state.current_validator_set_quorum_hash(), platform_state
                    .validator_sets().keys().map(|quorum_hash| quorum_hash.to_string()).join(" | "),
                        platform_state.last_committed_block_height() + 1,
                ))))?;
        // we should rotate the quorum
        let quorum_count = platform_state.validator_sets().len();
        match quorum_count {
            0 => Err(Error::Execution(ExecutionError::CorruptedCachedState(
                "no current quorums".to_string(),
            ))),
            1 => Ok(()), // no rotation as we are the only quorum
            count => {
                let start_index = index;
                let oldest_quorum_index_we_can_go_to = if count > 10 {
                    // if we have a lot of quorums (like on testnet and mainnet)
                    // we shouldn't start using the last ones as they could cycle out
                    count - 2
                } else {
                    count
                };
                index = if index + 1 >= oldest_quorum_index_we_can_go_to {
                    0
                } else {
                    index + 1
                };
                // We can't just take the next item because it might no longer be in the state
                for _i in 0..oldest_quorum_index_we_can_go_to {
                    let (quorum_hash, _) = platform_state
                        .validator_sets()
                        .get_index(index)
                        .expect("expected next validator set");

                    // We still have it in the state
                    if let Some(new_validator_set) =
                        platform_state.validator_sets().get(quorum_hash)
                    {
                        platform_state.set_current_validator_set_quorum_hash(*quorum_hash);
                        return Ok(());
                    }
                    index = (index + 1) % oldest_quorum_index_we_can_go_to;
                    if index == start_index {
                        break;
                    }
                }
                // All quorums changed
                if let Some((quorum_hash, new_validator_set)) =
                    platform_state.validator_sets().first()
                {
                    let new_quorum_hash = *quorum_hash;
                    platform_state.set_current_validator_set_quorum_hash(new_quorum_hash);
                    return Ok(());
                }
                Ok(())
            }
        }
    } else {
        Ok(())
    }
}
