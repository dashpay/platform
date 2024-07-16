mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Fetches a prefunded specialized balance from the backing store, respecting drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Credits>, Error>` - The balance of the Identity if successful, or an error.
    pub fn fetch_prefunded_specialized_balance(
        &self,
        prefunded_specialized_balance_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .fetch_single
        {
            0 => self.fetch_prefunded_specialized_balance_v0(
                prefunded_specialized_balance_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_prefunded_specialized_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches a prefunded specialized balance from the backing store, including the estimated costs
    /// of the operation.
    /// Respects drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `block_info` - The information about the current block.
    /// * `apply` - Whether to actually run the query or just get the estimated costs that the query
    ///     would use.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<Credits>, FeeResult), Error>` - The balance of the Identity and the fee if successful, or an error.
    pub fn fetch_prefunded_specialized_balance_with_costs(
        &self,
        prefunded_specialized_balance_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Credits>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .fetch_single
        {
            0 => self.fetch_prefunded_specialized_balance_with_costs_v0(
                prefunded_specialized_balance_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_prefunded_specialized_balance_with_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Creates operations to get a prefunded specialized balance from the backing store.
    /// Operations are created based on the 'apply' argument (stateful vs stateless).
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `apply` - Whether to create stateful or stateless operations.
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The drive operations to be updated.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Credits>, Error>` - The balance of the Identity if successful, or an error.
    #[allow(unused)]
    pub(crate) fn fetch_prefunded_specialized_balance_operations(
        &self,
        prefunded_specialized_balance_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Credits>, Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .fetch_single
        {
            0 => self.fetch_prefunded_specialized_balance_operations_v0(
                prefunded_specialized_balance_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_prefunded_specialized_balance_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
