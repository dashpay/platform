use crate::drive::shielded::paths::shielded_credit_pool_nullifiers_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of checking whether a nullifier exists.
    ///
    /// Performs an O(1) key lookup in the nullifiers tree at
    /// `[AddressBalances, "s", [64]]`.
    pub(in crate::drive) fn has_nullifier_v0(
        &self,
        nullifier: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let nullifiers_path = shielded_credit_pool_nullifiers_path();

        self.grove_has_raw(
            (&nullifiers_path).into(),
            nullifier,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn has_nullifier_returns_false_for_unspent() {
        // A fresh pool has no nullifiers -> has_nullifier returns false.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut drive_ops = vec![];

        let found = drive
            .has_nullifier_v0(&[1u8; 32], None, &mut drive_ops, platform_version)
            .expect("has_nullifier should succeed on empty tree");
        assert!(!found);
    }

    #[test]
    fn has_nullifier_returns_true_after_insert() {
        // After inserting nullifiers, has_nullifier must return true for each
        // and false for an unrelated nullifier.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();

        let nullifier_a = [0xAAu8; 32];
        let nullifier_b = [0xBBu8; 32];
        let ops = drive
            .insert_nullifiers_v0(&[nullifier_a, nullifier_b])
            .expect("insert_nullifiers");
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
            .expect("apply insert_nullifiers ops");

        let mut drive_ops = vec![];
        assert!(drive
            .has_nullifier_v0(
                &nullifier_a,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(drive
            .has_nullifier_v0(
                &nullifier_b,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(!drive
            .has_nullifier_v0(
                &[0xCCu8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }
}
