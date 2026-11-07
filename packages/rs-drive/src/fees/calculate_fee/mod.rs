use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::Epoch;
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
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.fees.calculate_fee {
            0 => Self::calculate_fee_v0(
                base_operations,
                drive_operations,
                epoch,
                epochs_per_era,
                &platform_version.fee_version,
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
    use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
    use dpp::fee::epoch::DEFAULT_EPOCHS_PER_ERA;
    use dpp::version::fee::FeeVersion;
    use dpp::version::PLATFORM_VERSIONS;
    use grovedb_costs::storage_cost::removal::{
        StorageRemovalPerEpochByIdentifier, StorageRemovedBytes,
    };
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;

    /// Refunds used to be priced against an empty fee history, which resolves every epoch to the
    /// first schedule. This holds every protocol version to that amount now that they read the
    /// rate from their own schedule.
    #[test]
    fn should_price_a_refund_the_same_at_every_protocol_version() {
        const OWNER: [u8; 32] = [7; 32];
        const STORING_EPOCH: u16 = 12;
        const REMOVED_BYTES: u32 = 1_000;
        const REMOVED_SYSTEM_BYTES: u32 = 50;
        let removing_epoch = Epoch::new(15).expect("expected an epoch");

        let (expected_refund, _) = calculate_storage_fee_refund_amount_and_leftovers(
            REMOVED_BYTES as u64
                * FeeVersion::first()
                    .storage
                    .storage_disk_usage_credit_per_byte,
            STORING_EPOCH,
            removing_epoch.index,
            DEFAULT_EPOCHS_PER_ERA,
        )
        .expect("expected a refund");
        assert!(expected_refund > 0);

        for platform_version in PLATFORM_VERSIONS {
            let mut removal = StorageRemovalPerEpochByIdentifier::default();
            removal
                .entry(OWNER)
                .or_default()
                .insert(STORING_EPOCH, REMOVED_BYTES);
            removal
                .entry([0; 32])
                .or_default()
                .insert(STORING_EPOCH, REMOVED_SYSTEM_BYTES);
            let removed_bytes = LowLevelDriveOperation::CalculatedCostOperation(OperationCost {
                storage_cost: StorageCost {
                    removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(removal),
                    ..Default::default()
                },
                ..Default::default()
            });

            let fee_result = Drive::calculate_fee(
                None,
                Some(vec![removed_bytes]),
                &removing_epoch,
                DEFAULT_EPOCHS_PER_ERA,
                platform_version,
            )
            .expect("expected to calculate the fee");

            assert_eq!(
                fee_result
                    .fee_refunds
                    .get(&OWNER)
                    .and_then(|credits_per_epoch| {
                        credits_per_epoch.get(&STORING_EPOCH).copied()
                    }),
                Some(expected_refund),
                "protocol version {}",
                platform_version.protocol_version
            );
            assert_eq!(fee_result.removed_bytes_from_system, REMOVED_SYSTEM_BYTES);
        }
    }
}
