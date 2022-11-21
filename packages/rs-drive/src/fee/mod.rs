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

//! Fees Mod File.
//!

use costs::storage_cost::removal::Identifier;
use enum_map::EnumMap;
use intmap::IntMap;
use std::collections::BTreeMap;

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::op::{BaseOp, DriveOperation};
use crate::fee::removed_bytes_from_epochs_by_identities::RemovedBytesFromEpochsByIdentities;
use crate::fee_pools::epochs::Epoch;

/// Default costs module
pub mod default_costs;
/// Op module
pub mod op;
mod removed_bytes_from_epochs_by_identities;

/// Fee Result
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FeeResult {
    /// Storage fee
    pub storage_fee: u64,
    /// Processing fee
    pub processing_fee: u64,
    /// Removed bytes from identities
    pub removed_bytes_from_identities: RemovedBytesFromEpochsByIdentities,
    /// Removed bytes not needing to be refunded to identities
    pub removed_bytes_from_system: u32,
}

/// Calculates fees for the given operations. Returns the storage and processing costs.
pub fn calculate_fee(
    base_operations: Option<EnumMap<BaseOp, u64>>,
    drive_operations: Option<Vec<DriveOperation>>,
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
        for drive_fee_result in DriveOperation::consume_to_fees(drive_operations, epoch)? {
            aggregate_fee_result.checked_add_assign(drive_fee_result)?;
        }
    }

    Ok(aggregate_fee_result)
}

impl FeeResult {
    /// Adds and self assigns result between two Fee Results
    pub fn checked_add_assign(&mut self, rhs: Self) -> Result<(), Error> {
        self.storage_fee = self
            .storage_fee
            .checked_add(rhs.storage_fee)
            .ok_or(Error::Fee(FeeError::Overflow("storage fee overflow error")))?;
        self.processing_fee =
            self.processing_fee
                .checked_add(rhs.processing_fee)
                .ok_or(Error::Fee(FeeError::Overflow(
                    "processing fee overflow error",
                )))?;
        self.removed_bytes_from_identities
            .checked_add_assign(rhs.removed_bytes_from_identities)?;
        self.removed_bytes_from_system = self
            .removed_bytes_from_system
            .checked_add(rhs.removed_bytes_from_system)
            .ok_or(Error::Fee(FeeError::Overflow(
                "removed_bytes_from_system overflow error",
            )))?;
        Ok(())
    }
}
