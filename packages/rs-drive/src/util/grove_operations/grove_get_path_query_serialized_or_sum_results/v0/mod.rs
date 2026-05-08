use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::operations::QueryItemOrSumReturnType;
use grovedb::{PathQuery, TransactionArg};
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB path query.
    /// Pushes the cost to `drive_operations` and returns the return value.
    pub(super) fn grove_get_path_query_serialized_or_sum_results_v0(
        &self,
        path_query: &PathQuery,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(Vec<QueryItemOrSumReturnType>, u16), Error> {
        let CostContext { value, cost } = self.grove.query_item_value_or_sum(
            path_query,
            transaction.is_some(),
            true,
            true,
            transaction,
            &drive_version.grove_version,
        );
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::operations::QueryItemOrSumReturnType;
    use grovedb::{Element, PathQuery, Query, SizedQuery, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_get_path_query_serialized_or_sum_results_empty() {
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
            .expect("expected to insert root tree");

        let path_query = PathQuery::new(
            vec![b"root".to_vec()],
            SizedQuery::new(Query::new(), None, None),
        );

        let mut ops = vec![];
        let (results, count) = drive
            .grove_get_path_query_serialized_or_sum_results_v0(
                &path_query,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert!(results.is_empty());
        assert_eq!(count, 0);
    }

    #[test]
    fn test_grove_get_path_query_serialized_or_sum_results_with_items() {
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
            .expect("expected to insert root tree");

        drive
            .grove
            .insert(
                &[b"root".as_slice()],
                b"a",
                Element::new_item(b"val_a".to_vec()),
                None,
                Some(&tx),
                &pv.drive.grove_version,
            )
            .unwrap()
            .expect("expected to insert element");

        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(vec![b"root".to_vec()], SizedQuery::new(query, None, None));

        let mut ops = vec![];
        let (results, _count) = drive
            .grove_get_path_query_serialized_or_sum_results_v0(
                &path_query,
                Some(&tx),
                &mut ops,
                &pv.drive,
            )
            .expect("expected operation to succeed");

        assert_eq!(results.len(), 1);
        // Verify the result contains the expected serialized value
        match &results[0] {
            QueryItemOrSumReturnType::ItemData(data) => {
                assert_eq!(data, &b"val_a".to_vec());
            }
            other => panic!("expected ItemData, got {:?}", other),
        }
    }
}
