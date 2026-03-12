use crate::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_NOTES_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of counting shielded pool notes.
    ///
    /// Returns the total number of items in the CommitmentTree at
    /// `[AddressBalances, "s", [1]]`.
    pub(in crate::drive) fn shielded_pool_notes_count_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let pool_path = shielded_credit_pool_path();
        self.grove_commitment_tree_count(
            (&pool_path).into(),
            &[SHIELDED_NOTES_KEY],
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
