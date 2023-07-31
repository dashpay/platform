mod v0;

use dashcore_rpc::dashcore_rpc_json::MasternodeListItem;
use dpp::dashcore::hashes::Hash;
use crate::error::Error;
use crate::error::execution::ExecutionError;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;

impl<C> Platform<C>
    where
        C: CoreRPCLike,
{
    /// Retrieves a voter identifier using the provided transaction hash and voting address.
    ///
    /// This function derives the voter identifier and delegates to a version-specific method depending on the platform version.
    ///
    /// # Arguments
    ///
    /// * pro_tx_hash - A reference to the transaction hash.
    /// * voting_address - A reference to the voting address.
    /// * platform_version - The version of the platform to determine which method to delegate to.
    ///
    /// # Returns
    ///
    /// * Result<[u8; 32], Error> - Returns the derived voter identifier if successful. Otherwise, returns an error.
    pub(crate) fn get_voter_identifier(
        pro_tx_hash: &[u8; 32],
        voting_address: &[u8; 20],
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates.masternode_updates
            .get_voter_identifier
        {
            0 => Self::get_voter_identifier_v0(pro_tx_hash, voting_address, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_voter_identifier".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

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
    /// * Result<[u8; 32], Error> - Returns the derived voter identifier if successful. Otherwise, returns an error.
    pub(crate) fn get_voter_identifier_from_masternode_list_item(
        masternode: &MasternodeListItem,
        platform_version: &PlatformVersion,
    ) -> Result<[u8; 32], Error> {
        match platform_version
            .drive_abci
            .methods
            .identity_methods
            .get_voter_identifier_from_masternode_list_item
        {
            0 => Self::get_voter_identifier_from_masternode_list_item_v0(masternode, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "get_voter_identifier_from_masternode_list_item".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}