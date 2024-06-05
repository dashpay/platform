use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dashcore_rpc::json::{ExtendedQuorumListResult, QuorumType};
use std::collections::BTreeMap;

use crate::platform_types::validator_set::v0::{
    ValidatorSetV0, ValidatorSetV0Getters, ValidatorSetV0Setters,
};
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;

use crate::platform_types::core_quorum_set::{CoreQuorumSet, CoreQuorumSetV0Methods, Quorum};
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use dpp::dashcore::QuorumHash;
use tracing::Level;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the quorum information for the platform state based on the given core block height.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state.
    /// * `core_block_height` - The core block height for which to update the quorum information.
    ///
    /// # Returns
    ///
    /// * `Result<SimpleConsensusValidationResult, ExecutionError>` - A `SimpleConsensusValidationResult`
    ///   on success, or an `Error` on failure.
    pub(super) fn update_quorum_info_v0(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<(), Error> {
        let _span = tracing::span!(Level::TRACE, "update_quorum_info", core_block_height).entered();

        let last_committed_core_height = block_platform_state.last_committed_core_height();

        if start_from_scratch {
            tracing::debug!("update quorum info from scratch up to {core_block_height}");
        } else if core_block_height != last_committed_core_height {
            tracing::debug!(
                previous_core_block_height = last_committed_core_height,
                "update quorum info from {} to {}",
                last_committed_core_height,
                core_block_height
            );
        } else {
            tracing::debug!("quorum info at height {core_block_height} already updated");

            return Ok(()); // no need to do anything
        }

        // We request the quorum list from the current core block height, this is because we also keep
        // the previous chain lock validating quorum. Core will sign from 8 blocks before the current
        // core block height, so often we will use the previous chain lock validating quorums instead.

        let mut extended_quorum_list = self
            .core_rpc
            .get_quorum_listextended(Some(core_block_height))?;

        let validator_set_quorum_type = self.config.validator_set.quorum_type;
        let chain_lock_quorum_type = self.config.chain_lock.quorum_type;
        let instant_lock_quorum_type = self.config.instant_lock.quorum_type;

        let validator_quorums_list: BTreeMap<_, _> = extended_quorum_list
            .quorums_by_type
            .remove(&validator_set_quorum_type)
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    self.config.validator_set.quorum_type
                ),
            )))?
            .into_iter()
            .collect();

        let mut removed_a_validator_set = false;

        // Remove validator_sets entries that are no longer valid for the core block height
        // and update quorum index for existing validator sets
        block_platform_state
            .validator_sets_mut()
            .retain(|quorum_hash, validator_set| {
                let retain = match validator_quorums_list.get(quorum_hash) {
                    Some(quorum_details) => {
                        validator_set.set_quorum_index(quorum_details.quorum_index);
                        true
                    }
                    None => false,
                };

                removed_a_validator_set |= !retain;

                if !retain {
                    tracing::trace!(
                        ?quorum_hash,
                        quorum_type = ?self.config.validator_set.quorum_type,
                        "removed validator set {} with quorum type {}",
                        quorum_hash,
                        self.config.validator_set.quorum_type
                    )
                }

                retain
            });

        // Fetch quorum info and their keys from the RPC for new quorums
        let mut quorum_infos = validator_quorums_list
            .into_iter()
            .filter(|(key, _)| {
                !block_platform_state
                    .validator_sets()
                    .contains_key::<QuorumHash>(key)
            })
            .map(|(key, _)| {
                let quorum_info_result = self.core_rpc.get_quorum_info(
                    self.config.validator_set.quorum_type,
                    &key,
                    None,
                )?;

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
                let validator_set = ValidatorSet::V0(ValidatorSetV0::try_from_quorum_info_result(
                    info_result,
                    block_platform_state,
                )?);

                tracing::trace!(
                    ?validator_set,
                    ?quorum_hash,
                    quorum_type = ?self.config.validator_set.quorum_type,
                    "add new validator set {} with quorum type {}",
                    quorum_hash,
                    self.config.validator_set.quorum_type
                );

                Ok((quorum_hash, validator_set))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let is_validator_set_updated = !new_validator_sets.is_empty() || removed_a_validator_set;

        // Add new validator_sets entries
        block_platform_state
            .validator_sets_mut()
            .extend(new_validator_sets);

        // Sort all validator sets into deterministic order by core block height of creation
        block_platform_state
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

        // Update Chain Lock quorums

        // Use already updated validator sets if we use the same quorums
        if validator_set_quorum_type == chain_lock_quorum_type {
            // Update only in case if there are any changes
            if is_validator_set_updated {
                let quorums = block_platform_state
                    .validator_sets()
                    .iter()
                    .map(|(quorum_hash, validator_set)| {
                        (
                            *quorum_hash,
                            validator_set.threshold_public_key().clone(),
                            validator_set.quorum_index(),
                        )
                    })
                    .collect();

                let quorum_set = block_platform_state.chain_lock_validating_quorums_mut();

                tracing::trace!("updated chain lock validating quorums to current validator set");

                if !start_from_scratch {
                    // we already have state, so we update last and previous quorums
                    quorum_set.replace_quorums(
                        quorums,
                        last_committed_core_height,
                        core_block_height,
                    );
                } else {
                    // the only case where there will be no platform_state is init chain,
                    // so there is no previous quorums to update
                    quorum_set.set_current_quorums(quorums)
                }
            }
        } else {
            // Update quorums from the extended quorum list
            self.update_quorums_from_quorum_list(
                "chain lock",
                chain_lock_quorum_type,
                block_platform_state.chain_lock_validating_quorums_mut(),
                platform_state,
                &extended_quorum_list,
                last_committed_core_height,
                core_block_height,
            )?;
        }

        // Update Instant Lock quorums

        // Use already updated validator sets if we use the same quorums
        if validator_set_quorum_type == instant_lock_quorum_type {
            // Update only in case if there are any changes
            if is_validator_set_updated {
                let quorums = block_platform_state
                    .validator_sets()
                    .iter()
                    .map(|(quorum_hash, validator_set)| {
                        (
                            *quorum_hash,
                            validator_set.threshold_public_key().clone(),
                            validator_set.quorum_index(),
                        )
                    })
                    .collect();

                let quorum_set = block_platform_state.instant_lock_validating_quorums_mut();

                tracing::trace!("updated instant lock validating quorums to current validator set");

                if !start_from_scratch {
                    // we already have state, so we update last and previous quorums
                    quorum_set.replace_quorums(
                        quorums,
                        last_committed_core_height,
                        core_block_height,
                    );
                } else {
                    // the only case where there will be no platform_state is init chain,
                    // so there is no previous quorums to update
                    quorum_set.set_current_quorums(quorums)
                }
            }
        } else {
            // Update quorums from the extended quorum list
            self.update_quorums_from_quorum_list(
                "instant lock",
                instant_lock_quorum_type,
                block_platform_state.instant_lock_validating_quorums_mut(),
                platform_state,
                &extended_quorum_list,
                last_committed_core_height,
                core_block_height,
            )?;
        }

        Ok(())
    }

    fn update_quorums_from_quorum_list(
        &self,
        name: &str,
        quorum_type: QuorumType,
        quorum_set: &mut CoreQuorumSet,
        platform_state: Option<&PlatformState>,
        full_quorum_list: &ExtendedQuorumListResult,
        last_committed_core_height: u32,
        next_core_height: u32,
    ) -> Result<(), Error> {
        let quorums_list: BTreeMap<_, _> = full_quorum_list
            .quorums_by_type
            .get(&quorum_type)
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    quorum_type
                ),
            )))?
            .iter()
            .map(|(quorum_hash, extended_quorum_details)| {
                (quorum_hash, extended_quorum_details.quorum_index)
            })
            .collect();

        let mut removed_a_validating_quorum = false;

        // Remove validating_quorums entries that are no longer valid for the core block height
        // and update quorum index for existing validator sets
        quorum_set
            .current_quorums_mut()
            .retain(|quorum_hash, quorum| {
                let retain = match quorums_list.get(quorum_hash) {
                    Some(index) => {
                        quorum.index = *index;
                        true
                    }
                    None => false,
                };

                if !retain {
                    tracing::trace!(
                        ?quorum_hash,
                        quorum_type = ?quorum_type,
                        "removed old {name} quorum {} with quorum type {}",
                        quorum_hash,
                        quorum_type
                    );
                }
                removed_a_validating_quorum |= !retain;
                retain
            });

        // Fetch quorum info and their keys from the RPC for new quorums
        let quorum_infos = quorums_list
            .into_iter()
            .filter(|(key, _)| !quorum_set.current_quorums().contains_key::<QuorumHash>(key))
            .map(|(key, _)| {
                let quorum_info_result = self.core_rpc.get_quorum_info(quorum_type, key, None)?;

                Ok((*key, quorum_info_result))
            })
            .collect::<Result<Vec<_>, Error>>()?;

        let added_a_validating_quorum = !quorum_infos.is_empty();

        if added_a_validating_quorum {
            // Map to validating quorums
            let new_quorums = quorum_infos
                .into_iter()
                .map(|(quorum_hash, info_result)| {
                    let public_key =
                        match BlsPublicKey::from_bytes(info_result.quorum_public_key.as_slice())
                            .map_err(ExecutionError::BlsErrorFromDashCoreResponse)
                        {
                            Ok(public_key) => public_key,
                            Err(e) => return Err(e.into()),
                        };

                    tracing::trace!(
                        ?public_key,
                        ?quorum_hash,
                        quorum_type = ?quorum_type,
                        "add new {name} quorum {} with quorum type {}",
                        quorum_hash,
                        quorum_type
                    );

                    let index = if info_result.quorum_index == 0 {
                        None
                    } else {
                        Some(info_result.quorum_index)
                    };

                    Ok((quorum_hash, Quorum { public_key, index }))
                })
                .collect::<Result<Vec<_>, Error>>()?;

            // Add new validator_sets entries
            quorum_set.current_quorums_mut().extend(new_quorums);
        }

        if added_a_validating_quorum || removed_a_validating_quorum {
            if let Some(old_state) = platform_state {
                let previous_validating_quorums =
                    old_state.chain_lock_validating_quorums().current_quorums();

                quorum_set.update_previous_quorums(
                    previous_validating_quorums.clone(),
                    last_committed_core_height,
                    next_core_height,
                );
            }
        }

        Ok(())
    }
}
