use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use crate::util::grove_operations::{DirectQueryType, QueryTarget};
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, GroveDb, TransactionArg};
use grovedb_costs::CostContext;
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the element at the given path from groveDB.
    /// Pushes the `OperationCost` of getting the element to `drive_operations`.
    pub(super) fn grove_get_big_sum_tree_total_value_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<i128, Error> {
        match query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type,
                query_target,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, tree_type) => {
                        Ok(GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            tree_type,
                            in_tree_type,
                            &drive_version.grove_version,
                        )?)
                    }
                    _ => Err(Error::Drive(DriveError::CorruptedCodeExecution(
                        "can not query a non tree in grove_get_big_sum_tree_total_value",
                    ))),
                }?;

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(0)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } =
                    self.grove
                        .get_raw(path, key, transaction, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                let element = value.map_err(Error::GroveDB)?;
                match element {
                    Element::BigSumTree(_, value, _) => Ok(value),
                    _ => Err(Error::Drive(DriveError::CorruptedBalancePath(
                        "balance path does not refer to a big sum tree",
                    ))),
                }
            }
        }
    }
}
