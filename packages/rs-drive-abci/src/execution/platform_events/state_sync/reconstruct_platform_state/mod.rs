use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::signature_verification_quorum_set::{
    Quorums, SignatureVerificationQuorumSet, SignatureVerificationQuorumSetV0Methods,
    VerificationQuorum,
};
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::v0::{ExtendedBlockInfoV0, ExtendedBlockInfoV0Getters};
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::platform_value::Bytes32;
use dpp::reduced_platform_state::v0::ReducedPreviousQuorumsV0;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use std::collections::{BTreeMap, BTreeSet};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Reconstructs the full in-memory platform state after a state sync snapshot
    /// restore, and persists it to aux storage so it survives restarts.
    ///
    /// ## Expected state
    ///
    /// The restored grovedb contains the reduced platform state that
    /// `run_block_proposal` v1 wrote while processing the snapshot block, i.e. the
    /// state after the whole block including `validator_set_update`, immediately
    /// before the root hash was computed. Reconstruction:
    ///
    /// 1. restores the scalar fields (protocol versions, quorum hashes, fee versions)
    ///    directly from the reduced state;
    /// 2. re-derives the masternode lists, masternode identities and quorums from Core
    ///    via `update_core_info` with `start_from_scratch = true` — the identity writes
    ///    are re-derivations of data already present in the restored state, so the
    ///    grovedb root hash MUST NOT change (the caller's root-hash equality check is
    ///    the proof of that idempotence);
    /// 3. restores the validator set order recorded by the source (`quorum_positions`),
    ///    which cannot be recovered from Core RPC;
    /// 4. advances the state to the snapshot block via `update_state_cache`, which
    ///    performs the same next-into-current validator set rotation the source node
    ///    performed when it finalized that block, persists the state to aux storage and
    ///    publishes it, so the `info` handler reports the snapshot height and app hash
    ///    after both this restore and any later restart.
    pub fn reconstruct_platform_state(
        &self,
        app_hash: &[u8; 32],
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let reduced_platform_state = self
            .fetch_reduced_platform_state(None, platform_version)?
            .ok_or_else(|| {
                AbciError::StateSyncInternalError(
                    "reconstruct_platform_state restored snapshot does not contain a reduced \
                     platform state (was it taken before the v15 activation height?)"
                        .to_string(),
                )
            })?;
        let ReducedPlatformState::V0(saved) = reduced_platform_state;

        // Everything below runs with the platform version the snapshot's chain was
        // actually on, which may lag the version this binary considers latest.
        let state_platform_version =
            PlatformVersion::get(saved.current_protocol_version_in_consensus)?;

        // Restore the fee versions of previous epochs faithfully, by version number
        let previous_fee_versions: CachedEpochIndexFeeVersions = saved
            .previous_fee_versions
            .iter()
            .map(|(epoch_index, fee_version_number)| {
                Ok((*epoch_index, FeeVersion::get(*fee_version_number)?))
            })
            .collect::<Result<_, dpp::version::PlatformVersionError>>()?;

        let mut platform_state = PlatformState {
            genesis_block_info: None,
            last_committed_block_info: None,
            current_protocol_version_in_consensus: saved.current_protocol_version_in_consensus,
            next_epoch_protocol_version: saved.next_epoch_protocol_version,
            current_validator_set_quorum_hash: QuorumHash::from_byte_array(
                saved.current_validator_set_quorum_hash.to_buffer(),
            ),
            next_validator_set_quorum_hash: saved
                .next_validator_set_quorum_hash
                .map(|quorum_hash| QuorumHash::from_byte_array(quorum_hash.to_buffer())),
            validator_sets: Default::default(),
            chain_lock_validating_quorums: SignatureVerificationQuorumSet::new(
                &self.config.chain_lock,
                state_platform_version,
            )?,
            instant_lock_validating_quorums: SignatureVerificationQuorumSet::new(
                &self.config.instant_lock,
                state_platform_version,
            )?,
            full_masternode_list: Default::default(),
            hpmn_masternode_list: Default::default(),
            previous_fee_versions,
        };

        let saved_block_info =
            saved
                .last_committed_block_info
                .ok_or(AbciError::StateSyncInternalError(
                    "reconstruct_platform_state reduced platform state has no last committed \
                     block info"
                        .to_string(),
                ))?;

        // The reduced state is written before the block's root hash exists, so its app
        // hash is normally None and the snapshot app hash fills it in. If it does carry
        // one, it must agree with the snapshot.
        if let Some(saved_app_hash) = saved_block_info.app_hash {
            if saved_app_hash.to_buffer() != *app_hash {
                return Err(AbciError::StateSyncInternalError(format!(
                    "reconstruct_platform_state reduced platform state app hash {} does not \
                     match snapshot app hash {}",
                    hex::encode(saved_app_hash.to_buffer()),
                    hex::encode(app_hash),
                ))
                .into());
            }
        }

        let current_block_info: ExtendedBlockInfo = ExtendedBlockInfoV0 {
            basic_info: saved_block_info.basic_info,
            app_hash: *app_hash,
            quorum_hash: saved_block_info.quorum_hash.to_buffer(),
            // Not known during proposal processing, and not needed for consensus after
            // a restore; restored as zeroes.
            block_id_hash: saved_block_info
                .block_id_hash
                .map(|hash| hash.to_buffer())
                .unwrap_or_default(),
            proposer_pro_tx_hash: saved_block_info.proposer_pro_tx_hash.to_buffer(),
            // Same: unknown at store time, restored as zeroes when absent.
            signature: saved_block_info.signature.unwrap_or([0u8; 96]),
            round: saved_block_info.round,
        }
        .into();

        // Re-derive masternode lists, masternode identities and quorums from Core, from
        // scratch, at the core height the snapshot block ran with. The identity writes
        // must be byte-identical to what is already in the restored state.
        let transaction = self.drive.grove.start_transaction();
        self.update_core_info(
            None,
            &mut platform_state,
            saved.proposed_core_chain_locked_height,
            true,
            current_block_info.basic_info(),
            &transaction,
            state_platform_version,
        )?;

        // The validator sets live in the platform state, NOT in grovedb, so the caller's
        // app-hash equality check cannot see a disagreement between what Core just handed
        // us and what the snapshot source actually ran with. `quorum_positions` is the
        // consensus-covered list of validator set hashes from the source, so require an
        // exact match before publishing anything: a restored node running with validator
        // sets the chain never agreed on is worse than no restore at all, and the caller
        // turns this error into a REJECT_SNAPSHOT.
        let derived_validator_sets: BTreeSet<[u8; 32]> = platform_state
            .validator_sets()
            .keys()
            .map(|quorum_hash| quorum_hash.to_byte_array())
            .collect();
        let saved_validator_sets: BTreeSet<[u8; 32]> = saved
            .quorum_positions
            .iter()
            .map(|quorum_hash| quorum_hash.to_buffer())
            .collect();
        if derived_validator_sets != saved_validator_sets {
            return Err(AbciError::StateSyncInternalError(format!(
                "reconstruct_platform_state validator sets re-derived from Core do not match the \
                 snapshot: {} derived, {} saved, {} only in Core, {} only in the snapshot",
                derived_validator_sets.len(),
                saved_validator_sets.len(),
                derived_validator_sets
                    .difference(&saved_validator_sets)
                    .count(),
                saved_validator_sets
                    .difference(&derived_validator_sets)
                    .count(),
            ))
            .into());
        }

        // Core RPC returns quorums in an order that need not match the incremental
        // order the source node maintained; restore the recorded order.
        sort_validator_sets_by_saved_positions(
            platform_state.validator_sets_mut(),
            &saved.quorum_positions,
        );

        // Reinstate the signature-verification quorum HISTORY. `update_core_info` above
        // rebuilt the current sets from Core — which is exact, the quorums of a type at a
        // core height are whatever Core reports — but it was given `platform_state = None`
        // and so could not produce any previous set. That history is consensus-relevant:
        // `select_quorums` uses the previous set for locks signed within `SIGN_OFFSET`
        // core blocks of a change, and for instant locks there is no Core fallback, so a
        // restored node missing it would reject an asset lock proof the network accepted.
        restore_previous_quorums(
            platform_state.chain_lock_validating_quorums_mut(),
            saved.previous_chain_lock_quorums.as_ref(),
        )?;
        restore_previous_quorums(
            platform_state.instant_lock_validating_quorums_mut(),
            saved.previous_instant_lock_quorums.as_ref(),
        )?;

        let block_height = platform_state.last_committed_block_height();

        // Commit the re-derivation BEFORE the in-memory state is published: if this
        // commit fails, nothing has been published and the error propagates with the
        // node's observable state unchanged. (Publishing first, as normal block
        // finalization does, would leave the info handler reporting a snapshot height
        // that grovedb never persisted.)
        self.drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "reconstruct_platform_state unable to commit transaction: {}",
                    e
                ))
            })?;

        // Advance the state to the snapshot block: rotates next-into-current exactly as
        // the source did on finalization, persists to aux storage and publishes the
        // state for the info handler. Aux writes are not part of the root hash, so
        // committing them separately cannot change the app hash the caller verifies.
        let aux_transaction = self.drive.grove.start_transaction();
        self.update_state_cache(
            current_block_info,
            platform_state,
            &aux_transaction,
            state_platform_version,
        )?;
        self.drive
            .grove
            .commit_transaction(aux_transaction)
            .unwrap()
            .map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "reconstruct_platform_state unable to commit aux transaction: {}",
                    e
                ))
            })?;

        tracing::debug!(
            block_height,
            app_hash = hex::encode(app_hash),
            "[state_sync] platform state reconstructed",
        );

        Ok(())
    }
}

/// Reinstates the superseded quorums of a signature-verification quorum set exactly as the
/// snapshot source held them.
///
/// `None` is a legitimate answer (the source had seen no quorum change yet) and leaves the
/// set without a history, which is what the source had.
fn restore_previous_quorums(
    quorum_set: &mut SignatureVerificationQuorumSet,
    saved: Option<&ReducedPreviousQuorumsV0>,
) -> Result<(), Error> {
    let Some(saved) = saved else {
        return Ok(());
    };

    let quorums = saved
        .quorums
        .iter()
        .map(|quorum| {
            let public_key = BlsPublicKey::try_from(quorum.public_key.as_slice()).map_err(|e| {
                AbciError::StateSyncInternalError(format!(
                    "reconstruct_platform_state previous quorum {} has an undeserializable public \
                     key: {}",
                    hex::encode(quorum.quorum_hash.to_buffer()),
                    e
                ))
            })?;

            Ok((
                QuorumHash::from_byte_array(quorum.quorum_hash.to_buffer()),
                VerificationQuorum {
                    public_key,
                    index: quorum.index,
                },
            ))
        })
        .collect::<Result<Quorums<VerificationQuorum>, Error>>()?;

    quorum_set.restore_previous_past_quorums(
        quorums,
        saved.last_active_core_height,
        saved.updated_at_core_height,
        saved.previous_change_height,
    );

    Ok(())
}

/// Sorts the validator sets into the order recorded in the reduced platform state.
///
/// Validator sets not present in the recorded order (which should not happen when the
/// reduced state and Core agree on the quorum list) sort last, preserving their
/// relative order.
fn sort_validator_sets_by_saved_positions(
    validator_sets: &mut IndexMap<QuorumHash, ValidatorSet>,
    quorum_positions: &[Bytes32],
) {
    let lookup_table: BTreeMap<&[u8], usize> = quorum_positions
        .iter()
        .enumerate()
        .map(|(position, quorum_hash)| (quorum_hash.as_slice(), position))
        .collect();

    validator_sets.sort_by(|a_hash, _, b_hash, _| {
        let a_position = lookup_table
            .get(a_hash.as_byte_array().as_slice())
            .unwrap_or(&usize::MAX);
        let b_position = lookup_table
            .get(b_hash.as_byte_array().as_slice())
            .unwrap_or(&usize::MAX);

        a_position.cmp(b_position)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quorum_hash(seed: u8) -> QuorumHash {
        let mut bytes = [0u8; 32];
        bytes[31] = seed;
        QuorumHash::from_byte_array(bytes)
    }

    /// The quorum-set history that travels with a snapshot must come back byte for byte,
    /// including `previous_change_height` — `set_previous_past_quorums` DERIVES that field
    /// from whatever the set already holds, which on a freshly reconstructed set is
    /// nothing, so restoring through it would silently lose it and change which quorums
    /// `select_quorums` considers verifiable.
    #[test]
    fn should_restore_the_previous_quorum_history_verbatim() {
        use crate::config::ChainLockConfig;
        use crate::platform_types::platform_state::to_reduced_previous_quorums;
        use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut rng = StdRng::seed_from_u64(11);
        let quorums: Quorums<VerificationQuorum> = [(1u8, None), (2u8, Some(3u32))]
            .into_iter()
            .map(|(seed, index)| {
                (
                    quorum_hash(seed),
                    VerificationQuorum {
                        public_key: SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key(),
                        index,
                    },
                )
            })
            .collect();

        let mut source = SignatureVerificationQuorumSet::new(
            &ChainLockConfig::default_100_67(),
            PlatformVersion::latest(),
        )
        .expect("should build quorum set");
        // Two changes, so `previous_change_height` is populated and can be lost
        source.set_previous_past_quorums(quorums.clone(), 900, 950);
        source.set_previous_past_quorums(quorums.clone(), 990, 995);

        let saved = to_reduced_previous_quorums(&source).expect("should capture the history");

        let mut restored = SignatureVerificationQuorumSet::new(
            &ChainLockConfig::default_100_67(),
            PlatformVersion::latest(),
        )
        .expect("should build quorum set");
        restore_previous_quorums(&mut restored, Some(&saved)).expect("should restore");

        let source_previous = source.previous_past_quorums().expect("source has history");
        let restored_previous = restored
            .previous_past_quorums()
            .expect("restored must have history");

        assert_eq!(
            restored_previous.last_active_core_height,
            source_previous.last_active_core_height
        );
        assert_eq!(
            restored_previous.updated_at_core_height,
            source_previous.updated_at_core_height
        );
        assert_eq!(
            restored_previous.previous_change_height,
            source_previous.previous_change_height
        );
        assert_eq!(restored_previous.previous_change_height, Some(950));

        assert_eq!(
            restored_previous.quorums.len(),
            source_previous.quorums.len()
        );
        for (quorum_hash, source_quorum) in source_previous.quorums.iter() {
            let restored_quorum = restored_previous
                .quorums
                .get(quorum_hash)
                .expect("every quorum must be restored");
            assert_eq!(restored_quorum.public_key, source_quorum.public_key);
            assert_eq!(restored_quorum.index, source_quorum.index);
        }
    }

    /// A set with no history restores to no history, not to an empty one — an empty
    /// previous set would make `select_quorums` consider locks verifiable against nothing.
    #[test]
    fn should_leave_a_set_without_history_alone() {
        use crate::config::ChainLockConfig;

        let mut restored = SignatureVerificationQuorumSet::new(
            &ChainLockConfig::default_100_67(),
            PlatformVersion::latest(),
        )
        .expect("should build quorum set");
        restore_previous_quorums(&mut restored, None).expect("should restore");
        assert!(!restored.has_previous_past_quorums());
    }

    #[test]
    fn should_sort_validator_sets_into_saved_positions() {
        use dpp::bls_signatures::{Bls12381G2Impl, SecretKey};
        use dpp::core_types::validator_set::v0::ValidatorSetV0;
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        let mut rng = StdRng::seed_from_u64(7);
        let mut validator_sets: IndexMap<QuorumHash, ValidatorSet> = IndexMap::new();
        for seed in [1u8, 2, 3] {
            validator_sets.insert(
                quorum_hash(seed),
                ValidatorSet::V0(ValidatorSetV0 {
                    quorum_hash: quorum_hash(seed),
                    quorum_index: None,
                    core_height: 100,
                    members: Default::default(),
                    threshold_public_key: SecretKey::<Bls12381G2Impl>::random(&mut rng)
                        .public_key(),
                }),
            );
        }

        let saved_positions: Vec<Bytes32> = [3u8, 1, 2]
            .into_iter()
            .map(|seed| quorum_hash(seed).to_byte_array().into())
            .collect();

        sort_validator_sets_by_saved_positions(&mut validator_sets, &saved_positions);

        let order: Vec<QuorumHash> = validator_sets.keys().copied().collect();
        assert_eq!(order, vec![quorum_hash(3), quorum_hash(1), quorum_hash(2)]);
    }
}
