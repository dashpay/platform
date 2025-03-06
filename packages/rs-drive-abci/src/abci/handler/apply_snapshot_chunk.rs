use crate::abci::app::{
    PlatformApplication, SnapshotFetchingApplication, SnapshotManagerApplication,
};
use crate::abci::handler::load_snapshot_chunk::ChunkData;
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
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::block::extended_block_info::v0::{ExtendedBlockInfoV0, ExtendedBlockInfoV0Getters};
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::core_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use dpp::core_types::validator_set::ValidatorSet as CoreValidatorSet;
use dpp::dashcore::QuorumHash;
use dpp::reduced_platform_state;
use dpp::reduced_platform_state::ReducedPlatformStateForSaving;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use itertools::Itertools;
use std::collections::BTreeMap;
use std::sync::Arc;
use tenderdash_abci::proto::abci as proto;

pub fn apply_snapshot_chunk<'a, 'db: 'a, A, C: 'db /*+ CoreRPCLike*/>(
    app: &'a A,
    request: proto::RequestApplySnapshotChunk,
) -> Result<proto::ResponseApplySnapshotChunk, Error>
where
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    tracing::trace!(
        "[state_sync] api apply_snapshot_chunk chunk_id:{}",
        hex::encode(&request.chunk_id)
    );
    let mut is_state_sync_completed: bool = false;
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

        let chunk_data = ChunkData::deserialize(&request.chunk).map_err(|e| {
            AbciError::StateSyncInternalError(format!(
                "apply_snapshot_chunk unable to deserialize chunk: {}",
                e
            ))
        })?;
        let chunk = chunk_data.chunk();

        let next_chunk_ids = session
            .state_sync_info
            .apply_chunk(
                &app.platform().drive.grove,
                &request.chunk_id,
                chunk,
                1u16,
                &PlatformVersion::latest().drive.grove_version,
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
        if next_chunk_ids.is_empty() && session.state_sync_info.is_sync_completed() {
            is_state_sync_completed = true;
        }
        tracing::debug!(is_state_sync_completed, "state_sync apply_snapshot_chunk",);
        if !is_state_sync_completed {
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
            .verify_grovedb(
                None,
                true,
                false,
                &PlatformVersion::latest().drive.grove_version,
            )
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

        reconstruct_platform_state(app, session.app_hash.as_slice())?;

        Ok(proto::ResponseApplySnapshotChunk {
            result: proto::response_apply_snapshot_chunk::Result::CompleteSnapshot.into(),
            refetch_chunks: vec![],
            reject_senders: vec![],
            next_chunks: vec![],
        })
    }
}

fn reconstruct_platform_state<'a, 'db: 'a, A, C: 'db>(app: &'a A, app_hash: &[u8]) -> Result<(), Error>
where
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
    C: CoreRPCLike,
{
    let config = &app.platform().config;
    let platform = app.platform();
    let drive = &platform.drive;
    let core_rpc = &platform.core_rpc;

    let reduced_platform_state =
        Platform::<C>::fetch_reduced_platform_state(drive, None, &PlatformVersion::latest())?
            .ok_or_else(|| {
                AbciError::StateSyncInternalError("reduced_platform_state".to_string())
            })?;

    let ReducedPlatformStateForSaving::V0(v0) = reduced_platform_state else {
        return Err(Error::from(AbciError::StateSyncInternalError(
            "reduced_platform_state invalid matching".to_string(),
        )));
    };

    let block_info = Platform::<C>::fetch_last_block_info(drive, None, &PlatformVersion::latest())?
        .ok_or_else(|| {
            AbciError::StateSyncInternalError("last_block_info".to_string())
        })?;
    let core_height = block_info.core_height;

    let last_committed_block = ExtendedBlockInfo::V0 {
        0: ExtendedBlockInfoV0 {
            basic_info: block_info,
            app_hash: app_hash.try_into().unwrap(),
            quorum_hash: [0u8; 32],
            block_id_hash: [0u8; 32],
            proposer_pro_tx_hash: [0u8; 32],
            signature: [0u8; 96],
            round: 0,
        },
        
    };

    let mut platform_state = PlatformState::V0(PlatformStateV0 {
        genesis_block_info: None,
        last_committed_block_info: Some(last_committed_block),
        current_protocol_version_in_consensus: v0.current_protocol_version_in_consensus,
        next_epoch_protocol_version: v0.next_epoch_protocol_version,
        current_validator_set_quorum_hash: QuorumHash::from_byte_array(
            v0.current_validator_set_quorum_hash.to_buffer(),
        ),
        next_validator_set_quorum_hash: v0
            .next_validator_set_quorum_hash
            .map(|bytes| QuorumHash::from_byte_array(bytes.to_buffer())),
        patched_platform_version: None,
        validator_sets: Default::default(),
        chain_lock_validating_quorums: SignatureVerificationQuorumSet::from(
            SignatureVerificationQuorumSet::new(
                &config.chain_lock,
                PlatformVersion::get(v0.current_protocol_version_in_consensus)?,
            )?,
        ),
        instant_lock_validating_quorums: SignatureVerificationQuorumSet::from(
            SignatureVerificationQuorumSet::new(
                &config.instant_lock,
                PlatformVersion::get(v0.current_protocol_version_in_consensus)?,
            )?,
        ),
        full_masternode_list: BTreeMap::new(),
        hpmn_masternode_list: BTreeMap::new(),
        previous_fee_versions: v0
            .previous_fee_versions
            .into_iter()
            .map(|(epoch_index, _)| (epoch_index, FeeVersion::first()))
            .collect(),
    });

    build_masternode_lists(app, &mut platform_state, core_height)?;

    let mut extended_quorum_list = core_rpc.get_quorum_listextended(Some(core_height))?;

    build_quorum_verification_set(
        app,
        &extended_quorum_list,
        QuorumSetType::ChainLock(config.chain_lock.quorum_type),
        platform_state.chain_lock_validating_quorums_mut(),
    )?;
    build_quorum_verification_set(
        app,
        &extended_quorum_list,
        QuorumSetType::InstantLock(config.instant_lock.quorum_type),
        platform_state.instant_lock_validating_quorums_mut(),
    )?;

    build_validators_list(
        app,
        &mut platform_state,
        &mut extended_quorum_list,
        config.validator_set.quorum_type,
    )?;

    let block_height = platform_state.last_committed_block_height();
    tracing::info!(block_height, platform_state = ?platform_state, "state_sync_finalize");

    let tx = drive.grove.start_transaction();
    platform.store_platform_state(&platform_state, Some(&tx), &PlatformVersion::latest())?;
    let _ = drive.grove.commit_transaction(tx);

    platform.state.store(Arc::new(platform_state));

    Ok(())
}

fn build_masternode_lists<'a, 'db: 'a, A, C: 'db>(
    app: &'a A,
    platform_state: &mut PlatformState,
    core_block_height: u32,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
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
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
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
    validator_set_quorum_type: QuorumType,
) -> Result<(), Error>
where
    A: SnapshotManagerApplication + SnapshotFetchingApplication<'db, C> + 'db,
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
        .into_iter()
        .map(|(key, _)| {
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
    let new_validator_sets = quorum_infos
        .into_iter()
        .map(|(quorum_hash, info_result)| {
            let validator_set = CoreValidatorSet::V0(ValidatorSetV0::try_from_quorum_info_result(
                info_result,
                &platform_state,
            )?);
            Ok((quorum_hash, validator_set))
        })
        .collect::<Result<Vec<_>, Error>>()?;

    platform_state
        .validator_sets_mut()
        .extend(new_validator_sets);

    // Sort all validator sets into deterministic order by core block height of creation
    platform_state
        .validator_sets_mut()
        .sort_by(|_, quorum_a, _, quorum_b| {
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

    Ok(())
}
