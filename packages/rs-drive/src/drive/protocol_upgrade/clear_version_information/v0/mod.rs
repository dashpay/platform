use crate::drive::protocol_upgrade::{desired_version_for_validators_path, versions_counter_path};
use crate::drive::Drive;

use crate::error::Error;

use dpp::version::drive_versions::DriveVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Clear all version information from the backing store, this is done on epoch change in
    /// execution logic
    pub(super) fn clear_version_information_v0(
        &self,
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        self.grove_clear(
            (&versions_counter_path()).into(),
            transaction,
            drive_version,
        )?;
        self.grove_clear(
            (&desired_version_for_validators_path()).into(),
            transaction,
            drive_version,
        )?;

        Ok(())
    }
}
