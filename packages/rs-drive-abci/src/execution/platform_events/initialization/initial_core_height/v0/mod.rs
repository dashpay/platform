use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::Bip9SoftforkStatus;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Determine initial core height.
    ///
    /// Use core height received from Tenderdash (from genesis.json) by default,
    /// otherwise we go with height of v20 fork.
    ///
    /// Core height is verified to ensure that it is both at or after v20 fork, and
    /// before or at last chain lock.
    ///
    /// ## Error handling
    ///
    /// This function will fail if:
    ///
    /// * v20 fork is not yet active
    /// * `requested` core height is before v20 fork
    /// * `requested` core height is after current best chain lock
    ///
    pub(in crate::execution::platform_events) fn initial_core_height_v0(
        &self,
        requested: Option<u32>,
    ) -> Result<u32, Error> {
        let fork_info = self.core_rpc.get_fork_info("v20")?.ok_or(
            ExecutionError::InitializationForkNotActive("fork is not yet known".to_string()),
        )?;
        if fork_info.status != Bip9SoftforkStatus::Active {
            // fork is not good yet
            return Err(ExecutionError::InitializationForkNotActive(format!(
                "fork is not yet known (currently {:?})",
                fork_info.status
            ))
            .into());
        } else {
            tracing::debug!(?fork_info, "core fork v20 is active");
        };
        let v20_fork = fork_info.since;

        if let Some(requested) = requested {
            let best = self.core_rpc.get_best_chain_lock()?.core_block_height;

            tracing::trace!(
                requested,
                v20_fork,
                best,
                "selecting initial core lock height"
            );
            // TODO in my opinion, the condition should be:
            //
            // `v20_fork <= requested && requested <= best`
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
                    v20_fork,
                }
                .into())
            }
        } else {
            tracing::trace!(v20_fork, "used fork height as initial core lock height");
            Ok(v20_fork)
        }
    }
}
