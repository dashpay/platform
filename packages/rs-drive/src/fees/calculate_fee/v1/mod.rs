use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::{Epoch, EpochIndex};
use dpp::fee::fee_result::FeeResult;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use enum_map::EnumMap;

impl Drive {
    /// Calculates fees for the given operations, pricing each refund at the storage rate of the
    /// epoch that stored the bytes. Generation 0 prices every refund at the block's rate.
    #[inline(always)]
    pub(super) fn calculate_fee_v1(
        &self,
        base_operations: Option<EnumMap<BaseOp, u64>>,
        drive_operations: Option<Vec<LowLevelDriveOperation>>,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut aggregate_fee_result = Self::calculate_fee_v0(
            base_operations,
            None,
            epoch,
            self.config.epochs_per_era,
            &platform_version.fee_version,
        )?;

        if let Some(drive_operations) = drive_operations {
            for drive_fee_result in LowLevelDriveOperation::consume_to_fees_v1(
                drive_operations,
                epoch,
                self.config.epochs_per_era,
                &platform_version.fee_version,
                |storing_epoch| self.storage_rate_of_epoch(storing_epoch, epoch, platform_version),
            )? {
                aggregate_fee_result.checked_add_assign(drive_fee_result)?;
            }
        }

        Ok(aggregate_fee_result)
    }

    /// The storage rate of the fee schedule of the protocol version `storing_epoch` ran.
    ///
    /// Bytes stored in the block's own epoch are priced at the block's rate: that epoch's
    /// protocol version is the block's, and it is only recorded at the end of the epoch's first
    /// block. An earlier epoch that stored bytes had a block, so its first block recorded its
    /// protocol version, nothing removes that record, and it was committed before this block
    /// started, so it is read without a transaction.
    fn storage_rate_of_epoch(
        &self,
        storing_epoch: EpochIndex,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        if storing_epoch == epoch.index {
            return Ok(platform_version
                .fee_version
                .storage
                .storage_disk_usage_credit_per_byte);
        }
        let protocol_version =
            self.get_epoch_protocol_version(&Epoch::new(storing_epoch)?, None, platform_version)?;
        let storing_platform_version =
            PlatformVersion::get(protocol_version).map_err(ProtocolError::from)?;
        Ok(storing_platform_version
            .fee_version
            .storage
            .storage_disk_usage_credit_per_byte)
    }
}
