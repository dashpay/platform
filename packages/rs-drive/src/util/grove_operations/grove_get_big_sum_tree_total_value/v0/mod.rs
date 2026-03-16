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
                let element = value.map_err(Error::from)?;
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

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::{DirectQueryType, QueryTarget};
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::TreeType;
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_get_big_sum_tree_total_value_stateful() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"big_sum",
                TreeType::BigSumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let value = drive
            .grove_get_big_sum_tree_total_value_v0(
                SubtreePath::empty(),
                b"big_sum",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected to get element");

        assert_eq!(value, 0);
    }

    #[test]
    fn test_grove_get_big_sum_tree_total_value_stateful_wrong_type() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"normal",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut ops = vec![];
        let result = drive.grove_get_big_sum_tree_total_value_v0(
            SubtreePath::empty(),
            b"normal",
            DirectQueryType::StatefulDirectQuery,
            Some(&tx),
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_grove_get_big_sum_tree_total_value_stateless() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let value = drive
            .grove_get_big_sum_tree_total_value_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTarget::QueryTargetTree(0, TreeType::BigSumTree),
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert_eq!(value, 0);
    }

    #[test]
    fn test_grove_get_big_sum_tree_total_value_stateless_non_tree_error() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive.grove_get_big_sum_tree_total_value_v0(
            [b"root".as_slice()].as_slice().into(),
            b"key",
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::NormalTree,
                query_target: QueryTarget::QueryTargetValue(100),
            },
            None,
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }
}
