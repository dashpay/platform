use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::{CoreBlockHeight, TimestampMillis};
use std::time::{SystemTime, UNIX_EPOCH};

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determine initial core height.
    ///
    /// Use core height received from Tenderdash (from genesis.json) by default,
    /// otherwise we go with height of mn_rr fork.
    ///
    /// Core height is verified to ensure that it is both at or after mn_rr fork, and
    /// before or at last chain lock.
    ///
    /// ## Error handling
    ///
    /// This function will fail if:
    ///
    /// * mn_rr fork is not yet active
    /// * `requested` core height is before mn_rr fork
    /// * `requested` core height is after current best chain lock
    ///
    pub(in crate::execution::platform_events) fn initial_core_height_and_time_v0(
        &self,
        requested: Option<u32>,
    ) -> Result<(CoreBlockHeight, TimestampMillis), Error> {
        let fork_info = self.core_rpc.get_fork_info("mn_rr")?.ok_or(
            ExecutionError::InitializationForkNotActive("fork is not yet known".to_string()),
        )?;
        if !fork_info.active || fork_info.height.is_none() {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "fork is not yet known (currently {:?})",
                fork_info
            ))
            .into());
        } else {
            tracing::debug!(?fork_info, "core fork mn_rr is active");
        };
        // We expect height to present if the fork is active
        let mn_rr_fork_height = fork_info.height.unwrap();

        let initial_height = if let Some(requested) = requested {
            tracing::debug!(
                requested,
                mn_rr_fork_height,
                "initial core lock height is set in genesis"
            );

            requested
        } else {
            tracing::debug!(mn_rr_fork_height, "used fork height as initial core height");

            mn_rr_fork_height
        };

        // Make sure initial height is chain locked
        let chain_lock_height = self.core_rpc.get_best_chain_lock()?.block_height;

        if initial_height <= chain_lock_height {
            let block_time = self.core_rpc.get_block_time_from_height(initial_height)?;

            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards") // Copilot rocks :))
                .as_millis() as TimestampMillis;

            if block_time > current_time {
                return Err(ExecutionError::InitializationGenesisTimeInFuture {
                    initial_height,
                    genesis_time: block_time,
                    current_time,
                }
                .into());
            }

            Ok((initial_height, block_time))
        } else {
            Err(ExecutionError::InitializationHeightIsNotLocked {
                initial_height,
                chain_lock_height,
            }
            .into())
        }
    }
}
