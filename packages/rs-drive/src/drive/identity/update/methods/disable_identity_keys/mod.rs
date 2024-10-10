mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identity::KeyID;
use dpp::prelude::TimestampMillis;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::block::epoch::Epoch;
use std::collections::HashMap;

impl Drive {
    /// Disables identity keys. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose keys are to be disabled.
    /// * `keys_ids` - The IDs of the keys to be disabled.
    /// * `disable_at` - The time at which to disable the keys.
    /// * `block_info` - The current block information.
    /// * `apply` - Whether to apply the change.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The resulting fee if successful, or an error.
    pub fn disable_identity_keys(
        &self,
        identity_id: [u8; 32],
        keys_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .disable_identity_keys
        {
            0 => self.disable_identity_keys_v0(
                identity_id,
                keys_ids,
                disable_at,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "disable_identity_keys".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Disables identity key operations. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose keys are to be disabled.
    /// * `key_ids` - The IDs of the keys to be disabled.
    /// * `disable_at` - The time at which to disable the keys.
    /// * `estimated_costs_only_with_layer_info` - The estimated costs with layer information.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    pub fn disable_identity_keys_operations(
        &self,
        identity_id: [u8; 32],
        key_ids: Vec<KeyID>,
        disable_at: TimestampMillis,
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
            .disable_identity_keys
        {
            0 => self.disable_identity_keys_operations_v0(
                identity_id,
                key_ids,
                disable_at,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "disable_identity_keys_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
