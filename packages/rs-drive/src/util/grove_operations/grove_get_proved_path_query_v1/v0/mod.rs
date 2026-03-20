use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
use grovedb::PathQuery;
use grovedb_costs::CostContext;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Gets the return value and the cost of a groveDB V1 proved path query.
    /// V1 proofs support BulkAppendTree and CommitmentTree subqueries.
    /// Pushes the cost to `drive_operations` and returns the proof bytes.
    pub(super) fn grove_get_proved_path_query_v1_v0(
        &self,
        path_query: &PathQuery,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<Vec<u8>, Error> {
        let CostContext { value, cost } =
            self.grove
                .prove_query(path_query, None, &drive_version.grove_version);
        drive_operations.push(CalculatedCostOperation(cost));
        value.map_err(Error::from)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{PathQuery, Query, SizedQuery, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_get_proved_path_query_v1_basic() {
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
            .commit_transaction(tx)
            .unwrap()
            .expect("expected to commit transaction");

        let path_query = PathQuery::new(
            vec![b"root".to_vec()],
            SizedQuery::new(Query::new(), None, None),
        );

        let mut ops = vec![];
        let proof = drive
            .grove_get_proved_path_query_v1_v0(&path_query, &mut ops, &pv.drive)
            .expect("expected operation to succeed");

        assert!(!proof.is_empty());
        assert!(!ops.is_empty());
    }
}
