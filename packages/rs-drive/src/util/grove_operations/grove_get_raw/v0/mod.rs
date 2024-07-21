use crate::drive::Drive;
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
    /// grove_get_raw basically means that there are no reference hops, this only matters
    /// when calculating worst case costs
    pub(crate) fn grove_get_raw_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Option<Element>, Error> {
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_using_sums,
                query_target,
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
                        GroveDb::average_case_for_get_raw(
                            &key_info_path,
                            &key_info,
                            estimated_value_size,
                            in_tree_using_sums,
                            &drive_version.grove_version,
                        )
                    }
                }?;

                drive_operations.push(CalculatedCostOperation(cost));
                Ok(None)
            }
            DirectQueryType::StatefulDirectQuery => {
                let CostContext { value, cost } =
                    self.grove
                        .get_raw(path, key, transaction, &drive_version.grove_version);
                drive_operations.push(CalculatedCostOperation(cost));
                Ok(Some(value.map_err(Error::GroveDB)?))
            }
        }
    }
}
