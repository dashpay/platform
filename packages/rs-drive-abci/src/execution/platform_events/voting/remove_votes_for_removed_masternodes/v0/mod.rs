use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::hashes::Hash;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Removes the votes for removed masternodes
    pub(super) fn remove_votes_for_removed_masternodes_v0(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let masternode_list_changes =
            block_platform_state.full_masternode_list_changes(last_committed_platform_state);

        if !masternode_list_changes.removed_masternodes.is_empty() {
            self.drive.remove_all_votes_given_by_identities(
                masternode_list_changes
                    .removed_masternodes
                    .iter()
                    .map(|pro_tx_hash| pro_tx_hash.as_byte_array().to_vec())
                    .collect(),
                transaction,
                platform_version,
            )?;
        }

        Ok(())
    }
}
