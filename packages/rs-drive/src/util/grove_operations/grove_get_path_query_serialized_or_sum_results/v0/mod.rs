use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::operations::QueryItemOrSumReturnType;
use grovedb::{PathQuery, TransactionArg};
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(crate) fn grove_get_path_query_serialized_or_sum_results_v0(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(Vec<QueryItemOrSumReturnType>, u16), Error> {
        let CostContext { value, cost } = self.grove.query_item_value_or_sum(
            path_query,
            transaction.is_some(),
            true,
            true,
            transaction,
            &drive_version.grove_version,
        );
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }
}
