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

//! Fee Result
//!
//! Each drive operation returns FeeResult after execution.
//! This result contains fees which are required to pay for
//! computation and storage. It also contains fees to refund
//! for removed data from the state.
//!

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::credits::Credits;
use crate::fee::result::refunds::FeeRefunds;

pub mod refunds;

/// Fee Result
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct FeeResult {
    /// Storage fee
    pub storage_fee: Credits,
    /// Processing fee
    pub processing_fee: Credits,
    /// Credits to refund to identities
    pub fee_refunds: FeeRefunds,
    /// Removed bytes not needing to be refunded to identities
    pub removed_bytes_from_system: u32,
}

impl FeeResult {
    /// Creates a FeeResult instance with specified storage and processing fees
    pub fn default_with_fees(storage_fee: Credits, processing_fee: Credits) -> Self {
        FeeResult {
            storage_fee,
            processing_fee,
            ..Default::default()
        }
    }

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
        self.fee_refunds.checked_add_assign(rhs.fee_refunds)?;
        self.removed_bytes_from_system = self
            .removed_bytes_from_system
            .checked_add(rhs.removed_bytes_from_system)
            .ok_or(Error::Fee(FeeError::Overflow(
                "removed_bytes_from_system overflow error",
            )))?;
        Ok(())
    }
}
