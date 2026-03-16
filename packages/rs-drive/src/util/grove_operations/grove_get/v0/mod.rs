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
    pub(super) fn grove_get_v0<B: AsRef<[u8]>>(
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
                in_tree_type: in_tree_using_sums,
                query_target,
                estimated_reference_sizes,
            } => {
                let key_info_path = KeyInfoPath::from_known_owned_path(path.to_vec());
                let key_info = KeyInfo::KnownKey(key.to_vec());
                let cost = match query_target {
                    QueryTarget::QueryTargetTree(flags_size, tree_type) => {
                        GroveDb::average_case_for_get_tree(
                            &key_info_path,
                            &key_info,
                            flags_size,
                            tree_type,
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
                Ok(Some(value.map_err(Error::from)?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::{QueryTarget, QueryType};
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_get_stateful_existing_item() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree(
                SubtreePath::empty(),
                b"root",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .unwrap();

        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"key",
                Element::new_item(b"value".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .unwrap();

        let mut ops = vec![];
        let result = drive
            .grove_get_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                QueryType::StatefulQuery,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert_eq!(result, Some(Element::new_item(b"value".to_vec())));
        assert!(!ops.is_empty()); // cost op pushed
    }

    #[test]
    fn test_grove_get_stateless_returns_none() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive
            .grove_get_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                QueryType::StatelessQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTarget::QueryTargetValue(100),
                    estimated_reference_sizes: vec![],
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
        assert!(!ops.is_empty()); // cost op pushed
    }

    #[test]
    fn test_grove_get_stateless_tree_target() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive
            .grove_get_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                QueryType::StatelessQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTarget::QueryTargetTree(0, TreeType::NormalTree),
                    estimated_reference_sizes: vec![],
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_none());
    }
}
