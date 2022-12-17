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

//! Fee pool constants.
//!
//! This module defines constants related to fee distribution pools.
//!

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::credits::SignedCredits;
use crate::fee::epoch::{
    EpochIndex, SignedCreditsPerEpoch, EPOCHS_PER_YEAR, PERPETUAL_STORAGE_YEARS,
};
use crate::fee::get_overflow_error;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// TODO: Should be updated from the doc

/// The amount of the perpetual storage fee to be paid out to masternodes per year. Adds up to 1.
#[rustfmt::skip]
pub const FEE_DISTRIBUTION_TABLE: [Decimal; PERPETUAL_STORAGE_YEARS as usize] = [
    dec!(0.05000), dec!(0.04800), dec!(0.04600), dec!(0.04400), dec!(0.04200),
    dec!(0.04000), dec!(0.03850), dec!(0.03700), dec!(0.03550), dec!(0.03400),
    dec!(0.03250), dec!(0.03100), dec!(0.02950), dec!(0.02850), dec!(0.02750),
    dec!(0.02650), dec!(0.02550), dec!(0.02450), dec!(0.02350), dec!(0.02250),
    dec!(0.02150), dec!(0.02050), dec!(0.01950), dec!(0.01875), dec!(0.01800),
    dec!(0.01725), dec!(0.01650), dec!(0.01575), dec!(0.01500), dec!(0.01425),
    dec!(0.01350), dec!(0.01275), dec!(0.01200), dec!(0.01125), dec!(0.01050),
    dec!(0.00975), dec!(0.00900), dec!(0.00825), dec!(0.00750), dec!(0.00675),
    dec!(0.00600), dec!(0.00525), dec!(0.00475), dec!(0.00425), dec!(0.00375),
    dec!(0.00325), dec!(0.00275), dec!(0.00225), dec!(0.00175), dec!(0.00125),
];

/// Leftovers in result of divisions and rounding after storage fee distribution to epochs
pub type DistributionLeftoverCredits = SignedCredits;

/// Distributes storage fees to epochs and returns `DistributionLeftoverCredits`
pub fn distribute_storage_fee_to_epochs(
    storage_fee: SignedCredits,
    start_epoch_index: EpochIndex,
    from_epoch_index: EpochIndex,
    credits_per_epochs: &mut SignedCreditsPerEpoch,
) -> Result<DistributionLeftoverCredits, Error> {
    if storage_fee == 0 {
        return Ok(0);
    }

    let storage_fee_dec: Decimal = storage_fee.into();

    let mut distribution_leftover_credits = storage_fee;

    let epochs_per_year = Decimal::from(EPOCHS_PER_YEAR);

    for year in 0..PERPETUAL_STORAGE_YEARS {
        let distribution_for_that_year_ratio = FEE_DISTRIBUTION_TABLE[year as usize];

        let year_fee_share = storage_fee_dec * distribution_for_that_year_ratio;

        let epoch_fee_share_dec = year_fee_share / epochs_per_year;

        let epoch_fee_share: SignedCredits = epoch_fee_share_dec
            .floor()
            .to_i64()
            .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

        let year_start_epoch_index = start_epoch_index + EPOCHS_PER_YEAR * year;

        for epoch_index in year_start_epoch_index..year_start_epoch_index + EPOCHS_PER_YEAR {
            if epoch_index < from_epoch_index {
                continue;
            }

            let epoch_credits: SignedCredits = credits_per_epochs
                .get(&epoch_index)
                .map_or(0, |i| i.to_owned());

            let result_storage_fee: SignedCredits = epoch_credits
                .checked_add(epoch_fee_share)
                .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

            credits_per_epochs.insert(epoch_index, result_storage_fee);

            distribution_leftover_credits = distribution_leftover_credits
                .checked_sub(epoch_fee_share)
                .ok_or(Error::Fee(FeeError::CorruptedCodeExecution(
                    "leftovers bigger than initial value",
                )))?;
        }
    }

    Ok(distribution_leftover_credits)
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    #[test]
    fn test_distribution_table_sum() {
        assert_eq!(
            super::FEE_DISTRIBUTION_TABLE.iter().sum::<Decimal>(),
            dec!(1.0),
        );
    }

    #[test]
    fn test_distribution_of_value() {
        let mut buffer = dec!(0.0);
        let value = Decimal::new(i64::MAX, 0);

        for i in 0..50 {
            let share = value * super::FEE_DISTRIBUTION_TABLE[i];
            buffer += share;
        }

        assert_eq!(buffer, value);
    }
}
