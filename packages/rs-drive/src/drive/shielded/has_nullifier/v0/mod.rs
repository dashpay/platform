use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of checking whether a nullifier exists.
    ///
    /// Performs an O(1) key lookup in the nullifiers tree at
    /// `[AddressBalances, "s", [2]]`.
    pub(in crate::drive) fn has_nullifier_v0(
        &self,
        nullifier: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let nullifiers_path = shielded_credit_pool_nullifiers_path();

        self.grove_has_raw(
            (&nullifiers_path).into(),
            nullifier,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
