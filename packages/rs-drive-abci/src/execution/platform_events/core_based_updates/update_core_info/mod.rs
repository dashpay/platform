mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the core information in the platform state based on the given core block height.
    ///
    /// This function updates both the masternode list and the quorum information in the platform
    /// state. It calls the update_masternode_list and update_quorum_info functions to perform
    /// the respective updates.
    ///
    /// # Arguments
    ///
    /// * platform_state - A reference to the platform state before execution of current block.
    /// * block_platform_state - A mutable reference to the current platform state in the block
    ///   execution context to be updated.
    /// * core_block_height - The current block height in the Dash Core.
    /// * is_init_chain - A boolean indicating if the chain is being initialized.
    /// * block_info - A reference to the block information.
    /// * transaction - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    ///   there is a problem updating the masternode list, quorum information, or the state.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn update_core_info(
        &self,
        platform_state: Option<&PlatformState>,
        block_platform_state: &mut PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .update_core_info
        {
            0 => self.update_core_info_v0(
                platform_state,
                block_platform_state,
                core_block_height,
                is_init_chain,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_core_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Rebuilds the in-memory Core-derived state (masternode lists and every quorum
    /// set) from scratch at `core_block_height`, without touching GroveDB.
    ///
    /// State sync reconstruction uses this: the restored GroveDB already holds every
    /// masternode identity exactly as the source chain wrote it, so the identity writes
    /// `update_core_info` would issue are at best no-ops and at worst a root-hash
    /// mismatch. Only the platform state, which is not replicated, has to be rebuilt.
    /// Not consensus code, hence unversioned.
    pub(crate) fn rebuild_core_info_in_memory(
        &self,
        state: &mut PlatformState,
        core_block_height: u32,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.update_state_masternode_list_v0(state, core_block_height, true)?;
        self.update_quorum_info(None, state, core_block_height, true, platform_version)
    }
}
