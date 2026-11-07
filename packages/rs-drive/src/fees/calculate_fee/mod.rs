use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use enum_map::EnumMap;

mod v0;
mod v1;

impl Drive {
    /// Calculates fees for the given operations. Returns the storage and processing costs.
    ///
    /// From generation 1 (protocol version 15) a refund is priced at the storage rate of the
    /// epoch that stored the bytes, read from that epoch's tree; generation 0 prices it at the
    /// block's rate.
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
        &self,
        base_operations: Option<EnumMap<BaseOp, u64>>,
        drive_operations: Option<Vec<LowLevelDriveOperation>>,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        match platform_version.drive.methods.fees.calculate_fee {
            0 => Self::calculate_fee_v0(
                base_operations,
                drive_operations,
                epoch,
                self.config.epochs_per_era,
                &platform_version.fee_version,
            ),
            1 => self.calculate_fee_v1(base_operations, drive_operations, epoch, platform_version),
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
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
    use dpp::fee::Credits;
    use dpp::version::fee::FeeVersion;
    use dpp::version::{ProtocolVersion, PLATFORM_VERSIONS};
    use grovedb_costs::storage_cost::removal::{
        StorageRemovalPerEpochByIdentifier, StorageRemovedBytes,
    };
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use platform_version::version::mocks::v4_test::TEST_PROTOCOL_VERSION_4;

    const OWNER: [u8; 32] = [7; 32];
    const REMOVED_BYTES: u32 = 1_000;
    const REMOVED_SYSTEM_BYTES: u32 = 50;
    const REMOVING_EPOCH: u16 = 15;

    /// A Drive whose `epochs` record the given protocol versions, as their first blocks do.
    fn drive_with_epoch_protocol_versions(epochs: &[(u16, ProtocolVersion)]) -> Drive {
        let drive = setup_drive_with_initial_state_structure(None);
        if epochs.is_empty() {
            return drive;
        }
        let mut batch = GroveDbOpBatch::new();
        for (epoch_index, protocol_version) in epochs {
            let epoch = Epoch::new(*epoch_index).expect("expected an epoch");
            batch.push(epoch.update_protocol_version_operation(*protocol_version));
        }
        drive
            .grove_apply_batch(batch, false, None, &PlatformVersion::latest().drive)
            .expect("expected to record the protocol versions");
        drive
    }

    /// Prices the removal of one element holding `REMOVED_BYTES` of the owner's from each of
    /// `storing_epochs`, and `REMOVED_SYSTEM_BYTES` of the system's, in `REMOVING_EPOCH`.
    fn price_removal(
        drive: &Drive,
        storing_epochs: &[u16],
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut removal = StorageRemovalPerEpochByIdentifier::default();
        for storing_epoch in storing_epochs {
            removal
                .entry(OWNER)
                .or_default()
                .insert(*storing_epoch, REMOVED_BYTES);
        }
        removal
            .entry([0; 32])
            .or_default()
            .insert(REMOVING_EPOCH, REMOVED_SYSTEM_BYTES);
        let removed_bytes = LowLevelDriveOperation::CalculatedCostOperation(OperationCost {
            storage_cost: StorageCost {
                removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(removal),
                ..Default::default()
            },
            ..Default::default()
        });

        drive.calculate_fee(
            None,
            Some(vec![removed_bytes]),
            &Epoch::new(REMOVING_EPOCH).expect("expected an epoch"),
            platform_version,
        )
    }

    fn refund_at_rate(drive: &Drive, storing_epoch: u16, rate: Credits) -> Credits {
        calculate_storage_fee_refund_amount_and_leftovers(
            REMOVED_BYTES as Credits * rate,
            storing_epoch,
            REMOVING_EPOCH,
            drive.config.epochs_per_era,
        )
        .expect("expected a refund")
        .0
    }

    fn owner_refund(fee_result: &FeeResult, storing_epoch: u16) -> Option<Credits> {
        fee_result
            .fee_refunds
            .get(&OWNER)
            .and_then(|credits_per_epoch| credits_per_epoch.get(&storing_epoch).copied())
    }

    /// Every schedule so far has the first schedule's storage rate, so the generations agree.
    #[test]
    fn should_price_a_refund_the_same_at_every_protocol_version() {
        let drive =
            drive_with_epoch_protocol_versions(&[(12, PlatformVersion::latest().protocol_version)]);
        let rate = FeeVersion::first()
            .storage
            .storage_disk_usage_credit_per_byte;
        let expected_refund = refund_at_rate(&drive, 12, rate);
        assert!(expected_refund > 0);

        for platform_version in PLATFORM_VERSIONS {
            let fee_result = price_removal(&drive, &[12], platform_version)
                .expect("expected to calculate the fee");

            assert_eq!(
                owner_refund(&fee_result, 12),
                Some(expected_refund),
                "protocol version {}",
                platform_version.protocol_version
            );
            assert_eq!(fee_result.removed_bytes_from_system, REMOVED_SYSTEM_BYTES);
        }
    }

    #[test]
    fn should_price_a_refund_at_the_storage_rate_of_the_epoch_that_stored_the_bytes() {
        let latest = PlatformVersion::latest();
        // Epoch 12 ran a protocol version charging twice the storage rate; epoch 14 ran the
        // latest. Epoch 15, the removing block's own, records nothing yet.
        let drive = drive_with_epoch_protocol_versions(&[
            (12, TEST_PROTOCOL_VERSION_4),
            (14, latest.protocol_version),
        ]);
        let rate = latest
            .fee_version
            .storage
            .storage_disk_usage_credit_per_byte;

        let fee_result = price_removal(&drive, &[12, 14, REMOVING_EPOCH], latest)
            .expect("expected to calculate the fee");
        assert_eq!(
            owner_refund(&fee_result, 12),
            Some(refund_at_rate(&drive, 12, 2 * rate))
        );
        assert_eq!(
            owner_refund(&fee_result, 14),
            Some(refund_at_rate(&drive, 14, rate))
        );
        assert_eq!(
            owner_refund(&fee_result, REMOVING_EPOCH),
            Some(refund_at_rate(&drive, REMOVING_EPOCH, rate))
        );
        assert_eq!(fee_result.removed_bytes_from_system, REMOVED_SYSTEM_BYTES);

        // Generation 0 prices every refund at the block's rate.
        let protocol_version_14 = PlatformVersion::get(14).expect("expected protocol version 14");
        let fee_result = price_removal(&drive, &[12, 14], protocol_version_14)
            .expect("expected to calculate the fee");
        assert_eq!(
            owner_refund(&fee_result, 12),
            Some(refund_at_rate(&drive, 12, rate))
        );
        assert_eq!(
            owner_refund(&fee_result, 14),
            Some(refund_at_rate(&drive, 14, rate))
        );
    }

    /// An earlier epoch that stored bytes had a block, which recorded its protocol version, so
    /// a missing record is corrupted state, which generation 1 does not price over.
    #[test]
    fn should_not_price_a_refund_from_an_epoch_that_recorded_no_protocol_version() {
        let drive = drive_with_epoch_protocol_versions(&[]);

        assert!(price_removal(&drive, &[13], PlatformVersion::latest()).is_err());
        assert!(price_removal(
            &drive,
            &[13],
            PlatformVersion::get(14).expect("expected protocol version 14")
        )
        .is_ok());
    }
}
