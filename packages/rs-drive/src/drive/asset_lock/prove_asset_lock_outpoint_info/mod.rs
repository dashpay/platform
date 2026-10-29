mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::platform_value::Bytes36;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the committed state of one asset-lock outpoint, including absence.
    pub fn prove_asset_lock_outpoint_info(
        &self,
        outpoint: &Bytes36,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .asset_lock
            .prove_asset_lock_outpoint_info
        {
            0 => self.prove_asset_lock_outpoint_info_v0(outpoint, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_asset_lock_outpoint_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::RootTree;
    use crate::util::batch::drive_op_batch::{DriveOperation, SystemOperationType};
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::asset_lock::reduced_asset_lock_value::{AssetLockValue, AssetLockValueGettersV0};
    use dpp::block::block_info::BlockInfo;
    use dpp::platform_value::Bytes36;
    use dpp::version::PlatformVersion;
    use grovedb::Element;

    fn stored_value(remaining: u64, platform_version: &PlatformVersion) -> AssetLockValue {
        AssetLockValue::new(100, vec![0x51], remaining, vec![], platform_version)
            .expect("asset-lock value")
    }

    fn insert_item(
        drive: &Drive,
        outpoint: &Bytes36,
        bytes: Vec<u8>,
        platform_version: &PlatformVersion,
    ) {
        let transaction = drive.grove.start_transaction();
        drive
            .grove
            .insert(
                &[[RootTree::SpentAssetLockTransactions as u8].as_slice()],
                &outpoint.0,
                Element::new_item(bytes),
                None,
                Some(&transaction),
                &platform_version.drive.grove_version,
            )
            .value
            .expect("insert asset-lock item successfully");
        drive
            .grove
            .commit_transaction(transaction)
            .value
            .expect("commit asset-lock item transaction successfully");
    }

    #[test]
    fn should_prove_and_verify_each_asset_lock_outpoint_state() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let states = [(0u8, 0u64), (1u8, 25u64)];

        for (index, remaining) in states {
            let outpoint = Bytes36::from([index; 36]);
            drive
                .apply_drive_operations(
                    vec![DriveOperation::SystemOperation(
                        SystemOperationType::AddUsedAssetLock {
                            asset_lock_outpoint: outpoint,
                            asset_lock_value: stored_value(remaining, platform_version),
                        },
                    )],
                    true,
                    &BlockInfo::default(),
                    None,
                    platform_version,
                    None,
                )
                .expect("store asset-lock state");

            let proof = drive
                .prove_asset_lock_outpoint_info(&outpoint, None, platform_version)
                .expect("prove asset-lock state");
            let (_, info) =
                Drive::verify_asset_lock_outpoint_info(&proof, &outpoint, platform_version)
                    .expect("verify asset-lock state");

            if remaining == 0 {
                assert_eq!(info, dpp::asset_lock::StoredAssetLockInfo::FullyConsumed);
            } else {
                assert!(matches!(
                    info,
                    dpp::asset_lock::StoredAssetLockInfo::PartiallyConsumed(value)
                        if value.remaining_credit_value() == remaining
                ));
            }
        }

        let absent = Bytes36::from([2u8; 36]);
        let proof = drive
            .prove_asset_lock_outpoint_info(&absent, None, platform_version)
            .expect("prove absent asset-lock state");
        let (_, info) = Drive::verify_asset_lock_outpoint_info(&proof, &absent, platform_version)
            .expect("verify absent asset-lock state");
        assert_eq!(info, dpp::asset_lock::StoredAssetLockInfo::NotPresent);
    }

    #[test]
    fn should_reject_malformed_asset_lock_value() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let outpoint = Bytes36::from([3u8; 36]);
        insert_item(&drive, &outpoint, vec![0xff], platform_version);

        let proof = drive
            .prove_asset_lock_outpoint_info(&outpoint, None, platform_version)
            .expect("prove malformed asset-lock state");
        assert!(
            Drive::verify_asset_lock_outpoint_info(&proof, &outpoint, platform_version).is_err()
        );
    }
}
