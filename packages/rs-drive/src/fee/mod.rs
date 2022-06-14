use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::{BaseOp, DeleteOperation, DriveOperation, QueryOperation};
use enum_map::EnumMap;

pub mod op;

pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    query_operations: Option<Vec<QueryOperation>>,
    drive_operations: Option<Vec<DriveOperation>>,
) -> Result<(i64, u64), Error> {
    let mut storage_cost = 0i64;
    let mut cpu_cost = 0u64;
    if let Some(base_operations) = base_operations {
        for (base_op, count) in base_operations.iter() {
            match base_op.cost().checked_mul(*count) {
                // Todo: This should be made into an overflow error
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(cost) => match cpu_cost.checked_add(cost) {
                    None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                    Some(value) => cpu_cost = value,
                },
            }
        }
    }
    if let Some(query_operations) = query_operations {
        for query_operation in query_operations {
            match cpu_cost.checked_add(query_operation.ephemeral_cost()) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => cpu_cost = value,
            }
        }
    }

    if let Some(drive_operations) = drive_operations {
        for drive_operation in drive_operations {
            match cpu_cost.checked_add(drive_operation.ephemeral_cost()) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => cpu_cost = value,
            }

            match storage_cost.checked_add(drive_operation.storage_cost()) {
                None => return Err(Error::Fee(FeeError::Overflow("overflow error"))),
                Some(value) => storage_cost = value,
            }
        }
    }

    Ok((storage_cost, cpu_cost))
}
