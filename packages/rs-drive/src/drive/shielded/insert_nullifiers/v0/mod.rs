use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::Element;

impl Drive {
    /// Version 0 implementation of inserting nullifiers.
    ///
    /// For each nullifier, inserts into the permanent nullifiers tree to
    /// prevent double-spend.
    pub(in crate::drive) fn insert_nullifiers_v0(
        &self,
        nullifiers: &[[u8; 32]],
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let nullifiers_path = shielded_credit_pool_nullifiers_path_vec();

        // Build permanent nullifier tree ops
        let ops = nullifiers
            .iter()
            .map(|nullifier| {
                GroveOperation(
                    QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                        nullifiers_path.clone(),
                        nullifier.to_vec(),
                        Element::new_item(vec![]),
                    ),
                )
            })
            .collect();

        Ok(ops)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use platform_version::version::PlatformVersion;

    #[test]
    fn insert_empty_nullifier_slice_produces_empty_ops_vec() {
        // Empty input edge case: no ops should be produced.
        let drive = setup_drive_with_initial_state_structure(None);

        let ops = drive
            .insert_nullifiers_v0(&[])
            .expect("empty nullifiers should be allowed");
        assert!(ops.is_empty());
    }

    #[test]
    fn insert_single_nullifier_produces_single_op_and_applies() {
        // Single nullifier path.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let nullifier = [0x11u8; 32];
        let ops = drive
            .insert_nullifiers_v0(&[nullifier])
            .expect("insert single");
        assert_eq!(ops.len(), 1);

        // Apply and confirm presence.
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                Some(&transaction),
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply");

        let mut drive_ops = vec![];
        assert!(drive
            .has_nullifier(
                &nullifier,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }

    #[test]
    fn insert_many_nullifiers_produces_matching_op_count() {
        // Multiple nullifiers - verifies one op per nullifier.
        let drive = setup_drive_with_initial_state_structure(None);

        let nullifiers: Vec<[u8; 32]> = (0u8..5).map(|i| [i; 32]).collect();
        let ops = drive
            .insert_nullifiers_v0(&nullifiers)
            .expect("insert many");
        assert_eq!(ops.len(), 5);
    }
}
