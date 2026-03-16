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
use itertools::Itertools;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// grove_get_raw_item basically means that there are no reference hops, this only matters
    /// when calculating worst case costs
    pub(super) fn grove_get_raw_item_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        direct_query_type: DirectQueryType,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        match direct_query_type {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: in_tree_using_sums,
                query_target,
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
                Ok(vec![])
            }
            DirectQueryType::StatefulDirectQuery => {
                //todo remove path clone
                let CostContext { value, cost } = self.grove.get_raw(
                    path.clone(),
                    key,
                    transaction,
                    &drive_version.grove_version,
                );
                drive_operations.push(CalculatedCostOperation(cost));
                let element = value.map_err(Error::from)?;
                match element {
                    Element::Item(value, _) => Ok(value),
                    _ => Err(Error::Drive(DriveError::CorruptedDriveState(format!(
                        "path {}/0x{} does not refer to an item",
                        path.to_vec()
                            .iter()
                            .map(|bytes| format!("0x{}", hex::encode(bytes)))
                            .join("/"),
                        hex::encode(key)
                    )))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::{DirectQueryType, QueryTarget};
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_get_raw_item_stateful_success() {
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
            .grove_get_raw_item_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert_eq!(result, b"value".to_vec());
    }

    #[test]
    fn test_grove_get_raw_item_stateful_not_item_error() {
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

        // Insert a tree, not an item
        drive
            .grove_insert_empty_tree(
                [b"root".as_slice()].as_slice().into(),
                b"child",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .unwrap();

        let mut ops = vec![];
        let result = drive.grove_get_raw_item_v0(
            [b"root".as_slice()].as_slice().into(),
            b"child",
            DirectQueryType::StatefulDirectQuery,
            Some(&tx),
            &mut ops,
            &pv.drive,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_grove_get_raw_item_stateless_returns_empty() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();

        let mut ops = vec![];
        let result = drive
            .grove_get_raw_item_v0(
                [b"root".as_slice()].as_slice().into(),
                b"key",
                DirectQueryType::StatelessDirectQuery {
                    in_tree_type: TreeType::NormalTree,
                    query_target: QueryTarget::QueryTargetValue(100),
                },
                None,
                &mut ops,
                &pv.drive,
            )
            .unwrap();

        assert!(result.is_empty());
    }
}
