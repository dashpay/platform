use crate::drive::grove_operations::{DirectQueryType, QueryTarget};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::query::GroveError;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{GroveDb, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_path::SubtreePath;

impl Drive {
    /// Gets the return value and the cost of a groveDB `has_raw` operation.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(super) fn grove_has_raw_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<bool, Error> {
        let CostContext { value, cost } = match query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_len, is_sum_tree) => {
                        GroveDb::average_case_for_has_raw_tree(
                            &key_info_path,
                            &key_info,
                            flags_len,
                            is_sum_tree,
                            in_tree_using_sums,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_has_raw(
                            &key_info_path,
                            &key_info,
                            estimated_value_size,
                            in_tree_using_sums,
                        )
                    }
                };

                CostContext {
                    value: Ok(false),
                    cost,
                }
            }
            DirectQueryType::StatefulDirectQuery => {
                if self.config.has_raw_enabled {
                    self.grove.has_raw(path, key, transaction)
                } else {
                    self.grove.get_raw(path, key, transaction).map(|r| match r {
                        Err(GroveError::PathKeyNotFound(_))
                        | Err(GroveError::PathNotFound(_))
                        | Err(GroveError::PathParentLayerNotFound(_)) => Ok(false),
                        Err(e) => Err(e),
                        Ok(_) => Ok(true),
                    })
                }
            }
        };
        drive_operations.push(CalculatedCostOperation(cost));
        Ok(value?)
    }
}
