use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::Epoch;
use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use enum_map::EnumMap;

mod v0;
mod v1;

impl Drive {
    /// Calculates fees for the given operations. Returns the storage and processing costs.
    // Developer note : If there would ever need to be more parameters, they could be added as an option.
    // For example, we could transform
    //     pub fn calculate_fee(
    //         base_operations: Option<EnumMap<BaseOp, u64>>,
    //         drive_operations: Option<Vec<LowLevelDriveOperation>>,
    //         epoch: &Epoch,
    //         platform_version: &PlatformVersion,
    //     ) -> Result<FeeResult, Error> {
    // into
    //     pub fn calculate_fee(
    //         base_operations: Option<EnumMap<BaseOp, u64>>,
    //         drive_operations: Option<Vec<LowLevelDriveOperation>>,
    //         new_operations: Option<Vec<NewOperation>>,
    //         epoch: &Epoch,
    //         platform_version: &PlatformVersion,
    //     ) -> Result<FeeResult, Error> {
    // All places in old code would just use a None for new_operations
    // And calculate_fee_v0 would not use new_operations
    pub fn calculate_fee(
        base_operations: Option<EnumMap<BaseOp, u64>>,
        drive_operations: Option<Vec<LowLevelDriveOperation>>,
        epoch: &Epoch,
        epochs_per_era: u16,
        platform_version: &PlatformVersion,
        previous_fee_versions: Option<&CachedEpochIndexFeeVersions>,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.fees.calculate_fee {
            0 => Self::calculate_fee_v0(
                base_operations,
                drive_operations,
                epoch,
                epochs_per_era,
                &platform_version.fee_version,
                previous_fee_versions,
            ),
            1 => Self::calculate_fee_v1(
                base_operations,
                drive_operations,
                epoch,
                epochs_per_era,
                &platform_version.fee_version,
                previous_fee_versions,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::calculate_fee".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
    use grovedb_costs::storage_cost::removal::StorageRemovalPerEpochByIdentifier;
    use grovedb_costs::storage_cost::removal::StorageRemovedBytes;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use platform_version::version::fee::FeeVersion;
    use std::collections::BTreeMap;

    fn owner_attributed_removal() -> Vec<LowLevelDriveOperation> {
        let mut removal = StorageRemovalPerEpochByIdentifier::default();
        removal.entry([7; 32]).or_default().insert(3, 1000);
        vec![LowLevelDriveOperation::CalculatedCostOperation(
            OperationCost {
                seek_count: 1,
                storage_cost: StorageCost {
                    added_bytes: 0,
                    replaced_bytes: 0,
                    removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(removal),
                },
                storage_loaded_bytes: 0,
                hash_node_calls: 0,
                sinsemilla_hash_calls: 0,
            },
        )]
    }

    /// Both generations through the dispatcher on the same operations: the
    /// last frozen protocol version prices an owner-attributed removal without
    /// a fee history, the latest refuses it, and with the history both yield
    /// the same fee result.
    #[test]
    fn should_require_the_fee_history_for_storage_refunds_from_protocol_version_15() {
        let frozen_platform_version = PlatformVersion::get(14).expect("protocol version 14");
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(5).expect("epoch 5");

        let frozen_without_history = Drive::calculate_fee(
            None,
            Some(owner_attributed_removal()),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            frozen_platform_version,
            None,
        )
        .expect("protocol version 14 prices fee version number 1 without a history");
        assert!(
            frozen_without_history.fee_refunds.get(&[7; 32]).is_some(),
            "the shipped generation refunds at the first generation's rates"
        );

        let latest_without_history = Drive::calculate_fee(
            None,
            Some(owner_attributed_removal()),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            platform_version,
            None,
        );
        assert!(
            matches!(
                latest_without_history,
                Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
            ),
            "the latest generation refuses a storage refund without the fee history, got {:?}",
            latest_without_history
        );

        let history: CachedEpochIndexFeeVersions = BTreeMap::from([(0u16, FeeVersion::first())]);
        let frozen_with_history = Drive::calculate_fee(
            None,
            Some(owner_attributed_removal()),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            frozen_platform_version,
            Some(&history),
        )
        .expect("protocol version 14 prices with a history");
        let latest_with_history = Drive::calculate_fee(
            None,
            Some(owner_attributed_removal()),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            platform_version,
            Some(&history),
        )
        .expect("the latest generation prices with a history");

        assert_eq!(frozen_with_history, latest_with_history);
        assert_eq!(frozen_with_history, frozen_without_history);
    }
}
