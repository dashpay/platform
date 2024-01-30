use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0MutableGetters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::rpc::core::CoreRPCLike;

use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// We need to validate against the platform state for rotation and not the block execution
    /// context state
    pub(super) fn validator_set_update_v0(
        &self,
        platform_state: &PlatformState,
        block_execution_context: &mut BlockExecutionContext,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        let mut perform_rotation = false;

        if block_execution_context.block_state_info().height()
            % self.config.execution.validator_set_rotation_block_count as u64
            == 0
        {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: previous quorum finished members. quorum rotation expected"
            );
            perform_rotation = true;
        }
        // we also need to perform a rotation if the validator set is being removed
        if block_execution_context
            .block_platform_state()
            .validator_sets()
            .get(&platform_state.current_validator_set_quorum_hash())
            .is_none()
        {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: new quorums not containing current quorum current {:?}, {}. quorum rotation expected˚",
                block_execution_context
                    .block_platform_state()
                    .validator_sets()
                    .keys()
                    .map(|quorum_hash| format!("{}", quorum_hash)),
                &platform_state.current_validator_set_quorum_hash()
            );
            perform_rotation = true;
        }

        //todo: perform a rotation if quorum health is low

        if perform_rotation {
            // get the index of the previous quorum
            let mut index = platform_state
                .validator_sets()
                .get_index_of(&platform_state.current_validator_set_quorum_hash())
                .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                    "current quorums do not contain current validator set",
                )))?;
            // we should rotate the quorum
            let quorum_count = platform_state.validator_sets().len();
            match quorum_count {
                0 => Err(Error::Execution(ExecutionError::CorruptedCachedState(
                    "no current quorums",
                ))),
                1 => Ok(None),
                count => {
                    let start_index = index;
                    index = (index + 1) % count;
                    // We can't just take the next item because it might no longer be in the state
                    while index != start_index {
                        let (quorum_hash, _) = platform_state
                            .validator_sets()
                            .get_index(index)
                            .expect("expected next validator set");

                        // We still have it in the state
                        if let Some(new_validator_set) = block_execution_context
                            .block_platform_state()
                            .validator_sets()
                            .get(quorum_hash)
                        {
                            tracing::debug!(
                                method = "validator_set_update_v0",
                                "rotation: to new quorum: {} with {} members",
                                &quorum_hash,
                                new_validator_set.members().len()
                            );
                            let validator_set_update = new_validator_set.into();
                            block_execution_context
                                .block_platform_state_mut()
                                .set_next_validator_set_quorum_hash(Some(*quorum_hash));
                            return Ok(Some(validator_set_update));
                        }
                        index = (index + 1) % count;
                    }
                    // All quorums changed
                    if let Some((quorum_hash, new_validator_set)) = block_execution_context
                        .block_platform_state()
                        .validator_sets()
                        .first()
                    {
                        tracing::debug!(
                            method = "validator_set_update_v0",
                            "rotation: all quorums changed, rotation to new quorum: {}",
                            &quorum_hash
                        );
                        let validator_set_update = new_validator_set.into();
                        let new_quorum_hash = *quorum_hash;
                        block_execution_context
                            .block_platform_state_mut()
                            .set_next_validator_set_quorum_hash(Some(new_quorum_hash));
                        return Ok(Some(validator_set_update));
                    }
                    tracing::debug!("no new quorums to choose from");
                    Ok(None)
                }
            }
        } else {
            let current_validator_set = block_execution_context
                .block_platform_state()
                .current_validator_set()?;
            if current_validator_set != platform_state.current_validator_set()? {
                // Something changed, for example the IP of a validator changed, or someone's ban status

                tracing::debug!(
                    method = "validator_set_update_v0",
                    "validator set update without rotation"
                );
                Ok(Some(current_validator_set.into()))
            } else {
                tracing::debug!(
                    method = "validator_set_update_v0",
                    "no validator set update",
                );
                Ok(None)
            }
        }
    }
}
