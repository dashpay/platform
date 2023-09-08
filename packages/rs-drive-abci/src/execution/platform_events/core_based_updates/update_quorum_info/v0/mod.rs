use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;

use crate::platform_types::validator_set::v0::{ValidatorSetV0, ValidatorSetV0Getters};
use crate::platform_types::validator_set::ValidatorSet;
use crate::rpc::core::CoreRPCLike;

use dpp::dashcore::QuorumHash;
use std::cmp::Ordering;

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
        if !start_from_scratch && core_block_height == block_platform_state.core_height() {
            tracing::debug!(
                method = "update_quorum_info_v0",
                "no update quorum at height {}",
                core_block_height
            );
            return Ok(()); // no need to do anything
        }
        tracing::debug!(
            method = "update_quorum_info_v0",
            "update of quorums for height {}",
            core_block_height
        );
        let quorum_list = self
            .core_rpc
            .get_quorum_listextended(Some(core_block_height))?;
        let quorum_info = quorum_list
            .quorums_by_type
            .get(&self.config.quorum_type())
            .ok_or(Error::Execution(ExecutionError::DashCoreBadResponseError(
                format!(
                    "expected quorums of type {}, but did not receive any from Dash Core",
                    self.config.quorum_type
                ),
            )))?;

        tracing::debug!(
            method = "update_quorum_info_v0",
            "old {:?}",
            block_platform_state.validator_sets()
        );

        tracing::debug!(
            method = "update_quorum_info_v0",
            "new quorum_info {:?}",
            quorum_info
        );

        // Remove validator_sets entries that are no longer valid for the core block height
        block_platform_state
            .validator_sets_mut()
            .retain(|key, _| quorum_info.contains_key(key));

        // Fetch quorum info results and their keys from the RPC
        let mut quorum_infos = quorum_info
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

        // Map to quorums
        let new_quorums = quorum_infos
            .into_iter()
            .map(|(key, info_result)| {
                let validator_set = ValidatorSet::V0(ValidatorSetV0::try_from_quorum_info_result(
                    info_result,
                    block_platform_state,
                )?);
                Ok((key, validator_set))
            })
            .collect::<Result<Vec<_>, Error>>()?;
        // Add new validator_sets entries
        block_platform_state
            .validator_sets_mut()
            .extend(new_quorums.into_iter());

        block_platform_state
            .validator_sets_mut()
            .sort_by(|_, quorum_a, _, quorum_b| {
                let primary_comparison = quorum_b.core_height().cmp(&quorum_a.core_height());
                if primary_comparison == Ordering::Equal {
                    quorum_b
                        .quorum_hash()
                        .cmp(quorum_a.quorum_hash())
                        .then_with(|| quorum_b.core_height().cmp(&quorum_a.core_height()))
                } else {
                    primary_comparison
                }
            });

        tracing::debug!(
            method = "update_quorum_info_v0",
            "new {:?}",
            block_platform_state.validator_sets()
        );

        block_platform_state.set_quorums_extended_info(quorum_list.quorums_by_type);
        Ok(())
    }
}
