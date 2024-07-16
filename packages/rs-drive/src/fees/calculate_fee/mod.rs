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
