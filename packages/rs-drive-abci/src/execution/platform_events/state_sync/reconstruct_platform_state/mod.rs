use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Reconstructs the full in-memory platform state after a state sync snapshot
    /// restore, from the reduced platform state contained in the restored grovedb
    /// state, and persists it to aux storage so it survives restarts.
    ///
    /// Must not change the grovedb root hash: the caller compares the root hash against
    /// the snapshot app hash after this returns.
    pub fn reconstruct_platform_state(
        &self,
        _app_hash: &[u8; 32],
        _platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // TODO(state-sync): implemented in the follow-up commit that ports the platform
        // state reconstruction (reduced state fetch + update_core_info re-derivation).
        Err(AbciError::StateSyncInternalError(
            "platform state reconstruction is not implemented yet".to_string(),
        )
        .into())
    }
}
