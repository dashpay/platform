mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id. This function is version controlled.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity to which balance is to be added.
    /// * `added_balance` - The balance to be added.
    /// * `block_info` - The block information.
    /// * `apply` - Whether to apply the operations.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version.
    ///
    /// # Returns
    ///
    /// * `Result<FeeResult, Error>` - The fee result if successful, or an error.
    pub fn add_to_identity_balance(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
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
            .add_to_identity_balance
        {
            0 => self.add_to_identity_balance_v0(
                identity_id,
                added_balance,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Balances are stored in the balance tree under the identity's id
    /// This gets operations based on apply flag (stateful vs stateless)
    pub(crate) fn add_to_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        added_balance: Credits,
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
            .add_to_identity_balance
        {
            0 => self.add_to_identity_balance_operations_v0(
                identity_id,
                added_balance,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_to_identity_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
