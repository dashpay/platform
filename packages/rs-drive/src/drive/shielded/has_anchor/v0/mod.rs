use crate::drive::shielded::paths::shielded_credit_pool_anchors_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Version 0 implementation of checking whether a shielded anchor exists.
    ///
    /// Performs an O(1) key lookup in the anchors tree at
    /// `[AddressBalances, "s", [192]]`.
    pub(in crate::drive) fn has_shielded_anchor_v0(
        &self,
        anchor: &[u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let anchors_path = shielded_credit_pool_anchors_path();

        self.grove_has_raw(
            (&anchors_path).into(),
            anchor,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            drive_operations,
            &platform_version.drive,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::shielded::paths::shielded_credit_pool_anchors_path;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    #[test]
    fn has_anchor_returns_false_on_empty_tree() {
        // Fresh pool's anchors tree is empty -> false.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let mut drive_ops = vec![];

        let found = drive
            .has_shielded_anchor_v0(&[0u8; 32], None, &mut drive_ops, platform_version)
            .expect("has_shielded_anchor should succeed on empty tree");
        assert!(!found);
    }

    #[test]
    fn has_anchor_returns_true_after_direct_insert() {
        // Direct insertion into the anchors tree is the fastest way to exercise
        // the "present" branch without going through record_anchor_if_changed.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let transaction = drive.grove.start_transaction();
        let grove_version = &platform_version.drive.grove_version;

        let anchor = [0xEEu8; 32];
        drive
            .grove
            .insert(
                &shielded_credit_pool_anchors_path(),
                &anchor,
                Element::new_item(1u64.to_be_bytes().to_vec()),
                None,
                Some(&transaction),
                grove_version,
            )
            .unwrap()
            .expect("insert anchor");

        let mut drive_ops = vec![];
        assert!(drive
            .has_shielded_anchor_v0(
                &anchor,
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
        assert!(!drive
            .has_shielded_anchor_v0(
                &[0xFFu8; 32],
                Some(&transaction),
                &mut drive_ops,
                platform_version
            )
            .unwrap());
    }
}
