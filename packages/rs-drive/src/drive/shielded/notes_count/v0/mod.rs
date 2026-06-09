use crate::drive::shielded::paths::{shielded_credit_pool_path, SHIELDED_NOTES_KEY};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of counting shielded pool notes.
    ///
    /// Returns the total number of items in the CommitmentTree at
    /// `[AddressBalances, "s", [128]]`.
    pub(in crate::drive) fn shielded_pool_notes_count_v0(
        &self,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let pool_path = shielded_credit_pool_path();
        self.grove_commitment_tree_count(
            (&pool_path).into(),
            &[SHIELDED_NOTES_KEY],
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::Drive;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    #[test]
    fn empty_pool_has_zero_notes() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut drive_ops = vec![];

        let count = drive
            .shielded_pool_notes_count_v0(None, &mut drive_ops, platform_version)
            .expect("count on empty pool");
        assert_eq!(count, 0);
    }

    #[test]
    fn count_increments_after_single_insert() {
        // Inserting one note bumps the count to 1.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let ops = Drive::insert_note_op(
            [0xAAu8; 32],
            [0x01u8; 32],
            [0xCCu8; 32],
            vec![0x42u8; 216],
            platform_version,
        )
        .expect("build note op");
        let grove_ops =
            crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
        drive
            .grove_apply_batch_with_add_costs(
                grove_ops,
                false,
                None,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("apply note insert");

        let mut drive_ops = vec![];
        let count = drive
            .shielded_pool_notes_count_v0(None, &mut drive_ops, platform_version)
            .expect("count after insert");
        assert_eq!(count, 1);
    }

    #[test]
    fn count_reflects_multiple_inserts() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        // Insert 3 distinct notes with distinct cmx/nullifier pairs.
        for i in 0u8..3 {
            let mut cmx = [0u8; 32];
            cmx[0] = i + 1;
            let mut rho = [0u8; 32];
            rho[0] = i + 100;
            let mut cv_net = [0u8; 32];
            cv_net[0] = i + 200;

            let ops = Drive::insert_note_op(rho, cmx, cv_net, vec![i; 216], platform_version)
                .expect("build");
            let grove_ops =
                crate::fees::op::LowLevelDriveOperation::grovedb_operations_batch_consume(ops);
            drive
                .grove_apply_batch_with_add_costs(
                    grove_ops,
                    false,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("apply");
        }

        let mut drive_ops = vec![];
        let count = drive
            .shielded_pool_notes_count_v0(None, &mut drive_ops, platform_version)
            .expect("count");
        assert_eq!(count, 3);
    }
}
