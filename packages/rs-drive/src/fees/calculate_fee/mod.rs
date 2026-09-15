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
    use crate::fees::op::LowLevelDriveOperation::CalculatedCostOperation;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
    use dpp::fee::Credits;
    use dpp::version::mocks::fee_test::TEST_FEE_VERSION_DOUBLED_STORAGE_RATE;
    use dpp::version::mocks::v2_test::TEST_PLATFORM_V2;
    use grovedb_costs::storage_cost::removal::StorageRemovedBytes::SectionedStorageRemoval;
    use grovedb_costs::storage_cost::StorageCost;
    use grovedb_costs::OperationCost;
    use intmap::IntMap;
    use platform_version::version::drive_versions::DriveFeesMethodVersions;
    use platform_version::version::drive_versions::{DriveMethodVersions, DriveVersion};
    use platform_version::version::fee::FeeVersion;
    use platform_version::version::PlatformVersion;
    use std::collections::BTreeMap;

    const EPOCHS_PER_ERA: u16 = 20;
    const CURRENT_EPOCH: u16 = 15;
    const IDENTITY: [u8; 32] = [9; 32];

    /// One identity removing 100 bytes stored at epoch 5 and 100 bytes stored at epoch 12,
    /// plus 10 freshly added bytes.
    fn removal_operation() -> LowLevelDriveOperation {
        let mut removal = BTreeMap::new();
        removal.insert(
            IDENTITY,
            IntMap::from_iter([(5u16, 100u32), (12u16, 100u32)]),
        );
        CalculatedCostOperation(OperationCost {
            storage_cost: StorageCost {
                added_bytes: 10,
                replaced_bytes: 0,
                removed_bytes: SectionedStorageRemoval(removal),
            },
            ..Default::default()
        })
    }

    /// Fee history with a storage-rate boundary at epoch 10.
    fn boundary_history() -> CachedEpochIndexFeeVersions {
        BTreeMap::from([
            (0, FeeVersion::get(1).expect("registered")),
            (
                10,
                TEST_FEE_VERSION_DOUBLED_STORAGE_RATE
                    .as_static()
                    .expect("mock generation is registered"),
            ),
        ])
    }

    /// A mock platform version whose schedule is the mock generation, so the dispatcher must
    /// hand the fee history through for refunds to be priced at all, running the requested
    /// `calculate_fee` generation.
    fn platform_version_with_doubled_storage_rate(calculate_fee: u16) -> PlatformVersion {
        PlatformVersion {
            fee_version: TEST_FEE_VERSION_DOUBLED_STORAGE_RATE,
            drive: DriveVersion {
                methods: DriveMethodVersions {
                    fees: DriveFeesMethodVersions { calculate_fee },
                    ..TEST_PLATFORM_V2.drive.methods
                },
                ..TEST_PLATFORM_V2.drive
            },
            ..TEST_PLATFORM_V2
        }
    }

    fn first_rate() -> Credits {
        FeeVersion::get(1)
            .expect("registered")
            .storage
            .storage_disk_usage_credit_per_byte
    }

    fn doubled_rate() -> Credits {
        TEST_FEE_VERSION_DOUBLED_STORAGE_RATE
            .storage
            .storage_disk_usage_credit_per_byte
    }

    fn expected_refund(bytes: u32, rate: Credits, storage_epoch: u16) -> Credits {
        let (amount, _) = calculate_storage_fee_refund_amount_and_leftovers(
            bytes as Credits * rate,
            storage_epoch,
            CURRENT_EPOCH,
            EPOCHS_PER_ERA,
        )
        .expect("refund amount");
        amount
    }

    fn refunds_of(fee_result: &FeeResult) -> BTreeMap<u16, Credits> {
        fee_result
            .fee_refunds
            .get(&IDENTITY)
            .expect("identity has refunds")
            .iter()
            .map(|(epoch_index, credits)| (*epoch_index, *credits))
            .collect()
    }

    #[test]
    fn should_forward_the_platform_fee_schedule_and_the_fee_history_to_the_implementation() {
        let platform_version = PlatformVersion::latest();
        let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");
        let history = boundary_history();

        let through_dispatcher = Drive::calculate_fee(
            None,
            Some(vec![removal_operation()]),
            &epoch,
            EPOCHS_PER_ERA,
            platform_version,
            Some(&history),
        )
        .expect("latest platform version dispatches calculate_fee");

        let expected = Drive::calculate_fee_v0(
            None,
            Some(vec![removal_operation()]),
            &epoch,
            EPOCHS_PER_ERA,
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
        assert!(through_dispatcher.fee_refunds.get(&IDENTITY).is_some());
    }

    #[test]
    fn should_price_every_removed_epoch_at_the_current_epoch_rate_through_generation_zero() {
        // Shipped rule, selected by every released protocol version: the rate active at the
        // removal epoch (15, after the boundary) prices every removed epoch.
        let platform_version = platform_version_with_doubled_storage_rate(0);
        let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");
        let history = boundary_history();
        assert_ne!(first_rate(), doubled_rate());

        let fee_result = Drive::calculate_fee(
            None,
            Some(vec![removal_operation()]),
            &epoch,
            EPOCHS_PER_ERA,
            &platform_version,
            Some(&history),
        )
        .expect("history supplied through the dispatcher");

        assert_eq!(fee_result.storage_fee, 10 * doubled_rate());
        assert_eq!(
            refunds_of(&fee_result),
            BTreeMap::from([
                (5, expected_refund(100, doubled_rate(), 5)),
                (12, expected_refund(100, doubled_rate(), 12)),
            ]),
            "generation 0 prices both epochs at the current epoch's rate"
        );
    }

    #[test]
    fn should_price_refunds_across_a_rate_boundary_through_generation_one() {
        let platform_version = platform_version_with_doubled_storage_rate(1);
        let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");
        let history = boundary_history();

        let fee_result = Drive::calculate_fee(
            None,
            Some(vec![removal_operation()]),
            &epoch,
            EPOCHS_PER_ERA,
            &platform_version,
            Some(&history),
        )
        .expect("history supplied through the dispatcher");

        assert_eq!(fee_result.storage_fee, 10 * doubled_rate());
        assert_eq!(
            refunds_of(&fee_result),
            BTreeMap::from([
                (5, expected_refund(100, first_rate(), 5)),
                (12, expected_refund(100, doubled_rate(), 12)),
            ]),
            "generation 1 refunds each epoch at the rate its bytes were charged"
        );
    }

    #[test]
    fn should_agree_across_generations_whenever_every_generation_shares_one_storage_table() {
        // Every input reachable on a released protocol version: a history where every entry is
        // number 1. Both generations then resolve the same rate at every epoch.
        let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");
        let history: CachedEpochIndexFeeVersions = BTreeMap::from([
            (0, FeeVersion::get(1).expect("registered")),
            (10, FeeVersion::get(1).expect("registered")),
        ]);
        let results: Vec<FeeResult> = [0, 1]
            .into_iter()
            .map(|calculate_fee| {
                let platform_version = PlatformVersion {
                    drive: DriveVersion {
                        methods: DriveMethodVersions {
                            fees: DriveFeesMethodVersions { calculate_fee },
                            ..TEST_PLATFORM_V2.drive.methods
                        },
                        ..TEST_PLATFORM_V2.drive
                    },
                    ..TEST_PLATFORM_V2
                };
                Drive::calculate_fee(
                    None,
                    Some(vec![removal_operation()]),
                    &epoch,
                    EPOCHS_PER_ERA,
                    &platform_version,
                    Some(&history),
                )
                .expect("dispatches")
            })
            .collect();

        assert_eq!(results[0], results[1]);
        assert_eq!(
            refunds_of(&results[1]),
            BTreeMap::from([
                (5, expected_refund(100, first_rate(), 5)),
                (12, expected_refund(100, first_rate(), 12)),
            ])
        );
    }

    #[test]
    fn should_reject_a_missing_fee_history_through_the_dispatcher_for_a_later_generation() {
        for calculate_fee in [0, 1] {
            let platform_version = platform_version_with_doubled_storage_rate(calculate_fee);
            let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");

            let error = Drive::calculate_fee(
                None,
                Some(vec![removal_operation()]),
                &epoch,
                EPOCHS_PER_ERA,
                &platform_version,
                None,
            )
            .expect_err("a later generation cannot price refunds without the history");

            assert!(
                matches!(error, Error::Drive(DriveError::CorruptedCodeExecution(_))),
                "calculate_fee {calculate_fee}: unexpected error {error}"
            );
        }
    }

    #[test]
    fn should_reject_an_unknown_calculate_fee_version() {
        let platform_version = platform_version_with_doubled_storage_rate(2);
        let epoch = Epoch::new(CURRENT_EPOCH).expect("epoch");

        let error = Drive::calculate_fee(
            None,
            Some(vec![removal_operation()]),
            &epoch,
            EPOCHS_PER_ERA,
            &platform_version,
            None,
        )
        .expect_err("version 2 is not known");

        assert!(
            matches!(
                error,
                Error::Drive(DriveError::UnknownVersionMismatch { received: 2, .. })
            ),
            "unexpected error {error}"
        );
    }
}
