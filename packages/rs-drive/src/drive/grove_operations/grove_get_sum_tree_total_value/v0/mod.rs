use crate::drive::grove_operations::{DirectQueryType, QueryTarget};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, GroveDb, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_path::SubtreePath;

impl Drive {
    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub(super) fn grove_get_sum_tree_total_value_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<i64, Error> {
        match query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, is_sum_tree) => {
                        Ok(GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            is_sum_tree,
                            in_tree_using_sums,
                        ))
                    }
                    _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "can not query a non tree",
                    ))),
                }?;

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(0)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } = self.grove.get_raw(path, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                let element = value.map_err(Error::GroveDB)?;
                match element {
                    Element::SumTree(_, value, _) => Ok(value),
                    _ => Err(Error::Drive(DriveError::CorruptedBalancePath(
                        "balance path does not refer to a sum tree",
                    ))),
                }
            }
        }
    }
}
