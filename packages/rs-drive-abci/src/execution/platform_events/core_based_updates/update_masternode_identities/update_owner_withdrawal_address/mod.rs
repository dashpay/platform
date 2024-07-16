use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::ProTxHash;
use dashcore_rpc::json::DMNStateDiff;
use dpp::block::block_info::BlockInfo;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Updates the owner's withdrawal address.
    ///
    /// This function is responsible for updating the withdrawal address of the owner.
    /// The withdrawal address is a key attribute for any owner in a masternode.
    ///
    /// The change is first validated and then reflected on the blockchain.
    /// The function is implemented as a versioned function to allow changes to the
    /// implementation over time while maintaining backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `masternode`: a tuple of ProTxHash and DMNStateDiff containing the masternode details and state difference.
    /// * `block_info`: an object containing information about the block where this operation is happening.
    /// * `transaction`: the transaction in which this operation is happening.
    /// * `drive_operations`: a mutable reference to a vector of DriveOperations.
    /// * `platform_version`: the current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if there is no existing withdrawal address or
    /// if the existing withdrawal address is already disabled.
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn update_owner_withdrawal_address(
        &self,
        masternode: &(ProTxHash, DMNStateDiff),
        block_info: &BlockInfo,
        transaction: &Transaction,
        drive_operations: &mut Vec<DriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .core_based_updates
            .masternode_updates
            .update_owner_withdrawal_address
        {
            0 => self.update_owner_withdrawal_address_v0(
                masternode,
                block_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_owner_withdrawal_address".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
