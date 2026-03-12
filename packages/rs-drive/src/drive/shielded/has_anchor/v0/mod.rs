use crate::drive::shielded::paths::shielded_credit_pool_anchors_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of checking whether a shielded anchor exists.
    ///
    /// Performs an O(1) key lookup in the anchors tree at
    /// `[AddressBalances, "s", [6]]`.
    pub(in crate::drive) fn has_shielded_anchor_v0(
        &self,
        anchor: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let anchors_path = shielded_credit_pool_anchors_path();

        self.grove_has_raw(
            (&anchors_path).into(),
            anchor,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}
