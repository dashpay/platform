use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use std::collections::BTreeMap;

use crate::platform_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;

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
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        start_from_scratch: bool,
    ) -> Result<(), Error> {
        let _span = tracing::span!(Level::TRACE, "update_quorum_info", core_block_height).entered();

        if start_from_scratch {
            tracing::debug!("update quorum info from scratch up to {core_block_height}");
        } else if core_block_height != block_platform_state.core_height() {
            tracing::debug!(
                previous_core_block_height = block_platform_state.core_height(),
                "update quorum info from {} to {}",
                block_platform_state.core_height(),
                core_block_height
            );
        } else {
            tracing::debug!("quorum info at height {core_block_height} already updated");

            return Ok(()); // no need to do anything
        }

        let mut extended_quorum_list = self
            .core_rpc
            .get_quorum_listextended(Some(core_block_height))?;

        let validator_quorums_list: BTreeMap<_, _> = extended_quorum_list
            .quorums_by_type
            .remove(&self.config.quorum_type())
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    self.config.quorum_type
                ),
            )))?
            .into_iter()
            .collect();

        // Remove validator_sets entries that are no longer valid for the core block height
        block_platform_state
            .validator_sets_mut()
            .retain(|quorum_hash, _| {
                let has_quorum = validator_quorums_list.contains_key::<QuorumHash>(quorum_hash);

                if has_quorum {
                    tracing::trace!(
                        ?quorum_hash,
                        quorum_type = ?self.config.quorum_type(),
                        "remove validator set {} with quorum type {}",
                        quorum_hash,
                        self.config.quorum_type()
                    )
                }

                has_quorum
            });

        // Fetch quorum info and their keys from the RPC for new quorums
        let mut quorum_infos = validator_quorums_list
            .iter()
            .filter(|(key, _)| {
                !block_platform_state
                    .validator_sets()
                    .contains_key::<QuorumHash>(key)
            })
            .map(|(key, _)| {
                let quorum_info_result =
                    self.core_rpc
                        .get_quorum_info(self.config.quorum_type(), key, None)?;

                Ok((*key, quorum_info_result))
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
                    quorum_type = ?self.config.quorum_type(),
                    "add new validator set {} with quorum type {}",
                    quorum_hash,
                    self.config.quorum_type()
                );

                Ok((quorum_hash, validator_set))
            })
            .collect::<Result<Vec<_>, Error>>()?;

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

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(
                method = "update_quorum_info_v0",
                block_platform_state_fingerprint = hex::encode(block_platform_state.fingerprint()),
                "quorum info updated",
            );
        }

        Ok(())
    }
}
