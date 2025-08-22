mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Returns a GroveDB **proof** of the last claim for a perpetual distribution.
    ///
    /// Convenience wrapper that allocates an internal `Vec<LowLevelDriveOperation>`
    /// so callers don’t have to manage it.
    pub fn prove_perpetual_distribution_last_paid_moment(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.prove_perpetual_distribution_last_paid_moment_operations(
            token_id,
            identity_id,
            &mut vec![],
            transaction,
            platform_version,
        )
    }

    /// Version‑switching entry point used by other Drive internals.
    pub(crate) fn prove_perpetual_distribution_last_paid_moment_operations(
        &self,
        token_id: [u8; 32],
        identity_id: Identifier,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .prove
            .perpetual_distribution_last_paid_time
        {
            0 => self.prove_perpetual_distribution_last_paid_moment_operations_v0(
                token_id,
                identity_id,
                drive_operations,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_perpetual_distribution_last_paid_moment_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
