mod v0;
mod v1;

use crate::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcome;
use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::{BalanceChangeForIdentity, FeeResult};

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// Applies balance changes to an identity.
    ///
    /// Depending on the version specified in the `drive_version` parameter, this method
    /// will route the request to the correct versioned implementation.
    ///
    /// # Arguments
    ///
    /// * `balance_change` - The balance changes to be applied to an identity.
    /// * `block_info` - The block the balance change is applied in. From generation 1
    ///   (protocol version 14), credits that repay an identity's debt reach the processing fee
    ///   pool of its epoch.
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version configuration, which determines the version of
    ///   the method to be used.
    ///
    /// # Returns
    ///
    /// On success, it will return the `ApplyBalanceChangeOutcome` structure containing information
    /// about the balance change application operation. On error, it will return a relevant error.
    pub fn apply_balance_change_from_fee_to_identity(
        &self,
        balance_change: BalanceChangeForIdentity,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ApplyBalanceChangeOutcome, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .apply_balance_change_from_fee_to_identity
        {
            0 => self.apply_balance_change_from_fee_to_identity_v0(
                balance_change,
                transaction,
                platform_version,
            ),
            1 => self.apply_balance_change_from_fee_to_identity_v1(
                balance_change,
                block_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_balance_change_from_fee_to_identity".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Prepares operations to apply balance changes to an identity.
    ///
    /// Depending on the version specified in the `drive_version` parameter, this method
    /// will route the request to the correct versioned implementation.
    ///
    /// # Arguments
    ///
    /// * `balance_change` - The balance changes to be applied to an identity.
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version configuration, which determines the version of
    ///   the method to be used.
    ///
    /// # Returns
    ///
    /// On success, it will return a vector of low level drive operations and the fee result.
    /// From generation 1 (protocol version 14) the operations carry a
    /// [`LowLevelDriveOperation::RepaidIdentityDebt`] for every debt the balance change
    /// repaid, which the caller owes the current epoch's processing fee pool.
    /// On error, it will return a relevant error.
    pub fn apply_balance_change_from_fee_to_identity_operations(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .apply_balance_change_from_fee_to_identity
        {
            0 => self.apply_balance_change_from_fee_to_identity_operations_v0(
                balance_change,
                transaction,
                platform_version,
            ),
            1 => self.apply_balance_change_from_fee_to_identity_operations_v1(
                balance_change,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_balance_change_from_fee_to_identity_operations".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
