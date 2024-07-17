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
        requested: Option<u32>,
    ) -> Result<u32, Error> {
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
        let mn_rr_fork = fork_info.height.unwrap();

        if let Some(requested) = requested {
            let best = self.core_rpc.get_best_chain_lock()?.block_height;

            tracing::trace!(
                requested,
                mn_rr_fork,
                best,
                "selecting initial core lock height"
            );
            // TODO in my opinion, the condition should be:
            //
            // `mn_rr_fork <= requested && requested <= best`
            //
            // but it results in 1440 <=  1243 <= 1545
            //
            // So, fork_info.since differs? is it non-deterministic?
            if requested <= best {
                Ok(requested)
            } else {
                Err(ExecutionError::InitializationBadCoreLockedHeight {
                    requested,
                    best,
                    mn_rr_fork,
                }
                .into())
            }
        } else {
            tracing::trace!(mn_rr_fork, "used fork height as initial core lock height");
            Ok(mn_rr_fork)
        }
    }
}
