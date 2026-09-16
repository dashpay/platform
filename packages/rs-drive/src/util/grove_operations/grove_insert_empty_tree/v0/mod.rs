use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::push_drive_operation_result;
use grovedb::operations::insert::InsertOptions;
use grovedb::{Element, TransactionArg, TreeType};
use grovedb_path::SubtreePath;
use platform_version::version::drive_versions::DriveVersion;

impl Drive {
    /// Pushes the `OperationCost` of inserting an empty tree in groveDB to `drive_operations`.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn grove_insert_empty_tree_v0<B: AsRef<[u8]>>(
        &self,
        path: SubtreePath<'_, B>,
        key: &[u8],
        tree_type: TreeType,
        transaction: TransactionArg,
        options: Option<InsertOptions>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let tree_element = match tree_type {
            TreeType::NormalTree => Element::empty_tree(),
            TreeType::SumTree => Element::empty_sum_tree(),
            TreeType::BigSumTree => Element::empty_big_sum_tree(),
            TreeType::CountTree => Element::empty_count_tree(),
            TreeType::CountSumTree => Element::empty_count_sum_tree(),
            TreeType::ProvableCountTree => Element::empty_provable_count_tree(),
            TreeType::ProvableCountSumTree => Element::empty_provable_count_sum_tree(),
            TreeType::ProvableCountProvableSumTree => {
                Element::empty_provable_count_provable_sum_tree()
            }
            TreeType::ProvableSumTree => Element::empty_provable_sum_tree(),
            TreeType::CommitmentTree(chunk_power) => Element::empty_commitment_tree(chunk_power)?,
            TreeType::MmrTree => Element::empty_mmr_tree(),
            TreeType::BulkAppendTree(chunk_power) => Element::empty_bulk_append_tree(chunk_power)?,
            TreeType::DenseAppendOnlyFixedSizeTree(chunk_power) => {
                Element::empty_dense_tree(chunk_power)
            }
            // Single-axis indexed trees carry no axis list on the element, so
            // `TreeType` fully describes them. See the matching arms in
            // `fees::op`'s `empty_tree_operation_for_known_path_key`.
            TreeType::ProvableSumIndexedTree => Element::empty_provable_sum_indexed_tree(),
            TreeType::ProvableCountIndexedTree => Element::empty_provable_count_indexed_tree(),
            // The multi-axis variant's axes TLV is not carried by `TreeType`,
            // so this generic path cannot describe the `Element` to insert.
            // Ranked index creation uses the axis-taking helpers instead
            // (`batch_insert_empty_provable_count_provable_sum_indexed_tree`).
            TreeType::ProvableCountProvableSumIndexedTree => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "grove_insert_empty_tree cannot create a \
                     ProvableCountProvableSumIndexedTree — the ranked axis set is not carried \
                     by TreeType; use \
                     batch_insert_empty_provable_count_provable_sum_indexed_tree instead.",
                )))
            }
            // The private document store's entry size is not carried by
            // `TreeType` either (see `fees::op`); no generic creation path.
            TreeType::PrivateDocumentStore(_) => {
                return Err(Error::Drive(DriveError::NotSupported(
                    "grove_insert_empty_tree cannot create a PrivateDocumentStore — the entry \
                     size is not carried by TreeType",
                )))
            }
        };
        let cost_context = self.grove.insert(
            path,
            key,
            tree_element,
            options,
            transaction,
            &drive_version.grove_version,
        );
        push_drive_operation_result(cost_context, drive_operations)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::test_helpers::setup::setup_drive;
    use grovedb::{Element, TreeType};
    use grovedb_path::SubtreePath;
    use platform_version::version::PlatformVersion;

    #[test]
    fn test_grove_insert_empty_tree_normal() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree_v0(
                SubtreePath::empty(),
                b"normal",
                TreeType::NormalTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        // Verify the tree exists by querying for it
        let mut query_ops = vec![];
        let element = drive
            .grove_get_raw(
                SubtreePath::empty(),
                b"normal",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut query_ops,
                &pv.drive,
            )
            .expect("expected to get element");
        assert!(
            matches!(element, Some(Element::Tree(..))),
            "Expected a tree element after insert"
        );
    }

    #[test]
    fn test_grove_insert_empty_tree_sum() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree_v0(
                SubtreePath::empty(),
                b"sum",
                TreeType::SumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut query_ops = vec![];
        let element = drive
            .grove_get_raw(
                SubtreePath::empty(),
                b"sum",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut query_ops,
                &pv.drive,
            )
            .expect("expected to get element");
        assert!(
            matches!(element, Some(Element::SumTree(..))),
            "Expected a sum tree element after insert"
        );
    }

    #[test]
    fn test_grove_insert_empty_tree_big_sum() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree_v0(
                SubtreePath::empty(),
                b"big_sum",
                TreeType::BigSumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut query_ops = vec![];
        let element = drive
            .grove_get_raw(
                SubtreePath::empty(),
                b"big_sum",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut query_ops,
                &pv.drive,
            )
            .expect("expected to get element");
        assert!(
            matches!(element, Some(Element::BigSumTree(..))),
            "Expected a big sum tree element after insert"
        );
    }

    #[test]
    fn test_grove_insert_empty_tree_count() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree_v0(
                SubtreePath::empty(),
                b"count",
                TreeType::CountTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut query_ops = vec![];
        let element = drive
            .grove_get_raw(
                SubtreePath::empty(),
                b"count",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut query_ops,
                &pv.drive,
            )
            .expect("expected to get element");
        assert!(
            matches!(element, Some(Element::CountTree(..))),
            "Expected a count tree element after insert"
        );
    }

    #[test]
    fn test_grove_insert_empty_tree_count_sum() {
        let drive = setup_drive(None);
        let pv = PlatformVersion::latest();
        let tx = drive.grove.start_transaction();

        drive
            .grove_insert_empty_tree_v0(
                SubtreePath::empty(),
                b"count_sum",
                TreeType::CountSumTree,
                Some(&tx),
                None,
                &mut vec![],
                &pv.drive,
            )
            .expect("expected to insert root tree");

        let mut query_ops = vec![];
        let element = drive
            .grove_get_raw(
                SubtreePath::empty(),
                b"count_sum",
                DirectQueryType::StatefulDirectQuery,
                Some(&tx),
                &mut query_ops,
                &pv.drive,
            )
            .expect("expected to get element");
        assert!(
            matches!(element, Some(Element::CountSumTree(..))),
            "Expected a count sum tree element after insert"
        );
    }
}
