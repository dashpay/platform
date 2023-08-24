use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves an identity with all its information from an identity id.
    pub(super) fn prove_identity_keys_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let query = Self::full_identity_query(&identity_id)?;
        self.grove_get_proved_path_query(
            &query,
            false,
            transaction,
            &mut drive_operations,
            drive_version,
        )
    }
}