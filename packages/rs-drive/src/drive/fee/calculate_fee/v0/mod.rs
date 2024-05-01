use crate::drive::Drive;
use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::{BaseOp, LowLevelDriveOperation};
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;

use enum_map::EnumMap;

impl Drive {
    /// Calculates fees for the given operations. Returns the storage and processing costs.
    #[inline(always)]
    pub(super) fn calculate_fee_v0(
        base_operations: Option<EnumMap<BaseOp, u64>>,
        drive_operations: Option<Vec<LowLevelDriveOperation>>,
        epoch: &Epoch,
        epochs_per_era: u16,
    ) -> Result<FeeResult, Error> {
        let mut aggregate_fee_result = FeeResult::default();
        if let Some(base_operations) = base_operations {
            for (base_op, count) in base_operations.iter() {
                match base_op.cost().checked_mul(*count) {
                    None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                    Some(cost) => match aggregate_fee_result.processing_fee.checked_add(cost) {
                        None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                        Some(value) => aggregate_fee_result.processing_fee = value,
                    },
                }
            }
        }

        if let Some(drive_operations) = drive_operations {
            // println!("{:#?}", drive_operations);
            for drive_fee_result in
                LowLevelDriveOperation::consume_to_fees(drive_operations, epoch, epochs_per_era)?
            {
                aggregate_fee_result.checked_add_assign(drive_fee_result)?;
            }
        }

        Ok(aggregate_fee_result)
    }
}
