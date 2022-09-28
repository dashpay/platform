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

//! This module defines a function to get a list of storage credits from a range of epochs.
//!

use crate::drive::Drive;
use crate::fee_pools::epochs::Epoch;
use grovedb::TransactionArg;
use std::ops::Range;

/// Returns a list of storage credits to be distributed to proposers from a range of epochs.
pub fn get_storage_credits_for_distribution_for_epochs_in_range(
    drive: &Drive,
    epoch_range: Range<u16>,
    transaction: TransactionArg,
) -> Vec<u64> {
    epoch_range
        .map(|index| {
            let epoch = Epoch::new(index);
            drive
                .get_epoch_storage_credits_for_distribution(&epoch, transaction)
                .expect("should get storage fee")
        })
        .collect()
}
