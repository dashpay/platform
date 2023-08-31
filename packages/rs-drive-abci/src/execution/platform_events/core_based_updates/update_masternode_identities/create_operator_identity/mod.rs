mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::identity::Identity;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Creates an operator identity based on the given masternode list item.
    ///
    /// This function constructs an identity for an operator using details from the masternode.
    /// It delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode list item.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Identity, Error> - Returns the constructed identity for the operator if successful.
    /// Otherwise, returns an error.
    pub(crate) fn create_operator_identity(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .create_operator_identity
        {
            0 => Self::create_operator_identity_v0(masternode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_operator_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
