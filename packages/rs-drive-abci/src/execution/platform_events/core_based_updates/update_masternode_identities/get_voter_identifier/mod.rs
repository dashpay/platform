mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identifier::Identifier;

use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Retrieves a voter identifier based on the provided masternode list item.
    ///
    /// This function derives the voter identifier for a masternode and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode list item.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Identifier, Error> - Returns the derived voter identifier if successful. Otherwise, returns an error.
    pub(crate) fn get_voter_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .get_voter_identifier_from_masternode_list_item
        {
            0 => Ok(Self::get_voter_identifier_from_masternode_list_item_v0(
                masternode,
            )),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_voter_identifier_from_masternode_list_item".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
