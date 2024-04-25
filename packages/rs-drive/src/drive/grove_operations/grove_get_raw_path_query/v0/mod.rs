use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::query_result_type::{QueryResultElements, QueryResultType};
use grovedb::{PathQuery, TransactionArg};
use grovedb_costs::CostContext;

impl Drive {
    /// Gets the return value and the cost of a groveDB raw path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(super) fn grove_get_raw_path_query_v0(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        result_type: QueryResultType,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<(QueryResultElements, u16), Error> {
        let CostContext { value, cost } = self.grove.query_raw(
            path_query,
            transaction.is_some(),
            true,
            true,
            result_type,
            transaction,
        );
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::GroveDB)
    }
}
