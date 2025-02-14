mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the token statuses from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose statuses are to be proved.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.token.prove.token_statuses {
            0 => self.prove_token_statuses_v0(token_ids, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_statuses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves the token statuses with associated costs.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to prove the infos for.
    /// * `block_info` - Information about the current block for fee calculation.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses_with_costs(
        &self,
        token_ids: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<u8>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.prove_token_statuses_operations(
            token_ids,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;

        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;

        Ok((value, fees))
    }

    /// Creates the low-level operations needed to prove the Token statuses from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to query the statuses for.
    /// * `transaction` - The current transaction context.
    /// * `drive_operations` - A vector to store the created low-level drive operations.
    /// * `platform_version` - The platform version to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_token_statuses_operations(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version.drive.methods.token.prove.token_statuses {
            0 => self.prove_token_statuses_operations_v0(
                token_ids,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_statuses_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
