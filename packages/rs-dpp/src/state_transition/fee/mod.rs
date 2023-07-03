use crate::block::epoch::Epoch;
use crate::ProtocolError;

use crate::state_transition::fee::fee_result::FeeResult;

pub mod calculate_operation_fees;
pub mod calculate_state_transition_fee_factory;
pub mod calculate_state_transition_fee_from_operations_factory;
pub mod constants;
pub mod default_costs;
pub mod epoch;
pub mod fee_result;
pub mod operations;
use enum_map::EnumMap;

pub type Credits = u64;
pub type SignedCredits = i64;
//
// #[derive(Debug, Clone, PartialEq, Eq, Default)]
// pub struct FeeResult {
//     pub storage_fee: Credits,
//     pub processing_fee: Credits,
//     pub fee_refunds: Vec<Refunds>,
//     pub total_refunds: Credits,
//     pub desired_amount: Credits,
//     pub required_amount: Credits,
// }

// #[derive(Debug, Clone, PartialEq, Eq, Default)]
// pub struct DummyFeesResult {
//     pub storage: Credits,
//     pub processing: Credits,
//     pub fee_refunds: Vec<Refunds>,
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// #[serde(rename = "camelCase")]
// pub struct Refunds {
//     pub identifier: Identifier,
//     pub credits_per_epoch: HashMap<String, Credits>,
// }

/// Default original fee multiplier
pub const DEFAULT_ORIGINAL_FEE_MULTIPLIER: f64 = 2.0;

/// Calculates fees for the given operations. Returns the storage and processing costs.
pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    drive_operations: Option<Vec<LowLevelDriveOperation>>,
    epoch: &Epoch,
) -> Result<FeeResult, ProtocolError> {
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
        for drive_fee_result in LowLevelDriveOperation::consume_to_fees(drive_operations, epoch)? {
            aggregate_fee_result.checked_add_assign(drive_fee_result)?;
        }
    }

    Ok(aggregate_fee_result)
}

pub(crate) fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}
