use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state;
use crate::rpc::core::CoreRPCLike;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// We need to validate against the platform state for rotation and not the block execution
    /// context state
    pub(in crate::execution) fn validator_set_update_v0(
        &self,
        platform_state: &platform_state::v0::PlatformState,
        block_execution_context: &mut block_execution_context::v0::BlockExecutionContext,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        let mut perform_rotation = false;

        if block_execution_context.block_state_info.height
            % self.config.validator_set_quorum_rotation_block_count as u64
            == 0
        {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: previous quorum finished members"
            );
            perform_rotation = true;
        }
        // we also need to perform a rotation if the validator set is being removed
        if block_execution_context
            .block_platform_state
            .validator_sets
            .get(&platform_state.current_validator_set_quorum_hash)
            .is_none()
        {
            tracing::debug!(
                method = "validator_set_update_v0",
                "rotation: new quorums not containing current quorum current {:?}, {}",
                block_execution_context
                    .block_platform_state
                    .validator_sets
                    .keys()
                    .map(|quorum_hash| format!("{}", quorum_hash)),
                &platform_state.current_validator_set_quorum_hash
            );
            perform_rotation = true;
        }

        //todo: perform a rotation if quorum health is low

        if perform_rotation {
            // get the index of the previous quorum
            let mut index = platform_state
                .validator_sets
                .get_index_of(&platform_state.current_validator_set_quorum_hash)
                .ok_or(Error::Execution(ExecutionError::CorruptedCachedState(
                    "current quorums do not contain current validator set",
                )))?;
            // we should rotate the quorum
            let quorum_count = platform_state.validator_sets.len();
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
                            .validator_sets
                            .get_index(index)
                            .expect("expected next validator set");
                        // We still have it in the state
                        if let Some(new_validator_set) = block_execution_context
                            .block_platform_state
                            .validator_sets
                            .get(quorum_hash)
                        {
                            tracing::debug!(
                                method = "validator_set_update_v0",
                                "rotation: to new quorum: {} with {} members",
                                &quorum_hash,
                                new_validator_set.members.len()
                            );
                            block_execution_context
                                .block_platform_state
                                .next_validator_set_quorum_hash = Some(*quorum_hash);
                            return Ok(Some(new_validator_set.into()));
                        }
                        index = (index + 1) % count;
                    }
                    // All quorums changed
                    if let Some((quorum_hash, new_quorum)) = block_execution_context
                        .block_platform_state
                        .validator_sets
                        .first()
                    {
                        block_execution_context
                            .block_platform_state
                            .next_validator_set_quorum_hash = Some(*quorum_hash);
                        tracing::debug!(
                            method = "validator_set_update_v0",
                            "rotation: all quorums changed, rotation to new quorum: {}",
                            &quorum_hash
                        );
                        return Ok(Some(new_quorum.into()));
                    }
                    tracing::debug!("no new quorums to choose from");
                    Ok(None)
                }
            }
        } else {
            let current_validator_set = block_execution_context
                .block_platform_state
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
                    "no validator set update"
                );
                Ok(None)
            }
        }
    }
}
