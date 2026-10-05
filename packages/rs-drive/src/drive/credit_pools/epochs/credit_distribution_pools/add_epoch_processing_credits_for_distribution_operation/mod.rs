mod v0;

use grovedb::TransactionArg;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;

impl Drive {
    /// Adds to the amount of processing fees to be distributed for the Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_tree` - A reference to the Epoch.
    /// * `amount` - The amount to add.
    /// * `transaction` - A TransactionArg instance.
    /// * `platform_version` - A PlatformVersion instance representing the version of the drive.
    ///
    /// # Returns
    ///
    /// A Result containing either the processing fee for the epoch, if found,
    /// or an Error if something goes wrong.
    pub fn add_epoch_processing_credits_for_distribution_operation(
        &self,
        epoch: &Epoch,
        amount: Credits,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        match platform_version
            .drive
            .methods
            .credit_pools
            .epochs
            .add_epoch_processing_credits_for_distribution_operation
        {
            0 => self.add_epoch_processing_credits_for_distribution_operation_v0(
                epoch,
                amount,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_epoch_processing_credits_for_distribution_operation".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Adds `repaid_debt`, the credits identities' incoming balances repaid of their debts
    /// ([`LowLevelDriveOperation::RepaidIdentityDebt`]), to the processing fees of `epoch`, in
    /// a batch of its own that is not billed. It reads the pool through `transaction`, so called
    /// after the batch that repaid the debts it adds to any pool write that batch made. Nothing
    /// is written when `repaid_debt` is zero.
    pub(crate) fn apply_repaid_identity_debt_to_processing_pool(
        &self,
        repaid_debt: Credits,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if repaid_debt == 0 {
            return Ok(());
        }
        let pool_operation = self.add_epoch_processing_credits_for_distribution_operation(
            epoch,
            repaid_debt,
            transaction,
            platform_version,
        )?;
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            vec![pool_operation],
            &mut vec![],
            &platform_version.drive,
        )
    }
}
