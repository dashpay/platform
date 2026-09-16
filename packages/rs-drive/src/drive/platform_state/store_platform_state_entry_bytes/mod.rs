mod v0;

use crate::drive::platform_state::PlatformStateEntryKind;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Store one member of a per-entry platform state collection: `bytes` under
    /// the collection's key prefix followed by `key`, replacing any earlier value.
    pub fn store_platform_state_entry_bytes(
        &self,
        kind: PlatformStateEntryKind,
        key: &[u8],
        bytes: &[u8],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .store_platform_state_entry_bytes
        {
            0 => self.store_platform_state_entry_bytes_v0(kind, key, bytes, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "store_platform_state_entry_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
