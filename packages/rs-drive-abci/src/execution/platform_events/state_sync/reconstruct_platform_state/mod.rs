use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
use crate::platform_types::signature_verification_quorum_set::SignatureVerificationQuorumSet;
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;
use dpp::block::extended_block_info::v0::{ExtendedBlockInfoV0, ExtendedBlockInfoV0Getters};
use dpp::block::extended_block_info::ExtendedBlockInfo;
use dpp::dashcore::hashes::Hash;
use dpp::dashcore::QuorumHash;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::platform_value::Bytes32;
use dpp::reduced_platform_state::ReducedPlatformState;
use dpp::version::fee::FeeVersion;
use dpp::version::PlatformVersion;
use indexmap::IndexMap;
use std::collections::BTreeMap;

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

        // Core RPC returns quorums in an order that need not match the incremental
        // order the source node maintained; restore the recorded order.
        sort_validator_sets_by_saved_positions(
            platform_state.validator_sets_mut(),
            &saved.quorum_positions,
        );

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
