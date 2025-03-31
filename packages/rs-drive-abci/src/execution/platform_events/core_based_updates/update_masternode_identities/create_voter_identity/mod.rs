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
    /// Creates a voter identity using the provided transaction hash and voting key.
    ///
    /// This function constructs a voter identity and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * pro_tx_hash - A reference to the transaction hash.
    /// * voting_key - A reference to the voting key.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Identity, Error> - Returns the constructed identity for the voter if successful.
    ///   Otherwise, returns an error.
    pub(crate) fn create_voter_identity(
        pro_tx_hash: &[u8; 32],
        voting_key: &[u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .create_voter_identity
        {
            0 => Self::create_voter_identity_v0(pro_tx_hash, voting_key, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_voter_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates a voter identity based on the provided masternode list item.
    ///
    /// This function constructs a voter identity for a masternode and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode list item.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<Identity, Error> - Returns the constructed identity for the voter if successful.
    ///   Otherwise, returns an error.
    pub(crate) fn create_voter_identity_from_masternode_list_item(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<Identity, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .create_voter_identity
        {
            0 => Self::create_voter_identity_from_masternode_list_item_v0(
                masternode,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "create_voter_identity_from_masternode_list_item".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
