use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{PathQuery, TransactionArg};
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(super) fn grove_get_path_query_with_optional_v0(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        let CostContext { value, cost } = self.grove.query_keys_optional(
            path_query,
            true,
            true,
            false,
            transaction,
            &drive_version.grove_version,
        );
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }
}
