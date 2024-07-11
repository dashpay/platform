use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::block_execution_context::v0::{
    BlockExecutionContextV0Getters, BlockExecutionContextV0MutableGetters,
};
use crate::execution::types::block_execution_context::BlockExecutionContext;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::validator_set::v0::ValidatorSetV0Getters;
use crate::rpc::core::CoreRPCLike;
use itertools::Itertools;

use dpp::dashcore::hashes::Hash;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// We need to validate against the platform state for rotation and not the block execution
    /// context state
    #[inline(always)]
    pub(super) fn validator_set_update_v0(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        platform_state: &PlatformState,
        block_execution_context: &mut BlockExecutionContext,
    ) -> Result<Option<ValidatorSetUpdate>, Error> {
        let mut perform_rotation = false;

        if let Some(validator_set) = block_execution_context
            .block_platform_state()
            .validator_sets()
            .get(&platform_state.current_validator_set_quorum_hash())
        {
            if let Some((last_member_pro_tx_hash, _)) = validator_set.members().last_key_value() {
                // we should also perform a rotation if the validator set went through all quorum members
                // this means we are at the last member of the quorum
                if last_member_pro_tx_hash.as_byte_array() == &proposer_pro_tx_hash {
                    tracing::debug!(
                    method = "validator_set_update_v0",
                    "rotation: quorum finished as we hit last member {} of quorum {}. All known quorums are: [{}]. quorum rotation expected",
                    hex::encode(proposer_pro_tx_hash),
                        hex::encode(platform_state.current_validator_set_quorum_hash().as_byte_array()),
                    block_execution_context
                    .block_platform_state()
                    .validator_sets()
                    .keys()
                    .map(hex::encode).collect::<Vec<_>>().join(" | "),
                );
                    perform_rotation = true;
                }
            } else {
                // the validator set has no members, very weird, but let's just perform a rotation
                tracing::debug!(
                    method = "validator_set_update_v0",
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
                    method = "validator_set_update_v0",
                "rotation: quorum finished as we hit last an earlier member {} than last block proposer {} for quorum {}. All known quorums are: [{}]. quorum rotation expected",
                hex::encode(proposer_pro_tx_hash),
                    hex::encode(block_execution_context.block_platform_state().last_committed_block_proposer_pro_tx_hash()),
                    hex::encode(platform_state.current_validator_set_quorum_hash().as_byte_array()),
                block_execution_context
                .block_platform_state()
                .validator_sets()
                .keys()
                .map(hex::encode).collect::<Vec<_>>().join(" | "),
                );
                perform_rotation = true;
            }
        } else {
            // we also need to perform a rotation if the validator set is being removed
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
                1 => Ok(None), // no rotation as we are the only quorum
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
