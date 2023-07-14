mod v0;

use crate::drive::balances::balance_path;
use crate::drive::grove_operations::DirectQueryType;
use crate::drive::grove_operations::QueryTarget::QueryTargetValue;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::{DriveError, Error};
use crate::fee::calculate_fee;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::version::drive_versions::DriveVersion;
use grovedb::Element::SumItem;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's balance from the backing store, respecting drive versioning.
    /// The 'apply' argument indicates whether to get the estimated cost or the actual balance.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Credits>, Error>` - The balance of the Identity if successful, or an error.
    pub fn fetch_identity_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<Credits>, Error> {
        match drive_version.methods.identity.fetch.attributes.balance {
            0 => self.fetch_identity_balance_v0(identity_id, transaction, drive_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Identity's balance from the backing store, including the estimated cost.
    /// Respects drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `block_info` - The information about the current block.
    /// * `apply` - Whether to get the estimated cost or the actual balance.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<Credits>, FeeResult), Error>` - The balance of the Identity and the fee if successful, or an error.
    pub fn fetch_identity_balance_with_costs(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(Option<Credits>, FeeResult), Error> {
        match drive_version.methods.identity.fetch.attributes.balance {
            0 => self.fetch_identity_balance_with_costs_v0(
                identity_id,
                block_info,
                apply,
                transaction,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_balance_with_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates operations to get Identity's balance from the backing store, respecting drive versioning.
    /// Operations are created based on the 'apply' argument (stateful vs stateless).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `apply` - Whether to create stateful or stateless operations.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The drive operations to be updated.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Credits>, Error>` - The balance of the Identity if successful, or an error.
    pub(crate) fn fetch_identity_balance_operations(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Credits>, Error> {
        match drive_version.methods.identity.fetch.attributes.balance {
            0 => self.fetch_identity_balance_operations_v0(
                identity_id,
                apply,
                transaction,
                drive_operations,
                drive_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
