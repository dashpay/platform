mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identity::KeyID;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::block::epoch::Epoch;
use std::collections::HashMap;

impl Drive {
    /// Re-enables identity keys.
    ///
    /// Depending on the version specified in the `drive_version` parameter, this method
    /// will route the request to the correct versioned implementation.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the identity whose keys are to be re-enabled.
    /// * `key_ids` - The vector of keys to be re-enabled.
    /// * `estimated_costs_only_with_layer_info` - Optional parameter that contains the estimated costs.
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version configuration, which determines the version of
    ///                      the method to be used.
    ///
    /// # Returns
    ///
    /// On success, it will return a vector of low level drive operations.
    /// On error, it will return a relevant error.
    pub(crate) fn re_enable_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
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
            .re_enable_identity_keys
        {
            0 => self.re_enable_identity_keys_operations_v0(
                identity_id,
                key_ids,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "re_enable_identity_keys_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
