#![allow(clippy::result_large_err)] // Operation application returns drive::Error with rich causes
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Applies a batch of Drive operations to groveDB.
    pub(crate) fn apply_batch_low_level_drive_operations_v0(
        &self,
        estimated_costs_only_with_layer_info: Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        batch_operations: Vec<LowLevelDriveOperation>,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        let (grove_db_operations, ephemeral_grove_db_operations, mut other_operations) =
            LowLevelDriveOperation::grovedb_operations_batch_consume_split_ephemeral(
                batch_operations,
            );
        // The ephemeral (TTL'd-subtree) operations apply as their own batch
        // so their cost is known separately and can be consumed at the
        // ephemeral price — added bytes to processing instead of storage.
        // Cloning the layer info keeps the estimation path symmetric: the
        // dry run prices the ephemeral batch through the same worst-case
        // machinery, under the same pricing rule, so estimated stays an
        // upper bound of actual per fee class.
        let ephemeral_layer_info = if ephemeral_grove_db_operations.is_empty() {
            None
        } else {
            estimated_costs_only_with_layer_info.clone()
        };
        // Two batches must still commit as one. GroveDB opens and commits
        // an owned transaction per batch when none is supplied, which would
        // leave the standing batch committed if the ephemeral one failed —
        // a document row and its permanent index entries without their
        // TTL'd entries. Span both with one owned transaction instead and
        // commit only after both applied.
        let owned_transaction = (transaction.is_none()
            && estimated_costs_only_with_layer_info.is_none()
            && !grove_db_operations.is_empty()
            && !ephemeral_grove_db_operations.is_empty())
        .then(|| self.grove.start_transaction());
        let transaction = owned_transaction.as_ref().or(transaction);
        if !grove_db_operations.is_empty() {
            self.apply_batch_grovedb_operations(
                estimated_costs_only_with_layer_info,
                transaction,
                grove_db_operations,
                drive_operations,
                drive_version,
            )?;
        }
        if !ephemeral_grove_db_operations.is_empty() {
            let mut ephemeral_cost_operations: Vec<LowLevelDriveOperation> = vec![];
            self.apply_batch_grovedb_operations(
                ephemeral_layer_info,
                transaction,
                ephemeral_grove_db_operations,
                &mut ephemeral_cost_operations,
                drive_version,
            )?;
            drive_operations.extend(
                ephemeral_cost_operations
                    .into_iter()
                    .map(LowLevelDriveOperation::retag_ephemeral),
            );
        }
        drive_operations.append(&mut other_operations);
        if let Some(owned_transaction) = owned_transaction {
            self.commit_transaction(owned_transaction, drive_version)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::grove_operations::DirectQueryType;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;
    use grovedb_path::SubtreePath;

    /// A write that splits into a standing batch and an ephemeral batch
    /// must commit as one even when the caller supplies no transaction:
    /// if the ephemeral batch fails, nothing from the standing batch may
    /// survive. Without the owned transaction, grovedb would have
    /// committed the standing batch on its own before the failure.
    #[test]
    fn a_failing_ephemeral_batch_rolls_back_the_standing_batch_without_a_caller_transaction() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let parent_key = b"atomicity-parent".to_vec();
        drive
            .grove
            .insert(
                SubtreePath::empty(),
                parent_key.as_slice(),
                Element::empty_tree(),
                None,
                None,
                &platform_version.drive.grove_version,
            )
            .unwrap()
            .expect("the parent tree inserts");

        let standing_key = b"standing".to_vec();
        let standing = LowLevelDriveOperation::for_known_path_key_empty_tree(
            vec![parent_key.clone()],
            standing_key.clone(),
            None,
        );
        // Targets a subtree that does not exist, so the ephemeral batch
        // fails at apply time — after the standing batch already applied.
        let failing_ephemeral = LowLevelDriveOperation::insert_for_known_path_key_element(
            vec![parent_key.clone(), b"missing".to_vec()],
            b"x".to_vec(),
            Element::new_item(vec![1]),
        )
        .retag_ephemeral();

        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                vec![standing, failing_ephemeral],
                &mut vec![],
                &platform_version.drive,
            )
            .expect_err("the ephemeral batch targets a missing subtree");

        let mut scratch = vec![];
        let standing_survived = drive
            .grove_has_raw(
                SubtreePath::from([parent_key.as_slice()].as_slice()),
                standing_key.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut scratch,
                &platform_version.drive,
            )
            .expect("existence check");
        assert!(
            !standing_survived,
            "the standing batch must roll back with the failed ephemeral batch"
        );
    }
}
