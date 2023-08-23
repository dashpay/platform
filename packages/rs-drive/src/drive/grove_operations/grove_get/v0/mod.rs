use crate::drive::grove_operations::{QueryTarget, QueryType};
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use crate::fee::op::LowLevelDriveOperation::CalculatedCostOperation;
use costs::CostContext;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, GroveDb, TransactionArg};
use path::SubtreePath;

impl Drive {
    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub(super) fn grove_get_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: QueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
    ) -> Result<Option<Element>, Error> {
        match query_type {
            QueryType::StatelessQuery {
                in_tree_using_sums,
                query_target,
                estimated_reference_sizes,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, is_sum_tree) => {
                        GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            is_sum_tree,
                            in_tree_using_sums,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_get(
                            &key_info_path,
                            &key_info,
                            in_tree_using_sums,
                            estimated_value_size,
                            estimated_reference_sizes,
                        )
                    }
                };

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            QueryType::StatefulQuery => {
                let CostContext { value, cost } = self.grove.get(path, key, transaction);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
        }
    }
}
