use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
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
    /// execution context to be updated.
    /// * core_block_height - The current block height in the Dash Core.
    /// * is_init_chain - A boolean indicating if the chain is being initialized.
    /// * block_info - A reference to the block information.
    /// * transaction - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    /// there is a problem updating the masternode list, quorum information, or the state.
    pub(crate) fn update_core_info_v0(
        &self,
        platform_state: Option<&platform_state::v0::PlatformState>,
        block_platform_state: &mut platform_state::v0::PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        self.update_masternode_list_v0(
            platform_state,
            block_platform_state,
            core_block_height,
            is_init_chain,
            block_info,
            transaction,
        )?;

        self.update_quorum_info_v0(block_platform_state, core_block_height, false)
    }
}
