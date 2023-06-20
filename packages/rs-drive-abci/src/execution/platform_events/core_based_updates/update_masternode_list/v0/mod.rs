use dashcore_rpc::dashcore::hashes::Hash;
use dpp::block::block_info::BlockInfo;
use drive::grovedb::Transaction;
use crate::error::Error;
use crate::execution::types::update_state_masternode_list_outcome;
use crate::platform::{Platform, state};
use crate::rpc::core::CoreRPCLike;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Updates the masternode list in the platform state based on changes in the masternode list
    /// from Dash Core between two block heights.
    ///
    /// This function fetches the masternode list difference between the current core block height
    /// and the previous core block height, then updates the full masternode list and the
    /// HPMN (high performance masternode) list in the platform state accordingly.
    ///
    /// # Arguments
    ///
    /// * `state` - A mutable reference to the platform state to be updated.
    /// * `core_block_height` - The current block height in the Dash Core.
    /// * `transaction` - The current groveDB transaction.
    ///
    /// # Returns
    ///
    /// * `Result<(), Error>` - Returns `Ok(())` if the update is successful. Returns an error if
    ///   there is a problem fetching the masternode list difference or updating the state.
    fn update_masternode_list(
        &self,
        platform_state: Option<&state::v0::PlatformState>,
        block_platform_state: &mut state::v0::PlatformState,
        core_block_height: u32,
        is_init_chain: bool,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<(), Error> {
        if let Some(last_commited_block_info) =
            block_platform_state.last_committed_block_info.as_ref()
        {
            if core_block_height == last_commited_block_info.basic_info.core_height {
                tracing::debug!(
                    method = "update_masternode_list",
                    "no update mnl at height {}",
                    core_block_height
                );
                return Ok(()); // no need to do anything
            }
        }
        tracing::debug!(
            method = "update_masternode_list",
            "update mnl to height {} at block {}",
            core_block_height,
            block_platform_state.core_height()
        );
        if block_platform_state.last_committed_block_info.is_some() || is_init_chain {
            let update_state_masternode_list_outcome::v0::UpdateStateMasternodeListOutcome {
                masternode_list_diff,
                removed_masternodes,
            } = self.update_state_masternode_list(
                block_platform_state,
                core_block_height,
                is_init_chain,
            )?;

            self.update_masternode_identities_v0(
                masternode_list_diff,
                &removed_masternodes,
                block_info,
                platform_state,
                transaction,
            )?;

            if !removed_masternodes.is_empty() {
                self.drive.remove_validators_proposed_app_versions(
                    removed_masternodes
                        .into_keys()
                        .map(|pro_tx_hash| pro_tx_hash.into_inner()),
                    Some(transaction),
                )?;
            }
        }

        Ok(())
    }
}
