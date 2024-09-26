mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::ProTxHash;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Handles the update of an operator identity.
    ///
    /// An operator is an entity that has the right to operate a node within a blockchain
    /// network. This method is responsible for updating the details of the operator on the blockchain.
    ///
    /// There are three main attributes of an operator that can be changed: the public key,
    /// the payout address, and the platform node id.
    ///
    /// The `update_operator_identity` function is implemented as a versioned function.
    /// Different versions of the function are maintained to handle changes in the logic over time,
    /// and also to provide backwards compatibility.
    ///
    /// # Arguments
    ///
    /// * `masternode`: a tuple of ProTxHash and DMNStateDiff containing the masternode details and state difference.
    /// * `block_info`: an object containing information about the block where this operation is happening.
    /// * `platform_state`: the current state of the platform.
    /// * `transaction`: the transaction in which this operation is happening.
    /// * `drive_operations`: a mutable reference to a vector of DriveOperations.
    /// * `platform_version`: the current version of the platform.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the `update_operator_identity` function
    /// does not match any of the known versions.
    pub(in crate::execution::platform_events::core_based_updates::update_masternode_identities) fn update_operator_identity(
        &self,
        masternode_pro_tx_hash: &ProTxHash,
        pub_key_operator_change: Option<&Vec<u8>>,
        operator_payout_address_change: Option<Option<[u8; 20]>>,
        platform_node_id_change: Option<[u8; 20]>,
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
            .update_operator_identity
        {
            0 => self.update_operator_identity_v0(
                masternode_pro_tx_hash,
                pub_key_operator_change,
                operator_payout_address_change,
                platform_node_id_change,
                platform_state,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "update_operator_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
