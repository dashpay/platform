mod v0;

use crate::drive::platform_state::{PlatformStateEntry, PlatformStateEntryKind};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Every stored member of a per-entry platform state collection, in key
    /// order, as `(key, bytes)` pairs with the collection's prefix removed from
    /// the key.
    pub fn fetch_platform_state_entries_bytes(
        &self,
        kind: PlatformStateEntryKind,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<PlatformStateEntry>, Error> {
        match platform_version
            .drive
            .methods
            .platform_state
            .fetch_platform_state_entries_bytes
        {
            0 => self.fetch_platform_state_entries_bytes_v0(kind, transaction),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_platform_state_entries_bytes".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
