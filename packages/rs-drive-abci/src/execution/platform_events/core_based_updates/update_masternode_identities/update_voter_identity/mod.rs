mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;

use dashcore_rpc::dashcore::ProTxHash;

use dashcore_rpc::json::DMNStateDiff;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use drive::util::batch::DriveOperation;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the voter identity in the masternode.
    ///
    /// This function updates the voter identity in the masternode. The old identity key is
    /// disabled (which might make the identity unusable) and a new identity is added with
    /// the new key, which is a non-unique key.
    ///
    /// # Arguments
    ///
    /// * masternode - A reference to the masternode to be updated.
    /// * block_info - A reference to the block information.
    /// * platform_state - A reference to the platform state.
    /// * transaction - The current groveDB transaction.
    /// * drive_operations - A mutable reference to the Drive operations vector.
    /// * platform_version - A reference to the platform version.
    ///
    /// # Returns
    ///
    /// * Result<(), Error> - Returns Ok(()) if the update is successful. Returns an error if
    /// there is a problem updating the voter identity.
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn update_voter_identity(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        platform_state: &PlatformState,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .update_voter_identity
        {
            0 => self.update_voter_identity_v0(
                masternode,
                block_info,
                platform_state,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_voter_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
