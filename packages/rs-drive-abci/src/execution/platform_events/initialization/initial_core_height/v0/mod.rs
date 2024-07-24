use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;

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
    pub(in crate::execution::platform_events) fn initial_core_height_v0(
        &self,
        requested_height: Option<u32>,
    ) -> Result<u32, Error> {
        if let Some(height) = requested_height {
            tracing::debug!(
                initial_core_chain_locked_height = height,
                "genesis initial core chain locked height is set to {}, skip fork height initialization",
                height
            );

            let best_chain_locked_height = self.core_rpc.get_best_chain_lock()?.block_height;

            // TODO in my opinion, the condition should be:
            //
            // `mn_rr_fork <= requested && requested <= best`
            //
            // but it results in 1440 <=  1243 <= 1545
            //
            // So, fork_info.since differs? is it non-deterministic?
            if height > best_chain_locked_height {
                return Err(ExecutionError::InitializationBadCoreLockedHeight {
                    requested: height,
                    best: best_chain_locked_height,
                }
                .into());
            };

            return Ok(height);
        }

        let fork_info = self.core_rpc.get_fork_info("mn_rr")?.ok_or(
            ExecutionError::InitializationForkNotActive(
                "platform activation fork is not yet known".to_string(),
            ),
        )?;

        if !fork_info.active || fork_info.height.is_none() {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "platform activation fork is not yet known (currently {:?})",
                fork_info
            ))
            .into());
        };

        // We expect height to present if the fork is active
        let mn_rr_fork = fork_info.height.unwrap();

        tracing::debug!(
            mn_rr_fork,
            "used platform activation fork height as initial core lock height"
        );

        Ok(mn_rr_fork)
    }
}
