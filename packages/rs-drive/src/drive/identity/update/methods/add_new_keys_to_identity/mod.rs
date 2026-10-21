mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::IdentityPublicKey;

use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations for adding new keys to an identity. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity to which keys are to be added.
    /// * `unique_keys_to_add` - The unique keys to be added.
    /// * `non_unique_keys_to_add` - The non-unique keys to be added.
    /// * `with_references` - Whether to add with references.
    /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    #[allow(clippy::too_many_arguments)]
    pub fn add_new_keys_to_identity_operations(
        &self,
        identity_id: [u8; 32],
        unique_keys_to_add: Vec<IdentityPublicKey>,
        non_unique_keys_to_add: Vec<IdentityPublicKey>,
        with_references: bool,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .add_new_keys_to_identity
        {
            0 => self.add_new_keys_to_identity_operations_v0(
                identity_id,
                unique_keys_to_add,
                non_unique_keys_to_add,
                with_references,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_keys_to_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations for adding new non-unique keys to a masternode identity. This function is
    /// version controlled by the version of `add_new_keys_to_identity_operations`, whose
    /// operations it emits when estimating costs and for every key whose hash is not a unique
    /// key hash.
    ///
    /// A key whose hash is already a unique key hash is added without a non-unique key hash
    /// reference, matching how keys are stored when a masternode identity is created.
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
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .add_new_keys_to_identity
        {
            0 => self.add_new_keys_to_masternode_identity_operations_v0(
                identity_id,
                non_unique_keys_to_add,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_new_keys_to_masternode_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
