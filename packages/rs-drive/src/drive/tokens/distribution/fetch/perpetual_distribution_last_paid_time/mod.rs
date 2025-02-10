mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::{Identifier, TimestampMillis};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the last paid timestamp for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method queries the perpetual distributions tree at the path
    /// `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing the last paid `TimestampMillis` on success or an `Error` on failure.
    pub fn fetch_perpetual_distribution_last_paid_time(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TimestampMillis>, Error> {
        self.fetch_perpetual_distribution_last_paid_time_operations(
            token_id,
            identity_id,
            &mut vec![],
            transaction,
            platform_version,
        )
    }

    /// Fetches the last paid timestamp for a perpetual distribution for a given identity,
    /// using the appropriate versioned method.
    ///
    /// This method queries the perpetual distributions tree at the path
    /// `perpetual_distribution_last_paid_time_path_vec(token_id, identity_id)`.
    ///
    /// # Parameters
    ///
    /// - `token_id`: The 32‑byte identifier for the token.
    /// - `identity_id`: The identifier of the identity whose last paid time is being queried.
    /// - `drive_operations`: A mutable vector to accumulate low-level drive operations.
    /// - `transaction`: The current GroveDB transaction.
    /// - `platform_version`: The platform version to determine the method variant.
    ///
    /// # Returns
    ///
    /// A `Result` containing the last paid `TimestampMillis` on success or an `Error` on failure.
    pub(crate) fn fetch_perpetual_distribution_last_paid_time_operations(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TimestampMillis>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .perpetual_distribution_last_paid_time
        {
            0 => self.fetch_perpetual_distribution_last_paid_time_operations_v0(
                token_id,
                identity_id,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_perpetual_distribution_last_paid_time_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}