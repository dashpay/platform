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
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "Drive::calculate_fee".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use grovedb_costs::storage_cost::removal::StorageRemovedBytes::SectionedStorageRemoval;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use intmap::IntMap;
    use platform_version::version::fee::FeeVersion;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    #[test]
    fn should_forward_the_platform_fee_schedule_and_the_fee_history_to_the_implementation() {
        let platform_version = PlatformVersion::latest();
        let identity = [9; 32];
        let operation = || {
            let mut removal = BTreeMap::new();
            removal.insert(identity, IntMap::from_iter([(2u16, 200u32)]));
            CalculatedCostOperation(OperationCost {
                storage_cost: StorageCost {
                    added_bytes: 10,
                    replaced_bytes: 0,
                    removed_bytes: SectionedStorageRemoval(removal),
                },
                ..Default::default()
            })
        };
        let epoch = Epoch::new(6).expect("epoch");
        let history: CachedEpochIndexFeeVersions =
            BTreeMap::from([(0, FeeVersion::get(1).expect("registered"))]);

        let through_dispatcher = Drive::calculate_fee(
            None,
            Some(vec![operation()]),
            &epoch,
            20,
            platform_version,
            Some(&history),
        )
        .expect("latest platform version dispatches calculate_fee");

        let expected = Drive::calculate_fee_v0(
            None,
            Some(vec![operation()]),
            &epoch,
            20,
            &platform_version.fee_version,
            Some(&history),
        )
        .expect("direct implementation call");

        assert_eq!(through_dispatcher, expected);
        assert_eq!(
            through_dispatcher.storage_fee,
            10 * platform_version
                .fee_version
                .storage
                .storage_disk_usage_credit_per_byte
        );
        assert!(through_dispatcher.fee_refunds.get(&identity).is_some());
    }
}
