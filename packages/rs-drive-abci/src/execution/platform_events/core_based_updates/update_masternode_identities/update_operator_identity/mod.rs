mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::dashcore::ProTxHash;

use dpp::version::PlatformVersion;
use drive::util::batch::DriveOperation;

use drive::grovedb::Transaction;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Handles the update of an operator identity.
    /// Updates the operator identity when the masternode state reports a change.
    ///
    /// The operator may rotate its BLS operator key, change the payout address, or
    /// update the platform node identifier. This method orchestrates the Drive
    /// operations required to reflect those updates on chain, delegating the actual
    /// logic to the versioned implementation.
    ///
    /// # Arguments
    ///
    /// * `masternode_pro_tx_hash` - ProTx hash of the masternode whose identity is being updated.
    /// * `pub_key_operator_change` - Optional new operator public key; `None` when the key is unchanged.
    /// * `operator_payout_address_change` - Optional new payout address (`Some(None)` clears it).
    /// * `platform_node_id_change` - Optional new platform node identifier.
    /// * `platform_state` - View of the cached platform state used to look up prior data.
    /// * `transaction` - GroveDB transaction under which Drive operations will run.
    /// * `drive_operations` - Accumulator for Drive batch operations to perform the update.
    /// * `platform_version` - Platform version providing dispatch to the proper implementation.
    ///
    /// # Errors
    ///
    /// This function will return an error if the version of the `update_operator_identity` function
    /// does not match any of the known versions.
    #[allow(clippy::too_many_arguments)]
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
