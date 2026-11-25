use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;
use drive::drive::Drive;

/// The refund work erase state validation 0 bills each erase for:
/// [`Drive::erase_refund_recipient_cost`] at `epoch`.
#[inline(always)]
pub(super) fn erase_refund_recipient_reservation_v0(
    drive: &Drive,
    epoch: &Epoch,
    platform_version: &PlatformVersion,
) -> Result<ValidationOperation, Error> {
    Ok(ValidationOperation::PrecalculatedOperation(
        drive
            .erase_refund_recipient_cost(epoch, platform_version)
            .map_err(Error::Drive)?,
    ))
}
