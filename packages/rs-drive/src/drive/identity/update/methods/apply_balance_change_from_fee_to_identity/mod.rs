mod v0;

use crate::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcome;
use crate::drive::Drive;
use crate::error::drive::DriveError;

use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
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
    /// * `transaction` - The transaction information related to the operation.
    /// * `drive_version` - The drive version configuration, which determines the version of
    ///                      the method to be used.
    ///
    /// # Returns
    ///
    /// On success, it will return the `ApplyBalanceChangeOutcome` structure containing information
    /// about the balance change application operation. On error, it will return a relevant error.
    pub fn apply_balance_change_from_fee_to_identity(
        &self,
        balance_change: BalanceChangeForIdentity,
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_balance_change_from_fee_to_identity".to_string(),
                known_versions: vec![0],
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
    ///                      the method to be used.
    ///
    /// # Returns
    ///
    /// On success, it will return a vector of low level drive operations and the fee result.
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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "apply_balance_change_from_fee_to_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
