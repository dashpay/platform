mod v0;
mod v1;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::estimated_layers::EstimatedLayers;
use dpp::identity::IdentityPublicKey;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The operations for adding new non-unique keys to a masternode identity, version controlled
    /// by `add_new_keys_to_masternode_identity`: 0 up to protocol version 14, where it followed
    /// `add_new_keys_to_identity`, which is 0 there too. Version 0 emits the operations of
    /// `add_new_keys_to_identity_operations` when estimating costs and for every key whose hash is
    /// not a unique key hash.
    ///
    /// A key whose hash is already a unique key hash is added without a non-unique key hash
    /// reference, matching how keys are stored when a masternode identity is created. From
    /// version 1, a key whose hash the identity already holds a non-unique key hash reference
    /// for, or that an earlier key of the same call has, is also added without a reference. The
    /// owner, voter and operator identities of masternodes all add keys through this.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the masternode identity to which keys are to be added.
    /// * `non_unique_keys_to_add` - The keys to be added, all of non-unique key types.
    /// * `epoch` - The current epoch.
    /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    pub fn add_new_keys_to_masternode_identity_operations(
        &self,
        identity_id: [u8; 32],
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<EstimatedLayers>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .add_new_keys_to_masternode_identity
        {
            0 => self.add_new_keys_to_masternode_identity_operations_v0(
                identity_id,
                non_unique_keys_to_add,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            1 => self.add_new_keys_to_masternode_identity_operations_v1(
                identity_id,
                non_unique_keys_to_add,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_keys_to_masternode_identity_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
