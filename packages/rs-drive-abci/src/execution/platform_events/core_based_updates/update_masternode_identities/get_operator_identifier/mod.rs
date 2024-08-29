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
    /// Retrieves the operator identifier using the provided MasternodeListItem.
    ///
    /// This function derives the operator identifier and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the MasternodeListItem.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Identifier, Error> - Returns the derived operator identifier if successful. Otherwise, returns an error.
    pub(super) fn get_operator_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identifier, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .get_operator_identifier_from_masternode_list_item
        {
            0 => Ok(Self::get_operator_identifier_from_masternode_list_item_v0(
                masternode,
            )),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_operator_identifier_from_masternode_list_item".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
