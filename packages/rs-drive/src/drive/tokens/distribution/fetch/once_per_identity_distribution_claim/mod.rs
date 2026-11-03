mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::{Identifier, TimestampMillis};
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the block time at which an identity claimed a token's once-per-identity
    /// distribution, or `None` if the identity has not claimed it.
    ///
    /// # Parameters
    ///
    /// * `token_id`: The token.
    /// * `identity_id`: The identity whose claim is read.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(TimestampMillis))` with the block time of the claim, `Ok(None)` when the
    ///   identity has not claimed it.
    /// * `Err(Error)` when the method version is unknown, the read fails, or the stored claim
    ///   is not an 8 byte item.
    pub fn fetch_once_per_identity_distribution_claim(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TimestampMillis>, Error> {
        self.fetch_once_per_identity_distribution_claim_operations(
            token_id,
            identity_id,
            &mut vec![],
            transaction,
            platform_version,
        )
    }

    /// Fetches the block time at which an identity claimed a token's once-per-identity
    /// distribution, accumulating the read operations for fee calculation.
    ///
    /// # Parameters
    ///
    /// * `token_id`: The token.
    /// * `identity_id`: The identity whose claim is read.
    /// * `drive_operations`: The operations accumulator the read is appended to, for billing.
    /// * `transaction`: The GroveDB transaction.
    /// * `platform_version`: The platform version.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(TimestampMillis))` with the block time of the claim, `Ok(None)` when the
    ///   identity has not claimed it.
    /// * `Err(Error)` when the method version is unknown, the read fails, or the stored claim
    ///   is not an 8 byte item.
    pub fn fetch_once_per_identity_distribution_claim_operations(
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
            .once_per_identity_distribution_claim
        {
            0 => self.fetch_once_per_identity_distribution_claim_operations_v0(
                token_id,
                identity_id,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_once_per_identity_distribution_claim".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
