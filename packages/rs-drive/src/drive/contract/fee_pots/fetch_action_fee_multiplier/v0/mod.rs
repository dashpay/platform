use crate::drive::credit_pools::epochs::epoch_key_constants::KEY_FEE_MULTIPLIER;
use crate::drive::credit_pools::epochs::paths::EpochProposers;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::FeeMultiplier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    #[inline(always)]
    pub(super) fn fetch_action_fee_multiplier_with_fee_v0(
        &self,
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, FeeMultiplier), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let path = epoch.get_path();
        let stored = self.grove_get_raw_optional_item(
            (&path).into(),
            KEY_FEE_MULTIPLIER.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let multiplier = match stored {
            Some(encoded) => {
                FeeMultiplier::from_be_bytes(encoded.as_slice().try_into().map_err(|_| {
                    Error::Drive(DriveError::CorruptedSerialization(String::from(
                        "epochs multiplier must be a u64",
                    )))
                })?)
            }
            // The first block of the epoch: the epoch is initialized at the end of the block,
            // with this multiplier.
            None => platform_version
                .fee_version
                .uses_version_fee_multiplier_permille
                .ok_or(Error::Drive(DriveError::NotSupported(
                    "the fee_multiplier_permille must be set in fees to price document action fees",
                )))?,
        };
        let fee = Drive::calculate_fee(
            None,
            Some(drive_operations),
            epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((fee, multiplier))
    }
}
