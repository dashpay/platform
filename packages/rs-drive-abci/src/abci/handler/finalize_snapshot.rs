use std::collections::BTreeMap;
use std::sync::Arc;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{PubkeyHash, QuorumHash};
use dashcore_rpc::dashcore::blsful::Bls12381G2Impl;
use dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumListResult, MasternodeListDiff, MasternodeListItem, MasternodeType, QuorumType};
use indexmap::IndexMap;
use itertools::Itertools;
use tenderdash_abci::proto::{abci as proto, ToMillis};
use tenderdash_abci::proto::crypto::PublicKey;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::tenderdash_grpc::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::tenderdash_nostd::types::LightBlock;
use tenderdash_abci::proto::types::ValidatorSet;
use tenderdash_abci::signatures::Hashable;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::{Epoch, EPOCH_0};
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
use dpp::core_types::validator::v0::ValidatorV0;
use dpp::core_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::ProTxHash;
use dpp::platform_value::Bytes32;
use dpp::version::version::ProtocolVersion;
use dpp::version::PlatformVersion;
use dpp::core_types::validator_set::ValidatorSet as CoreValidatorSet;
use crate::abci::AbciError;
use crate::abci::app::{PlatformApplication};
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
use crate::platform_types::epoch_info::EpochInfo;
use crate::platform_types::epoch_info::v0::EpochInfoV0;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::v0::{PlatformStateForSavingV1, PlatformStateV0, PlatformStateV0Methods};
use crate::platform_types::signature_verification_quorum_set::{SignatureVerificationQuorumSet, SignatureVerificationQuorumSetForSaving, SignatureVerificationQuorumSetV0Methods, ThresholdBlsPublicKey, VerificationQuorum};
use crate::rpc::core::CoreRPCLike;
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSetForSaving::V1;
use crate::platform_types::validator_set::v0::ValidatorSetMethodsV0;
use crate::execution::platform_events::core_based_updates::update_quorum_info::v0::QuorumSetType;
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::validator_set::ValidatorSetExt;

pub fn finalize_snapshot<A, C>(
    app: &A,
    request: proto::RequestFinalizeSnapshot,
) -> Result<proto::ResponseFinalizeSnapshot, Error>
where
    A: PlatformApplication<C>,
    C: CoreRPCLike,
{
    let config = &app.platform().config;

    let snapshot_block =
        request
            .snapshot_block
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Block".to_string(),
            )))?;

    let snapshot_signed_header =
        snapshot_block
            .signed_header
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Signed Header".to_string(),
            )))?;

    let snapshot_header =
        snapshot_signed_header
            .header
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Header".to_string(),
            )))?;

    if snapshot_header.proposer_pro_tx_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Proposer Tx Hash Size".to_string(),
        )));
    }
    let mut snapshot_proposer_pro_tx_hash_32 = [0u8; 32];
    snapshot_proposer_pro_tx_hash_32.copy_from_slice(&snapshot_header.proposer_pro_tx_hash[..32]);

    let snapshot_header_version =
        snapshot_header
            .version
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Header Version".to_string(),
            )))?;

    let snapshot_header_time =
        snapshot_header
            .time
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Header Timestamp".to_string(),
            )))?;

    let snapshot_block_time = snapshot_header_time.to_millis().map_err(|_| {
        Error::Abci(AbciError::BadRequest(
            "Invalid Snapshot Header Timestamp".to_string(),
        ))
    })?;

    let snapshot_header_last_block_id =
        snapshot_header
            .last_block_id
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Header Last BlockId".to_string(),
            )))?;

    if snapshot_header.app_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Header App Hash Size".to_string(),
        )));
    }
    let mut snapshot_header_app_hash_32 = [0u8; 32];
    snapshot_header_app_hash_32.copy_from_slice(&snapshot_header.app_hash[..32]);

    if snapshot_header.proposer_pro_tx_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Header Proposer ProTx Hash Size".to_string(),
        )));
    }
    let mut snapshot_header_proposer_pro_tx_hash_32 = [0u8; 32];
    snapshot_header_proposer_pro_tx_hash_32
        .copy_from_slice(&snapshot_header.proposer_pro_tx_hash[..32]);

    if snapshot_header_last_block_id.hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Header Last Block Hash Size".to_string(),
        )));
    }
    let mut snapshot_header_last_block_id_hash_32 = [0u8; 32];
    snapshot_header_last_block_id_hash_32
        .copy_from_slice(&snapshot_header_last_block_id.hash[..32]);

    if snapshot_header.validators_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Header Validator Hash Size".to_string(),
        )));
    }
    let mut snapshot_header_validator_hash_32 = [0u8; 32];
    snapshot_header_validator_hash_32.copy_from_slice(&snapshot_header.validators_hash[..32]);

    if snapshot_header.next_validators_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Header Next Validator Hash Size".to_string(),
        )));
    }
    let mut snapshot_header_next_validator_hash_32 = [0u8; 32];
    snapshot_header_next_validator_hash_32
        .copy_from_slice(&snapshot_header.next_validators_hash[..32]);

    let snapshot_commit =
        snapshot_signed_header
            .commit
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Commit".to_string(),
            )))?;

    let snapshot_commit_block_id =
        snapshot_commit
            .block_id
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Commit".to_string(),
            )))?;

    if snapshot_commit_block_id.hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Commit Block Hash Size".to_string(),
        )));
    }
    let mut snapshot_commit_block_hash_32 = [0u8; 32];
    snapshot_commit_block_hash_32.copy_from_slice(&snapshot_commit_block_id.hash[..32]);

    if snapshot_commit.quorum_hash.len() != 32 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Commit Quorum Hash Size".to_string(),
        )));
    }
    let mut snapshot_commit_quorum_hash_32 = [0u8; 32];
    snapshot_commit_quorum_hash_32.copy_from_slice(&snapshot_commit.quorum_hash[..32]);

    if snapshot_commit.threshold_block_signature.len() != 96 {
        return Err(Error::Abci(AbciError::BadRequestDataSize(
            "Invalid Snapshot Commit Threshold Block Signature Size".to_string(),
        )));
    }
    let mut snapshot_commit_threshold_block_sig_96 = [0u8; 96];
    snapshot_commit_threshold_block_sig_96
        .copy_from_slice(&snapshot_commit.threshold_block_signature[..96]);

    let snapshot_validator_set =
        snapshot_block
            .validator_set
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Validator Set".to_string(),
            )))?;

    let snapshot_validator_set_proposer =
        snapshot_validator_set
            .proposer
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Snapshot Proposer".to_string(),
            )))?;

    let snapshot_validator_set_threshold_public_key = snapshot_validator_set
        .threshold_public_key
        .as_ref()
        .ok_or(Error::Abci(AbciError::BadRequest(
            "Empty Snapshot Threshold Public Key".to_string(),
        )))?;

    let genesis_block =
        request
            .genesis_block
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Genesis Block".to_string(),
            )))?;

    let genesis_signed_header =
        genesis_block
            .signed_header
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Genesis Signed Header".to_string(),
            )))?;

    let genesis_header =
        snapshot_signed_header
            .header
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Genesis Header".to_string(),
            )))?;

    let genesis_header_time =
        genesis_header
            .time
            .as_ref()
            .ok_or(Error::Abci(AbciError::BadRequest(
                "Empty Genesis Header Timestamp".to_string(),
            )))?;

    let genesis_block_time = genesis_header_time.to_millis().map_err(|_| {
        Error::Abci(AbciError::BadRequest(
            "Invalid Genesis Header Timestamp".to_string(),
        ))
    })?;

    let genesis_block_info = BlockInfo {
        time_ms: genesis_block_time,
        height: 1,
        core_height: genesis_header.core_chain_locked_height,
        epoch: EPOCH_0,
    };

    let snapshot_block_state_info = BlockStateInfoV0 {
        height: snapshot_header.height as u64,
        round: snapshot_commit.round as u32,
        block_time_ms: snapshot_block_time,
        previous_block_time_ms: None,
        proposer_pro_tx_hash: snapshot_proposer_pro_tx_hash_32,
        core_chain_locked_height: snapshot_header.core_chain_locked_height,
        block_hash: Some(snapshot_commit_block_hash_32),
        app_hash: None,
    };

    let current_epoch_info = EpochInfoV0::from_genesis_time_and_block_info(
        genesis_block_time,
        &snapshot_block_state_info,
        config.execution.epoch_time_length_s,
    )?;

    let current_protocol_version_in_consensus = snapshot_header_version.app as u32;
    let next_epoch_protocol_version = snapshot_header.proposed_app_version as u32;
    let current_validator_set_quorum_hash =
        QuorumHash::from_byte_array(snapshot_commit_quorum_hash_32);

    let mut platform_state = PlatformState::V0(PlatformStateV0 {
        genesis_block_info: Some(genesis_block_info),
        last_committed_block_info: Some(ExtendedBlockInfo::from(ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                time_ms: snapshot_block_time,
                height: snapshot_header.height as u64,
                core_height: snapshot_header.core_chain_locked_height,
                epoch: Epoch::new(current_epoch_info.current_epoch_index)?,
            },
            app_hash: snapshot_header_app_hash_32,
            quorum_hash: snapshot_commit_quorum_hash_32,
            block_id_hash: snapshot_header_last_block_id_hash_32,
            proposer_pro_tx_hash: snapshot_header_proposer_pro_tx_hash_32,
            signature: snapshot_commit_threshold_block_sig_96,
            round: snapshot_commit.round as u32,
        })),
        current_protocol_version_in_consensus,
        next_epoch_protocol_version,
        current_validator_set_quorum_hash,
        next_validator_set_quorum_hash: None,
        patched_platform_version: None,
        validator_sets: Default::default(),
        chain_lock_validating_quorums: SignatureVerificationQuorumSet::from(
            SignatureVerificationQuorumSet::new(
                &config.chain_lock,
                PlatformVersion::get(current_protocol_version_in_consensus)?,
            )?,
        ),
        instant_lock_validating_quorums: SignatureVerificationQuorumSet::from(
            SignatureVerificationQuorumSet::new(
                &config.instant_lock,
                PlatformVersion::get(current_protocol_version_in_consensus)?,
            )?,
        ),
        full_masternode_list: BTreeMap::new(),
        hpmn_masternode_list: BTreeMap::new(),
        previous_fee_versions: Default::default(),
    });

    build_masternode_lists(
        app,
        &mut platform_state,
        snapshot_header.core_chain_locked_height,
    )?;

    let mut extended_quorum_list = app
        .platform()
        .core_rpc
        .get_quorum_listextended(Some(snapshot_header.core_chain_locked_height))?;
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

    build_next_validator_set_quorum_hash(
        snapshot_header_proposer_pro_tx_hash_32,
        &mut platform_state,
    )?;

    /*
    if let Some(quorum_hash) = platform_state.next_validator_set_quorum_hash() {
        if let Some(validator_set) = platform_state.validator_sets().get(quorum_hash) {
            let threshold_public_key = PublicKey {
                sum: Some(Bls12381(validator_set.threshold_public_key().0.to_uncompressed().to_vec())),
            };
            let vs = tenderdash_abci::proto::types::ValidatorSet {
                threshold_public_key: Some(threshold_public_key),
                quorum_hash: quorum_hash.as_byte_array().to_vec(),
                ..Default::default()
            };
            let actual = vs.calculate_msg_hash("", snapshot_commit.height, snapshot_commit.round).unwrap();
            if (snapshot_header_next_validator_hash_32.to_vec() != actual) {
                Error::Abci(AbciError::BadRequest(
                    "Next Validator Quorum Hash not matched".to_string(),
                ));
            }
        }
    }
    */

    let block_height = platform_state.last_committed_block_height();

    tracing::info!(
        block_height,
        platform_state = ?platform_state,
        "state_finalize_snapshot",
    );

    let tx = app.platform().drive.grove.start_transaction();

    app.platform()
        .store_platform_state(&platform_state, Some(&tx), &PlatformVersion::latest())?;

    let _ = app.platform().drive.grove.commit_transaction(tx);

    app.platform().state.store(Arc::new(platform_state));

    Ok(Default::default())
}

fn build_masternode_lists<A, C>(
    app: &A,
    platform_state: &mut PlatformState,
    core_block_height: u32,
) -> Result<(), Error>
where
    A: PlatformApplication<C>,
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

fn build_quorum_verification_set<A, C>(
    app: &A,
    extended_quorum_list: &ExtendedQuorumListResult,
    quorum_set_type: QuorumSetType,
    quorum_set: &mut SignatureVerificationQuorumSet,
) -> Result<(), Error>
where
    A: PlatformApplication<C>,
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

fn build_validators_list<A, C>(
    app: &A,
    platform_state: &mut PlatformState,
    extended_quorum_list: &mut ExtendedQuorumListResult,
    validator_set_quorum_type: QuorumType,
) -> Result<(), Error>
where
    A: PlatformApplication<C>,
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

fn build_next_validator_set_quorum_hash(
    proposer_pro_tx_hash: [u8; 32],
    platform_state: &mut PlatformState,
) -> Result<(), Error> {
    let mut perform_rotation = false;

    if let Some(validator_set) = platform_state
        .validator_sets()
        .get(&platform_state.current_validator_set_quorum_hash())
    {
        if let Some((last_member_pro_tx_hash, _)) = validator_set.members().last_key_value() {
            // we should also perform a rotation if the validator set went through all quorum members
            // this means we are at the last member of the quorum
            if last_member_pro_tx_hash.as_byte_array() == &proposer_pro_tx_hash {
                tracing::debug!(
                    method = "build_next_validator_set_quorum_hash",
                    "rotation: quorum finished as we hit last member {} of quorum {}. All known quorums are: [{}]. quorum rotation expected",
                    hex::encode(proposer_pro_tx_hash),
                        hex::encode(platform_state.current_validator_set_quorum_hash().as_byte_array()),
                    platform_state
                    .validator_sets()
                    .keys()
                    .map(hex::encode).collect::<Vec<_>>().join(" | "),
                );
                perform_rotation = true;
            }
        } else {
            // the validator set has no members, very weird, but let's just perform a rotation
            tracing::debug!(
                method = "build_next_validator_set_quorum_hash",
                "rotation: validator set has no members",
            );
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
            tracing::debug!(
                    method = "build_next_validator_set_quorum_hash",
                "rotation: quorum finished as we hit last an earlier member {} than last block proposer {} for quorum {}. All known quorums are: [{}]. quorum rotation expected",
                hex::encode(proposer_pro_tx_hash),
                    hex::encode(platform_state.last_committed_block_proposer_pro_tx_hash()),
                    hex::encode(platform_state.current_validator_set_quorum_hash().as_byte_array()),
                platform_state
                .validator_sets()
                .keys()
                .map(hex::encode).collect::<Vec<_>>().join(" | "),
                );
            perform_rotation = true;
        }
    } else {
        // we also need to perform a rotation if the validator set is being removed
        tracing::debug!(
                method = "build_next_validator_set_quorum_hash",
                "rotation: new quorums not containing current quorum current {:?}, {}. quorum rotation expected",
                platform_state
                    .validator_sets()
                    .keys()
                    .map(|quorum_hash| format!("{}", quorum_hash)),
                &platform_state.current_validator_set_quorum_hash()
            );
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
                        tracing::debug!(
                            method = "build_next_validator_set_quorum_hash",
                            "rotation: to new quorum: {} with {} members",
                            &quorum_hash,
                            new_validator_set.members().len()
                        );
                        *platform_state.current_validator_set_quorum_hash_mut() = *quorum_hash;
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
                    tracing::debug!(
                        method = "build_next_validator_set_quorum_hash",
                        "rotation: all quorums changed, rotation to new quorum: {}",
                        &quorum_hash
                    );
                    *platform_state.current_validator_set_quorum_hash_mut() = *quorum_hash;
                    return Ok(());
                }
                tracing::debug!("no new quorums to choose from");
                Ok(())
            }
        }
    } else {
        let current_validator_set = platform_state.current_validator_set()?;
        if current_validator_set != platform_state.current_validator_set()? {
            // Something changed, for example the IP of a validator changed, or someone's ban status

            tracing::debug!(
                method = "build_next_validator_set_quorum_hash",
                "validator set update without rotation"
            );
            Ok(())
        } else {
            tracing::debug!(
                method = "build_next_validator_set_quorum_hash",
                "no validator set update",
            );
            Ok(())
        }
    }
}
