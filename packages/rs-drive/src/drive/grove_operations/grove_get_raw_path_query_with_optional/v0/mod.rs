use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::query_result_type::PathKeyOptionalElementTrio;
use grovedb::{PathQuery, TransactionArg};
use grovedb_costs::CostContext;

impl Drive {
    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(super) fn grove_get_raw_path_query_with_optional_v0(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Vec<PathKeyOptionalElementTrio>, Error> {
        let CostContext { value, cost } =
            self.grove
                .query_raw_keys_optional(path_query, true, true, true, transaction);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }
}
