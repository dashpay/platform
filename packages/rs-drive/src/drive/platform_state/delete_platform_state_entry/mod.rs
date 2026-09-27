mod v0;

use crate::drive::platform_state::PlatformStateEntryKind;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Delete one member of a per-entry platform state collection. Deleting a
    /// member that is not stored is not an error.
    pub fn delete_platform_state_entry(
        &self,
        kind: PlatformStateEntryKind,
        key: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .delete_platform_state_entry
        {
            0 => self.delete_platform_state_entry_v0(kind, key, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "delete_platform_state_entry".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
