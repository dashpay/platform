// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

// TODO: Should be moved to DPP when integration is done

#[cfg(feature = "full")]
use enum_map::EnumMap;

#[cfg(feature = "full")]
use crate::error::fee::FeeError;
#[cfg(feature = "full")]
use crate::error::Error;
#[cfg(feature = "full")]
use crate::fee::op::{BaseOp, LowLevelDriveOperation};
#[cfg(feature = "full")]
use crate::fee::result::FeeResult;
#[cfg(feature = "full")]
use crate::fee_pools::epochs::Epoch;

#[cfg(any(feature = "full", feature = "verify"))]
pub mod credits;
#[cfg(feature = "full")]
pub mod default_costs;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod epoch;
#[cfg(feature = "full")]
pub mod op;
#[cfg(any(feature = "full", feature = "verify"))]
pub mod result;

/// Default original fee multiplier
#[cfg(feature = "full")]
pub const DEFAULT_ORIGINAL_FEE_MULTIPLIER: f64 = 2.0;

/// Calculates fees for the given operations. Returns the storage and processing costs.
#[cfg(feature = "full")]
pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    drive_operations: Option<Vec<LowLevelDriveOperation>>,
    epoch: &Epoch,
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
        for drive_fee_result in LowLevelDriveOperation::consume_to_fees(drive_operations, epoch)? {
            aggregate_fee_result.checked_add_assign(drive_fee_result)?;
        }
    }

    Ok(aggregate_fee_result)
}

#[cfg(feature = "full")]
pub(crate) fn get_overflow_error(str: &'static str) -> Error {
    Error::Fee(FeeError::Overflow(str))
}
