use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::{QueryTarget, QueryType};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, GroveDb, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub(crate) fn grove_get_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: QueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
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
                            &drive_version.grove_version,
                        )
                    }
                    QueryTarget::QueryTargetValue(estimated_value_size) => {
                        GroveDb::average_case_for_get(
                            &key_info_path,
                            &key_info,
                            in_tree_using_sums,
                            estimated_value_size,
                            estimated_reference_sizes,
                            &drive_version.grove_version,
                        )
                    }
                }?;

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            QueryType::StatefulQuery => {
                let CostContext { value, cost } =
                    self.grove
                        .get(path, key, transaction, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
        }
    }
}
