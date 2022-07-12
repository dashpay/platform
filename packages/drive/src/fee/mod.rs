use enum_map::EnumMap;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::{BaseOp, DriveCost, DriveOperation};

pub mod default_costs;
pub mod epoch;
pub mod op;
pub(crate) mod pools;

pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    drive_operations: Option<Vec<DriveOperation>>,
) -> Result<(i64, u64), Error> {
    let mut storage_cost = 0i64;
    let mut processing_cost = 0u64;
    if let Some(base_operations) = base_operations {
        for (base_op, count) in base_operations.iter() {
            match base_op.cost().checked_mul(*count) {
                // Todo: This should be made into an overflow error
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(cost) => match processing_cost.checked_add(cost) {
                    None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                    Some(value) => processing_cost = value,
                },
            }
        }
    }

    if let Some(drive_operations) = drive_operations {
        // println!("{:#?}", drive_operations);
        for drive_operation in DriveOperation::consume_to_costs(drive_operations)? {
            match processing_cost.checked_add(drive_operation.ephemeral_cost()?) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => processing_cost = value,
            }

            match storage_cost.checked_add(drive_operation.storage_cost()?) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => storage_cost = value,
            }
        }
    }

    Ok((storage_cost, processing_cost))
}
