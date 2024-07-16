mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// The operations for removing a certain amount of credits from an identity's balance. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity from whose balance credits are to be removed.
    /// * `balance_to_remove` - The amount of credits to be removed from the identity's balance.
    /// * `block_info` - Information about the current block.
    /// * `apply` - A boolean indicating whether the operation should be applied or not.
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The resulting fee result if successful, or an error.
    pub fn remove_from_identity_balance(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .remove_from_identity_balance
        {
            0 => self.remove_from_identity_balance_v0(
                identity_id,
                balance_to_remove,
                block_info,
                apply,
                transaction,
                platform_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_identity_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Removes a specified amount of credits from an identity balance. This function doesn't allow the balance to go below zero.
    /// Balances are stored under key 0 in the identity. Operations are determined based on the `apply` flag (stateful vs stateless).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity from which credits are to be removed.
    /// * `balance_to_remove` - The amount of credits to be removed from the identity's balance.
    /// * `estimated_costs_only_with_layer_info` - Estimated costs with layer information, if any.
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<LowLevelDriveOperation>, Error>` - The resulting low level drive operations if successful, or an error.
    pub(crate) fn remove_from_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        balance_to_remove: Credits,
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
            .remove_from_identity_balance
        {
            0 => self.remove_from_identity_balance_operations_v0(
                identity_id,
                balance_to_remove,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_from_identity_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
