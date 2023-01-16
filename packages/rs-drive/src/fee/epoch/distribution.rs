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

//! Storage fee distribution into epochs
//!
//! Data is stored in Platform "forever" currently, which is 50 years.
//! To incentivise masternodes to continue store and serve this data,
//! payments are distributed for entire period split into epochs.
//! Every epoch, new aggregated storage fees are distributed among epochs
//! and masternodes receive payouts for previous epoch.
//!

use crate::error::fee::FeeError;
use crate::error::Error;
use crate::fee::credits::{Creditable, Credits, SignedCredits};
use crate::fee::epoch::{
    EpochIndex, SignedCreditsPerEpoch, EPOCHS_PER_YEAR, PERPETUAL_STORAGE_YEARS,
};
use crate::fee::get_overflow_error;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::ops::Mul;

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

type DistributionAmount = Credits;
type DistributionLeftovers = Credits;

/// Distributes storage fees to epochs into `SignedCreditsPerEpoch` and returns leftovers
pub fn distribute_storage_fee_to_epochs_collection(
    credits_per_epochs: &mut SignedCreditsPerEpoch,
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
) -> Result<DistributionLeftovers, Error> {
    distribution_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        |epoch_index, epoch_fee_share| {
            let epoch_credits: SignedCredits =
                credits_per_epochs.get(&epoch_index).map_or(0, |i| *i);

            let result_storage_fee: SignedCredits = epoch_credits
                .checked_add(epoch_fee_share.to_signed()?)
                .ok_or_else(|| {
                    get_overflow_error("updated epoch credits are not fitting to credits max size")
                })?;

            credits_per_epochs.insert(epoch_index, result_storage_fee);

            Ok(())
        },
    )
}

/// Distributes refunds to epochs into `SignedCreditsPerEpoch` and returns leftovers
/// It skips epochs up to specified `skip_until_epoch_index`
pub fn subtract_refunds_from_epoch_credits_collection(
    credits_per_epochs: &mut SignedCreditsPerEpoch,
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    current_epoch_index: EpochIndex,
) -> Result<(), Error> {
    let leftovers = refund_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        current_epoch_index + 1,
        |epoch_index, epoch_fee_share| {
            match credits_per_epochs.entry(epoch_index) {
                Entry::Occupied(occupied_entry) => {
                    let epoch_credits = occupied_entry.into_mut();
                    *epoch_credits = epoch_credits
                        .checked_sub_unsigned(epoch_fee_share)
                        .ok_or_else(|| {
                            get_overflow_error(
                                "updated epoch credits are not fitting to credits min size",
                            )
                        })?;
                }
                Entry::Vacant(epoch_credits) => {
                    epoch_credits.insert(-epoch_fee_share.to_signed()?);
                }
            }
            Ok(())
        },
    )?;

    // We need to remove the leftovers from the current epoch

    let epoch_credits: SignedCredits = credits_per_epochs
        .get(&current_epoch_index)
        .map_or(0, |i| *i);

    let result_storage_fee: SignedCredits = epoch_credits
        .checked_sub_unsigned(leftovers)
        .ok_or_else(|| {
            get_overflow_error("updated epoch credits are not fitting to credits min size")
        })?;

    credits_per_epochs.insert(current_epoch_index, result_storage_fee);

    Ok(())
}

/// Calculates leftovers and amount of credits by distributing storage fees to epochs
pub fn calculate_storage_fee_refund_amount_and_leftovers(
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    current_epoch_index: EpochIndex,
) -> Result<(DistributionAmount, DistributionLeftovers), Error> {
    let mut skipped_amount = 0;

    let leftovers = distribution_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        |epoch_index, epoch_fee_share| {
            if epoch_index < current_epoch_index + 1 {
                skipped_amount += epoch_fee_share;
            }

            Ok(())
        },
    )?;

    Ok((storage_fee - skipped_amount - leftovers, leftovers))
}

fn original_removed_credits_multiplier_from(
    start_epoch_index: EpochIndex,
    start_repayment_from_epoch_index: EpochIndex,
) -> Decimal {
    let paid_epochs = start_repayment_from_epoch_index - start_epoch_index;

    let current_year = (paid_epochs / EPOCHS_PER_YEAR) as usize;

    let ratio_used: Decimal = FEE_DISTRIBUTION_TABLE
        .iter()
        .enumerate()
        .filter_map(|(year, epoch_multiplier)| match year.cmp(&current_year) {
            Ordering::Less => None,
            Ordering::Equal => {
                let amount_epochs_left_in_year = EPOCHS_PER_YEAR - paid_epochs % EPOCHS_PER_YEAR;
                Some(epoch_multiplier.mul(
                    Decimal::from(amount_epochs_left_in_year) / Decimal::from(EPOCHS_PER_YEAR),
                ))
            }
            Ordering::Greater => Some(*epoch_multiplier),
        })
        .sum();

    dec!(1) / ratio_used
}

/// Let's imagine that we are refunding something from epoch 5
/// We are at Epoch 12
/// The refund amount is from Epoch 13 (current + 1) to Epoch 1005 (5 + 1000)
/// We need to figure out the amount extra those 8 costed
fn restore_original_removed_credits_amount(
    refund_amount: Decimal,
    start_epoch_index: EpochIndex,
    start_repayment_from_epoch_index: EpochIndex,
) -> Result<Decimal, Error> {
    let multiplier = original_removed_credits_multiplier_from(
        start_epoch_index,
        start_repayment_from_epoch_index,
    );

    refund_amount
        .checked_mul(multiplier)
        .ok_or(Error::Fee(FeeError::Overflow(
            "overflow when multiplying with the multiplier (this should be impossible)",
        )))
}

/// Distributes storage fees to epochs and call function for each epoch.
/// Returns leftovers
fn distribution_storage_fee_to_epochs_map<F>(
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    mut map_function: F,
) -> Result<DistributionLeftovers, Error>
where
    F: FnMut(EpochIndex, Credits) -> Result<(), Error>,
{
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

        let epoch_fee_share: Credits = epoch_fee_share_dec
            .floor()
            .to_u64()
            .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

        let year_start_epoch_index = start_epoch_index + EPOCHS_PER_YEAR * year;

        for epoch_index in year_start_epoch_index..year_start_epoch_index + EPOCHS_PER_YEAR {
            map_function(epoch_index, epoch_fee_share)?;

            distribution_leftover_credits = distribution_leftover_credits
                .checked_sub(epoch_fee_share)
                .ok_or(Error::Fee(FeeError::CorruptedCodeExecution(
                    "leftovers bigger than initial value",
                )))?;
        }
    }

    Ok(distribution_leftover_credits)
}

/// Distributes recovered by multiplier original removed
/// credits to epochs and call function for each epoch.
/// Leftovers are added to current epoch
fn refund_storage_fee_to_epochs_map<F>(
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    skip_until_epoch_index: EpochIndex,
    mut map_function: F,
) -> Result<DistributionLeftovers, Error>
where
    F: FnMut(EpochIndex, Credits) -> Result<(), Error>,
{
    if storage_fee == 0 {
        return Ok(0);
    }

    let storage_fee_dec: Decimal = storage_fee.into();

    let mut distribution_leftover_credits = storage_fee;

    let epochs_per_year = Decimal::from(EPOCHS_PER_YEAR);

    let start_year: u16 = (skip_until_epoch_index - start_epoch_index) / EPOCHS_PER_YEAR;

    // Let's imagine that we are refunding something from epoch 5
    // We are at Epoch 12
    // The refund amount is from Epoch 13 (current + 1) to Epoch 1005 (5 + 1000)
    // We need to figure out the amount extra those 8 costed
    let estimated_storage_fee_dec = restore_original_removed_credits_amount(
        storage_fee_dec,
        start_epoch_index,
        skip_until_epoch_index,
    )?;

    for year in start_year..PERPETUAL_STORAGE_YEARS {
        let distribution_for_that_year_ratio = FEE_DISTRIBUTION_TABLE[year as usize];

        let estimated_year_fee_share = estimated_storage_fee_dec * distribution_for_that_year_ratio;

        let estimated_epoch_fee_share_dec = estimated_year_fee_share / epochs_per_year;

        let estimated_epoch_fee_share: Credits = estimated_epoch_fee_share_dec
            .floor()
            .to_u64()
            .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

        let year_start_epoch_index = if year == start_year {
            skip_until_epoch_index
        } else {
            start_epoch_index + EPOCHS_PER_YEAR * year
        };

        let year_end_epoch_index = start_epoch_index + ((year + 1) * EPOCHS_PER_YEAR);

        for epoch_index in year_start_epoch_index..year_end_epoch_index {
            map_function(epoch_index, estimated_epoch_fee_share)?;

            distribution_leftover_credits = distribution_leftover_credits
                .checked_sub(estimated_epoch_fee_share)
                .ok_or(Error::Fee(FeeError::CorruptedCodeExecution(
                    "leftovers bigger than initial value",
                )))?;
        }
    }
    Ok(distribution_leftover_credits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fee::credits::{Creditable, MAX_CREDITS};
    use crate::fee::epoch::GENESIS_EPOCH_INDEX;
    use crate::fee::epoch::PERPETUAL_STORAGE_EPOCHS;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    mod original_removed_credits_multiplier_from {
        use super::*;

        #[test]
        fn should_create_multiplier_for_epochs_since_the_beginning() {
            // the multiplier should be
            let epoch_0_cost = dec!(0.05000) / dec!(20.0);
            let multiplier_should_be = dec!(1.0) / (dec!(1.0) - epoch_0_cost);

            let multiplier = original_removed_credits_multiplier_from(0, 1);

            assert_eq!(multiplier_should_be, multiplier);
        }

        #[test]
        fn should_create_multiplier_for_epochs_since_24_and_repayed_since_43() {
            // there were 19 epochs
            let epoch_0_cost = dec!(19.0) * dec!(0.05000) / dec!(20.0);

            let multiplier_should_be = dec!(1.0) / (dec!(1.0) - epoch_0_cost);

            let multiplier = original_removed_credits_multiplier_from(24, 43);

            assert_eq!(multiplier_should_be, multiplier);
        }
    }

    mod fee_distribution_table {
        use super::*;

        #[test]
        fn should_have_sum_of_1() {
            assert_eq!(FEE_DISTRIBUTION_TABLE.iter().sum::<Decimal>(), dec!(1.0),);
        }

        #[test]
        fn should_distribute_value() {
            let value = Decimal::from(i64::MAX);

            let calculated_value: Decimal = FEE_DISTRIBUTION_TABLE
                .into_iter()
                .map(|ratio| value * ratio)
                .sum();

            assert_eq!(calculated_value, value);
        }
    }

    mod distribution_storage_fee_to_epochs_map {
        use super::*;

        #[test]
        fn should_distribute_nothing_if_storage_fees_are_zero() {
            let mut calls = 0;

            let leftovers =
                distribution_storage_fee_to_epochs_map(0, GENESIS_EPOCH_INDEX, |_, _| {
                    calls += 1;

                    Ok(())
                })
                .expect("should distribute storage fee");

            assert_eq!(calls, 0);
            assert_eq!(leftovers, 0);
        }

        #[test]
        fn should_call_function_for_each_epoch_for_50_years_sequentially() {
            let mut calls = 0;

            let mut previous_epoch_index = -1;

            let leftovers = distribution_storage_fee_to_epochs_map(
                100000,
                GENESIS_EPOCH_INDEX,
                |epoch_index, _| {
                    assert_eq!(epoch_index as i32, previous_epoch_index + 1);
                    previous_epoch_index = epoch_index as i32;

                    calls += 1;

                    Ok(())
                },
            )
            .expect("should distribute storage fee");

            assert_eq!(calls, PERPETUAL_STORAGE_EPOCHS);
            assert_eq!(leftovers, 360);
        }
    }

    mod distribute_storage_fee_to_epochs_collection {
        use super::*;

        #[test]
        fn should_distribute_max_credits_value_without_overflow() {
            let storage_fee = MAX_CREDITS;

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                GENESIS_EPOCH_INDEX,
            )
            .expect("should distribute storage fee");

            // check leftover
            assert_eq!(leftovers, 507);
        }

        #[test]
        fn should_deterministically_distribute_fees() {
            let storage_fee = 1000000;
            let current_epoch_index = 42;

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                current_epoch_index,
            )
            .expect("should distribute storage fee");

            // check leftover
            assert_eq!(leftovers, 180);

            // compare them with reference table
            #[rustfmt::skip]
                let reference_fees: [SignedCredits; PERPETUAL_STORAGE_EPOCHS as usize] = [
                2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500,
                2500, 2500, 2500, 2500, 2500, 2500, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400,
                2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2300, 2300,
                2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300, 2300,
                2300, 2300, 2300, 2300, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200,
                2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2200, 2100, 2100, 2100, 2100,
                2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100, 2100,
                2100, 2100, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000,
                2000, 2000, 2000, 2000, 2000, 2000, 2000, 2000, 1925, 1925, 1925, 1925, 1925, 1925,
                1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925, 1925,
                1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850, 1850,
                1850, 1850, 1850, 1850, 1850, 1850, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775,
                1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1775, 1700, 1700,
                1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700, 1700,
                1700, 1700, 1700, 1700, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625,
                1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1625, 1550, 1550, 1550, 1550,
                1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550, 1550,
                1550, 1550, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475,
                1475, 1475, 1475, 1475, 1475, 1475, 1475, 1475, 1425, 1425, 1425, 1425, 1425, 1425,
                1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425, 1425,
                1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375, 1375,
                1375, 1375, 1375, 1375, 1375, 1375, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325,
                1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1325, 1275, 1275,
                1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275, 1275,
                1275, 1275, 1275, 1275, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225,
                1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1225, 1175, 1175, 1175, 1175,
                1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175, 1175,
                1175, 1175, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125,
                1125, 1125, 1125, 1125, 1125, 1125, 1125, 1125, 1075, 1075, 1075, 1075, 1075, 1075,
                1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075, 1075,
                1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025, 1025,
                1025, 1025, 1025, 1025, 1025, 1025, 975, 975, 975, 975, 975, 975, 975, 975, 975,
                975, 975, 975, 975, 975, 975, 975, 975, 975, 975, 975, 937, 937, 937, 937, 937,
                937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 937, 900,
                900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900, 900,
                900, 900, 900, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862, 862,
                862, 862, 862, 862, 862, 862, 862, 825, 825, 825, 825, 825, 825, 825, 825, 825,
                825, 825, 825, 825, 825, 825, 825, 825, 825, 825, 825, 787, 787, 787, 787, 787,
                787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 787, 750,
                750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750, 750,
                750, 750, 750, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712, 712,
                712, 712, 712, 712, 712, 712, 712, 675, 675, 675, 675, 675, 675, 675, 675, 675,
                675, 675, 675, 675, 675, 675, 675, 675, 675, 675, 675, 637, 637, 637, 637, 637,
                637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 637, 600,
                600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600, 600,
                600, 600, 600, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562, 562,
                562, 562, 562, 562, 562, 562, 562, 525, 525, 525, 525, 525, 525, 525, 525, 525,
                525, 525, 525, 525, 525, 525, 525, 525, 525, 525, 525, 487, 487, 487, 487, 487,
                487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 487, 450,
                450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450, 450,
                450, 450, 450, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412, 412,
                412, 412, 412, 412, 412, 412, 412, 375, 375, 375, 375, 375, 375, 375, 375, 375,
                375, 375, 375, 375, 375, 375, 375, 375, 375, 375, 375, 337, 337, 337, 337, 337,
                337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 337, 300,
                300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300, 300,
                300, 300, 300, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262, 262,
                262, 262, 262, 262, 262, 262, 262, 237, 237, 237, 237, 237, 237, 237, 237, 237,
                237, 237, 237, 237, 237, 237, 237, 237, 237, 237, 237, 212, 212, 212, 212, 212,
                212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 212, 187,
                187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187, 187,
                187, 187, 187, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162, 162,
                162, 162, 162, 162, 162, 162, 162, 137, 137, 137, 137, 137, 137, 137, 137, 137,
                137, 137, 137, 137, 137, 137, 137, 137, 137, 137, 137, 112, 112, 112, 112, 112,
                112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 112, 87,
                87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 87, 62,
                62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62, 62
            ];

            assert_eq!(
                credits_per_epochs.clone().into_values().collect::<Vec<_>>(),
                reference_fees
            );

            let total_distributed: SignedCredits = credits_per_epochs.values().sum();

            assert_eq!(total_distributed.to_unsigned() + leftovers, storage_fee);

            /*

            Repeat distribution to ensure deterministic results

             */

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                current_epoch_index,
            )
            .expect("should distribute storage fee");

            // assert that all the values doubled meaning that distribution is reproducible
            assert_eq!(
                credits_per_epochs.into_values().collect::<Vec<_>>(),
                reference_fees
                    .into_iter()
                    .map(|val| val * 2)
                    .collect::<Vec<_>>()
            );

            assert_eq!(leftovers, 180);
        }
    }

    mod subtract_refunds_from_epoch_credits_collection {
        use super::*;

        #[test]
        fn should_deduct_refunds_from_collection_since_specific_epoch_start_at_genesis() {
            // Example: Bob inserted an element into the tree
            // He paid slightly more than 1.2 Million credits for this operation that happened at epoch 0.
            // At epoch 42 we are asking for a refund.
            // The refund is 1.07 Million credits that were left from the 1.2.

            let start_epoch_index: EpochIndex = GENESIS_EPOCH_INDEX;
            const REFUNDED_EPOCH_INDEX: EpochIndex = 42;
            let original_storage_fee = 1200005;

            let (refund_amount, leftovers) = calculate_storage_fee_refund_amount_and_leftovers(
                original_storage_fee,
                start_epoch_index,
                REFUNDED_EPOCH_INDEX,
            )
            .expect("should distribute storage fee");

            assert_eq!(refund_amount, 1074120);
            assert_eq!(leftovers, 5);

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                refund_amount,
                start_epoch_index,
                REFUNDED_EPOCH_INDEX,
            )
            .expect("should distribute storage fee");

            // compare them with reference table
            // we expect to get 0 for the change of the current epochs balance
            // this is because there was only 1 refund so leftovers wouldn't have any effect
            #[rustfmt::skip]
            let reference_fees: [SignedCredits;
                (PERPETUAL_STORAGE_EPOCHS - REFUNDED_EPOCH_INDEX) as usize] = [0, -2760, -2760, -2760,
                -2760, -2760, -2760, -2760, -2760, -2760, -2760, -2760, -2760, -2760, -2760, -2760,
                -2760, -2760, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640,
                -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2640, -2520, -2520,
                -2520, -2520, -2520, -2520, -2520, -2520, -2520, -2520, -2520, -2520, -2520, -2520,
                -2520, -2520, -2520, -2520, -2520, -2520, -2400, -2400, -2400, -2400, -2400, -2400,
                -2400, -2400, -2400, -2400, -2400, -2400, -2400, -2400, -2400, -2400, -2400, -2400,
                -2400, -2400, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310,
                -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2310, -2220, -2220,
                -2220, -2220, -2220, -2220, -2220, -2220, -2220, -2220, -2220, -2220, -2220, -2220,
                -2220, -2220, -2220, -2220, -2220, -2220, -2130, -2130, -2130, -2130, -2130, -2130,
                -2130, -2130, -2130, -2130, -2130, -2130, -2130, -2130, -2130, -2130, -2130, -2130,
                -2130, -2130, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040,
                -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -2040, -1950, -1950,
                -1950, -1950, -1950, -1950, -1950, -1950, -1950, -1950, -1950, -1950, -1950, -1950,
                -1950, -1950, -1950, -1950, -1950, -1950, -1860, -1860, -1860, -1860, -1860, -1860,
                -1860, -1860, -1860, -1860, -1860, -1860, -1860, -1860, -1860, -1860, -1860, -1860,
                -1860, -1860, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770,
                -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1770, -1710, -1710,
                -1710, -1710, -1710, -1710, -1710, -1710, -1710, -1710, -1710, -1710, -1710, -1710,
                -1710, -1710, -1710, -1710, -1710, -1710, -1650, -1650, -1650, -1650, -1650, -1650,
                -1650, -1650, -1650, -1650, -1650, -1650, -1650, -1650, -1650, -1650, -1650, -1650,
                -1650, -1650, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590,
                -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1590, -1530, -1530,
                -1530, -1530, -1530, -1530, -1530, -1530, -1530, -1530, -1530, -1530, -1530, -1530,
                -1530, -1530, -1530, -1530, -1530, -1530, -1470, -1470, -1470, -1470, -1470, -1470,
                -1470, -1470, -1470, -1470, -1470, -1470, -1470, -1470, -1470, -1470, -1470, -1470,
                -1470, -1470, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410,
                -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1410, -1350, -1350,
                -1350, -1350, -1350, -1350, -1350, -1350, -1350, -1350, -1350, -1350, -1350, -1350,
                -1350, -1350, -1350, -1350, -1350, -1350, -1290, -1290, -1290, -1290, -1290, -1290,
                -1290, -1290, -1290, -1290, -1290, -1290, -1290, -1290, -1290, -1290, -1290, -1290,
                -1290, -1290, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230,
                -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1230, -1170, -1170,
                -1170, -1170, -1170, -1170, -1170, -1170, -1170, -1170, -1170, -1170, -1170, -1170,
                -1170, -1170, -1170, -1170, -1170, -1170, -1125, -1125, -1125, -1125, -1125, -1125,
                -1125, -1125, -1125, -1125, -1125, -1125, -1125, -1125, -1125, -1125, -1125, -1125,
                -1125, -1125, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080,
                -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1080, -1035, -1035,
                -1035, -1035, -1035, -1035, -1035, -1035, -1035, -1035, -1035, -1035, -1035, -1035,
                -1035, -1035, -1035, -1035, -1035, -1035, -990, -990, -990, -990, -990, -990, -990,
                -990, -990, -990, -990, -990, -990, -990, -990, -990, -990, -990, -990, -990, -945,
                -945, -945, -945, -945, -945, -945, -945, -945, -945, -945, -945, -945, -945, -945,
                -945, -945, -945, -945, -945, -900, -900, -900, -900, -900, -900, -900, -900, -900,
                -900, -900, -900, -900, -900, -900, -900, -900, -900, -900, -900, -855, -855, -855,
                -855, -855, -855, -855, -855, -855, -855, -855, -855, -855, -855, -855, -855, -855,
                -855, -855, -855, -810, -810, -810, -810, -810, -810, -810, -810, -810, -810, -810,
                -810, -810, -810, -810, -810, -810, -810, -810, -810, -765, -765, -765, -765, -765,
                -765, -765, -765, -765, -765, -765, -765, -765, -765, -765, -765, -765, -765, -765,
                -765, -720, -720, -720, -720, -720, -720, -720, -720, -720, -720, -720, -720, -720,
                -720, -720, -720, -720, -720, -720, -720, -675, -675, -675, -675, -675, -675, -675,
                -675, -675, -675, -675, -675, -675, -675, -675, -675, -675, -675, -675, -675, -630,
                -630, -630, -630, -630, -630, -630, -630, -630, -630, -630, -630, -630, -630, -630,
                -630, -630, -630, -630, -630, -585, -585, -585, -585, -585, -585, -585, -585, -585,
                -585, -585, -585, -585, -585, -585, -585, -585, -585, -585, -585, -540, -540, -540,
                -540, -540, -540, -540, -540, -540, -540, -540, -540, -540, -540, -540, -540, -540,
                -540, -540, -540, -495, -495, -495, -495, -495, -495, -495, -495, -495, -495, -495,
                -495, -495, -495, -495, -495, -495, -495, -495, -495, -450, -450, -450, -450, -450,
                -450, -450, -450, -450, -450, -450, -450, -450, -450, -450, -450, -450, -450, -450,
                -450, -405, -405, -405, -405, -405, -405, -405, -405, -405, -405, -405, -405, -405,
                -405, -405, -405, -405, -405, -405, -405, -360, -360, -360, -360, -360, -360, -360,
                -360, -360, -360, -360, -360, -360, -360, -360, -360, -360, -360, -360, -360, -315,
                -315, -315, -315, -315, -315, -315, -315, -315, -315, -315, -315, -315, -315, -315,
                -315, -315, -315, -315, -315, -285, -285, -285, -285, -285, -285, -285, -285, -285,
                -285, -285, -285, -285, -285, -285, -285, -285, -285, -285, -285, -255, -255, -255,
                -255, -255, -255, -255, -255, -255, -255, -255, -255, -255, -255, -255, -255, -255,
                -255, -255, -255, -225, -225, -225, -225, -225, -225, -225, -225, -225, -225, -225,
                -225, -225, -225, -225, -225, -225, -225, -225, -225, -195, -195, -195, -195, -195,
                -195, -195, -195, -195, -195, -195, -195, -195, -195, -195, -195, -195, -195, -195,
                -195, -165, -165, -165, -165, -165, -165, -165, -165, -165, -165, -165, -165, -165,
                -165, -165, -165, -165, -165, -165, -165, -135, -135, -135, -135, -135, -135, -135,
                -135, -135, -135, -135, -135, -135, -135, -135, -135, -135, -135, -135, -135, -105,
                -105, -105, -105, -105, -105, -105, -105, -105, -105, -105, -105, -105, -105, -105,
                -105, -105, -105, -105, -105, -75, -75, -75, -75, -75, -75, -75, -75, -75, -75, -75,
                -75, -75, -75, -75, -75, -75, -75, -75, -75];

            assert_eq!(
                credits_per_epochs.clone().into_values().collect::<Vec<_>>(),
                reference_fees
            );

            let total_distributed: SignedCredits = credits_per_epochs.values().sum();

            assert_eq!(total_distributed.to_unsigned(), refund_amount);
        }

        #[test]
        fn should_deduct_refunds_from_collection_start_epoch_doesnt_matter_check() {
            for start_epoch_index in 0..150 {
                let current_epoch_index_where_refund_occurred: EpochIndex = start_epoch_index + 14;

                let original_storage_fee = 3405507;
                let (refund_amount, leftovers) = calculate_storage_fee_refund_amount_and_leftovers(
                    original_storage_fee,
                    start_epoch_index,
                    current_epoch_index_where_refund_occurred,
                )
                .expect("should distribute storage fee");

                assert_eq!(refund_amount, 3277305);
                assert_eq!(leftovers, 507);

                let multiplier = original_removed_credits_multiplier_from(
                    start_epoch_index,
                    current_epoch_index_where_refund_occurred + 1,
                );

                // it's not going to be completely perfect but it's good enough
                // there were 24 epochs, on average we would be 12 off
                // while we could incorporate this offset into the multiplier it would
                // be overkill for such low credit amounts
                assert!(
                    (Decimal::from(refund_amount) * multiplier)
                        .abs_sub(&Decimal::from(original_storage_fee - leftovers))
                        < dec!(100)
                );

                // we do however want to make sure the multiplier makes things smaller
                assert!(
                    (Decimal::from(refund_amount) * multiplier)
                        < Decimal::from(original_storage_fee - leftovers)
                );

                let mut credits_per_epochs = SignedCreditsPerEpoch::default();

                subtract_refunds_from_epoch_credits_collection(
                    &mut credits_per_epochs,
                    refund_amount,
                    start_epoch_index,
                    current_epoch_index_where_refund_occurred,
                )
                .expect("should distribute storage fee");
                // compare them with reference table
                // we expect to get 0 for the change of the current epochs balance
                // this is because there was only 1 refund so leftovers wouldn't have any effect
                #[rustfmt::skip]
                    let reference_fees: Vec<SignedCredits> =
                    vec![-525, -8512, -8512, -8512, -8512, -8512, -8171, -8171, -8171, -8171, -8171, -8171,
                        -8171, -8171, -8171, -8171, -8171, -8171, -8171, -8171, -8171, -8171, -8171,
                        -8171, -8171, -8171, -7831, -7831, -7831, -7831, -7831, -7831, -7831, -7831,
                        -7831, -7831, -7831, -7831, -7831, -7831, -7831, -7831, -7831, -7831, -7831,
                        -7831, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490,
                        -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7490, -7150,
                        -7150, -7150, -7150, -7150, -7150, -7150, -7150, -7150, -7150, -7150, -7150,
                        -7150, -7150, -7150, -7150, -7150, -7150, -7150, -7150, -6809, -6809, -6809,
                        -6809, -6809, -6809, -6809, -6809, -6809, -6809, -6809, -6809, -6809, -6809,
                        -6809, -6809, -6809, -6809, -6809, -6809, -6554, -6554, -6554, -6554, -6554,
                        -6554, -6554, -6554, -6554, -6554, -6554, -6554, -6554, -6554, -6554, -6554,
                        -6554, -6554, -6554, -6554, -6299, -6299, -6299, -6299, -6299, -6299, -6299,
                        -6299, -6299, -6299, -6299, -6299, -6299, -6299, -6299, -6299, -6299, -6299,
                        -6299, -6299, -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043,
                        -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043, -6043,
                        -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788,
                        -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5788, -5533, -5533,
                        -5533, -5533, -5533, -5533, -5533, -5533, -5533, -5533, -5533, -5533, -5533,
                        -5533, -5533, -5533, -5533, -5533, -5533, -5533, -5277, -5277, -5277, -5277,
                        -5277, -5277, -5277, -5277, -5277, -5277, -5277, -5277, -5277, -5277, -5277,
                        -5277, -5277, -5277, -5277, -5277, -5022, -5022, -5022, -5022, -5022, -5022,
                        -5022, -5022, -5022, -5022, -5022, -5022, -5022, -5022, -5022, -5022, -5022,
                        -5022, -5022, -5022, -4852, -4852, -4852, -4852, -4852, -4852, -4852, -4852,
                        -4852, -4852, -4852, -4852, -4852, -4852, -4852, -4852, -4852, -4852, -4852,
                        -4852, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681,
                        -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4681, -4511,
                        -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4511,
                        -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4511, -4341, -4341, -4341,
                        -4341, -4341, -4341, -4341, -4341, -4341, -4341, -4341, -4341, -4341, -4341,
                        -4341, -4341, -4341, -4341, -4341, -4341, -4171, -4171, -4171, -4171, -4171,
                        -4171, -4171, -4171, -4171, -4171, -4171, -4171, -4171, -4171, -4171, -4171,
                        -4171, -4171, -4171, -4171, -4000, -4000, -4000, -4000, -4000, -4000, -4000,
                        -4000, -4000, -4000, -4000, -4000, -4000, -4000, -4000, -4000, -4000, -4000,
                        -4000, -4000, -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830,
                        -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830, -3830,
                        -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660,
                        -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3660, -3490, -3490,
                        -3490, -3490, -3490, -3490, -3490, -3490, -3490, -3490, -3490, -3490, -3490,
                        -3490, -3490, -3490, -3490, -3490, -3490, -3490, -3319, -3319, -3319, -3319,
                        -3319, -3319, -3319, -3319, -3319, -3319, -3319, -3319, -3319, -3319, -3319,
                        -3319, -3319, -3319, -3319, -3319, -3192, -3192, -3192, -3192, -3192, -3192,
                        -3192, -3192, -3192, -3192, -3192, -3192, -3192, -3192, -3192, -3192, -3192,
                        -3192, -3192, -3192, -3064, -3064, -3064, -3064, -3064, -3064, -3064, -3064,
                        -3064, -3064, -3064, -3064, -3064, -3064, -3064, -3064, -3064, -3064, -3064,
                        -3064, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936,
                        -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2936, -2809,
                        -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2809,
                        -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2809, -2681, -2681, -2681,
                        -2681, -2681, -2681, -2681, -2681, -2681, -2681, -2681, -2681, -2681, -2681,
                        -2681, -2681, -2681, -2681, -2681, -2681, -2553, -2553, -2553, -2553, -2553,
                        -2553, -2553, -2553, -2553, -2553, -2553, -2553, -2553, -2553, -2553, -2553,
                        -2553, -2553, -2553, -2553, -2426, -2426, -2426, -2426, -2426, -2426, -2426,
                        -2426, -2426, -2426, -2426, -2426, -2426, -2426, -2426, -2426, -2426, -2426,
                        -2426, -2426, -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298,
                        -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298, -2298,
                        -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170,
                        -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2170, -2042, -2042,
                        -2042, -2042, -2042, -2042, -2042, -2042, -2042, -2042, -2042, -2042, -2042,
                        -2042, -2042, -2042, -2042, -2042, -2042, -2042, -1915, -1915, -1915, -1915,
                        -1915, -1915, -1915, -1915, -1915, -1915, -1915, -1915, -1915, -1915, -1915,
                        -1915, -1915, -1915, -1915, -1915, -1787, -1787, -1787, -1787, -1787, -1787,
                        -1787, -1787, -1787, -1787, -1787, -1787, -1787, -1787, -1787, -1787, -1787,
                        -1787, -1787, -1787, -1659, -1659, -1659, -1659, -1659, -1659, -1659, -1659,
                        -1659, -1659, -1659, -1659, -1659, -1659, -1659, -1659, -1659, -1659, -1659,
                        -1659, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532,
                        -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1532, -1404,
                        -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1404,
                        -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1404, -1276, -1276, -1276,
                        -1276, -1276, -1276, -1276, -1276, -1276, -1276, -1276, -1276, -1276, -1276,
                        -1276, -1276, -1276, -1276, -1276, -1276, -1149, -1149, -1149, -1149, -1149,
                        -1149, -1149, -1149, -1149, -1149, -1149, -1149, -1149, -1149, -1149, -1149,
                        -1149, -1149, -1149, -1149, -1021, -1021, -1021, -1021, -1021, -1021, -1021,
                        -1021, -1021, -1021, -1021, -1021, -1021, -1021, -1021, -1021, -1021, -1021,
                        -1021, -1021, -893, -893, -893, -893, -893, -893, -893, -893, -893, -893,
                        -893, -893, -893, -893, -893, -893, -893, -893, -893, -893, -808, -808, -808,
                        -808, -808, -808, -808, -808, -808, -808, -808, -808, -808, -808, -808, -808,
                        -808, -808, -808, -808, -723, -723, -723, -723, -723, -723, -723, -723, -723,
                        -723, -723, -723, -723, -723, -723, -723, -723, -723, -723, -723, -638, -638,
                        -638, -638, -638, -638, -638, -638, -638, -638, -638, -638, -638, -638, -638,
                        -638, -638, -638, -638, -638, -553, -553, -553, -553, -553, -553, -553, -553,
                        -553, -553, -553, -553, -553, -553, -553, -553, -553, -553, -553, -553, -468,
                        -468, -468, -468, -468, -468, -468, -468, -468, -468, -468, -468, -468, -468,
                        -468, -468, -468, -468, -468, -468, -383, -383, -383, -383, -383, -383, -383,
                        -383, -383, -383, -383, -383, -383, -383, -383, -383, -383, -383, -383, -383,
                        -297, -297, -297, -297, -297, -297, -297, -297, -297, -297, -297, -297, -297,
                        -297, -297, -297, -297, -297, -297, -297, -212, -212, -212, -212, -212, -212,
                        -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212,
                        -212];

                assert_eq!(
                    credits_per_epochs.clone().into_values().collect::<Vec<_>>(),
                    reference_fees
                );

                let total_distributed: SignedCredits = credits_per_epochs.values().sum();

                assert_eq!(total_distributed.to_unsigned(), refund_amount);
            }
        }

        #[test]
        fn should_deduct_refunds_from_two_collection_since_specific_epoch() {
            const CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED: EpochIndex = 42;
            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            // First_refund

            // Example: Bob inserted an element into the tree
            // He paid slightly more than 1.2 Million credits for this operation that happened at epoch 0.
            // At epoch 42 we are asking for a refund.
            // The refund is 1.07 Million credits that were left from the 1.2.

            let first_start_epoch_index: EpochIndex = GENESIS_EPOCH_INDEX;

            let first_original_storage_fee = 1200005;
            let (first_refund_amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(
                    first_original_storage_fee,
                    first_start_epoch_index,
                    CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED,
                )
                .expect("should distribute storage fee");

            assert_eq!(first_refund_amount, 1074120);
            assert_eq!(leftovers, 5);

            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                first_refund_amount,
                first_start_epoch_index,
                CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED,
            )
            .expect("should distribute storage fee");

            // Second_refund

            // Example: Bob inserted an element into the tree
            // He paid slightly more than 3.4 Million credits for this operation that happened at epoch 0.
            // At epoch 42 we are asking for a refund.

            const SECOND_START_EPOCH_INDEX: EpochIndex = 28;

            let second_original_storage_fee = 3405507;
            let (second_refund_amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(
                    second_original_storage_fee,
                    SECOND_START_EPOCH_INDEX,
                    CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED,
                )
                .expect("should distribute storage fee");

            assert_eq!(second_refund_amount, 3277305);
            assert_eq!(leftovers, 507);

            let multiplier = original_removed_credits_multiplier_from(
                SECOND_START_EPOCH_INDEX,
                CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED + 1,
            );

            // it's not going to be completely perfect but it's good enough
            // there were 24 epochs, on average we would be 12 off
            // while we could incorporate this offset into the multiplier it would
            // be overkill for such low credit amounts
            assert!(
                (Decimal::from(second_refund_amount) * multiplier)
                    .abs_sub(&Decimal::from(second_original_storage_fee - leftovers))
                    < dec!(100)
            );

            // we do however want to make sure the multiplier makes things smaller
            assert!(
                (Decimal::from(second_refund_amount) * multiplier)
                    < Decimal::from(second_original_storage_fee - leftovers)
            );

            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                second_refund_amount,
                SECOND_START_EPOCH_INDEX,
                CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED,
            )
            .expect("should distribute storage fee");
            // compare them with reference table
            // we expect to get 0 for the change of the current epochs balance
            // this is because there was only 1 refund so leftovers wouldn't have any effect
            #[rustfmt::skip]
                let reference_fees: [SignedCredits;
                (SECOND_START_EPOCH_INDEX + PERPETUAL_STORAGE_EPOCHS - CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED) as usize] =
                [-525, -11272, -11272, -11272, -11272, -11272, -10931, -10931, -10931, -10931,
                    -10931, -10931, -10931, -10931, -10931, -10931, -10931, -10931, -10811, -10811,
                    -10811, -10811, -10811, -10811, -10811, -10811, -10471, -10471, -10471, -10471,
                    -10471, -10471, -10471, -10471, -10471, -10471, -10471, -10471, -10351, -10351,
                    -10351, -10351, -10351, -10351, -10351, -10351, -10010, -10010, -10010, -10010,
                    -10010, -10010, -10010, -10010, -10010, -10010, -10010, -10010, -9890, -9890,
                    -9890, -9890, -9890, -9890, -9890, -9890, -9550, -9550, -9550, -9550, -9550,
                    -9550, -9550, -9550, -9550, -9550, -9550, -9550, -9460, -9460, -9460, -9460,
                    -9460, -9460, -9460, -9460, -9119, -9119, -9119, -9119, -9119, -9119, -9119,
                    -9119, -9119, -9119, -9119, -9119, -9029, -9029, -9029, -9029, -9029, -9029,
                    -9029, -9029, -8774, -8774, -8774, -8774, -8774, -8774, -8774, -8774, -8774,
                    -8774, -8774, -8774, -8684, -8684, -8684, -8684, -8684, -8684, -8684, -8684,
                    -8429, -8429, -8429, -8429, -8429, -8429, -8429, -8429, -8429, -8429, -8429,
                    -8429, -8339, -8339, -8339, -8339, -8339, -8339, -8339, -8339, -8083, -8083,
                    -8083, -8083, -8083, -8083, -8083, -8083, -8083, -8083, -8083, -8083, -7993,
                    -7993, -7993, -7993, -7993, -7993, -7993, -7993, -7738, -7738, -7738, -7738,
                    -7738, -7738, -7738, -7738, -7738, -7738, -7738, -7738, -7648, -7648, -7648,
                    -7648, -7648, -7648, -7648, -7648, -7393, -7393, -7393, -7393, -7393, -7393,
                    -7393, -7393, -7393, -7393, -7393, -7393, -7303, -7303, -7303, -7303, -7303,
                    -7303, -7303, -7303, -7047, -7047, -7047, -7047, -7047, -7047, -7047, -7047,
                    -7047, -7047, -7047, -7047, -6987, -6987, -6987, -6987, -6987, -6987, -6987,
                    -6987, -6732, -6732, -6732, -6732, -6732, -6732, -6732, -6732, -6732, -6732,
                    -6732, -6732, -6672, -6672, -6672, -6672, -6672, -6672, -6672, -6672, -6502,
                    -6502, -6502, -6502, -6502, -6502, -6502, -6502, -6502, -6502, -6502, -6502,
                    -6442, -6442, -6442, -6442, -6442, -6442, -6442, -6442, -6271, -6271, -6271,
                    -6271, -6271, -6271, -6271, -6271, -6271, -6271, -6271, -6271, -6211, -6211,
                    -6211, -6211, -6211, -6211, -6211, -6211, -6041, -6041, -6041, -6041, -6041,
                    -6041, -6041, -6041, -6041, -6041, -6041, -6041, -5981, -5981, -5981, -5981,
                    -5981, -5981, -5981, -5981, -5811, -5811, -5811, -5811, -5811, -5811, -5811,
                    -5811, -5811, -5811, -5811, -5811, -5751, -5751, -5751, -5751, -5751, -5751,
                    -5751, -5751, -5581, -5581, -5581, -5581, -5581, -5581, -5581, -5581, -5581,
                    -5581, -5581, -5581, -5521, -5521, -5521, -5521, -5521, -5521, -5521, -5521,
                    -5350, -5350, -5350, -5350, -5350, -5350, -5350, -5350, -5350, -5350, -5350,
                    -5350, -5290, -5290, -5290, -5290, -5290, -5290, -5290, -5290, -5120, -5120,
                    -5120, -5120, -5120, -5120, -5120, -5120, -5120, -5120, -5120, -5120, -5060,
                    -5060, -5060, -5060, -5060, -5060, -5060, -5060, -4890, -4890, -4890, -4890,
                    -4890, -4890, -4890, -4890, -4890, -4890, -4890, -4890, -4830, -4830, -4830,
                    -4830, -4830, -4830, -4830, -4830, -4660, -4660, -4660, -4660, -4660, -4660,
                    -4660, -4660, -4660, -4660, -4660, -4660, -4615, -4615, -4615, -4615, -4615,
                    -4615, -4615, -4615, -4444, -4444, -4444, -4444, -4444, -4444, -4444, -4444,
                    -4444, -4444, -4444, -4444, -4399, -4399, -4399, -4399, -4399, -4399, -4399,
                    -4399, -4272, -4272, -4272, -4272, -4272, -4272, -4272, -4272, -4272, -4272,
                    -4272, -4272, -4227, -4227, -4227, -4227, -4227, -4227, -4227, -4227, -4099,
                    -4099, -4099, -4099, -4099, -4099, -4099, -4099, -4099, -4099, -4099, -4099,
                    -4054, -4054, -4054, -4054, -4054, -4054, -4054, -4054, -3926, -3926, -3926,
                    -3926, -3926, -3926, -3926, -3926, -3926, -3926, -3926, -3926, -3881, -3881,
                    -3881, -3881, -3881, -3881, -3881, -3881, -3754, -3754, -3754, -3754, -3754,
                    -3754, -3754, -3754, -3754, -3754, -3754, -3754, -3709, -3709, -3709, -3709,
                    -3709, -3709, -3709, -3709, -3581, -3581, -3581, -3581, -3581, -3581, -3581,
                    -3581, -3581, -3581, -3581, -3581, -3536, -3536, -3536, -3536, -3536, -3536,
                    -3536, -3536, -3408, -3408, -3408, -3408, -3408, -3408, -3408, -3408, -3408,
                    -3408, -3408, -3408, -3363, -3363, -3363, -3363, -3363, -3363, -3363, -3363,
                    -3236, -3236, -3236, -3236, -3236, -3236, -3236, -3236, -3236, -3236, -3236,
                    -3236, -3191, -3191, -3191, -3191, -3191, -3191, -3191, -3191, -3063, -3063,
                    -3063, -3063, -3063, -3063, -3063, -3063, -3063, -3063, -3063, -3063, -3018,
                    -3018, -3018, -3018, -3018, -3018, -3018, -3018, -2890, -2890, -2890, -2890,
                    -2890, -2890, -2890, -2890, -2890, -2890, -2890, -2890, -2845, -2845, -2845,
                    -2845, -2845, -2845, -2845, -2845, -2717, -2717, -2717, -2717, -2717, -2717,
                    -2717, -2717, -2717, -2717, -2717, -2717, -2672, -2672, -2672, -2672, -2672,
                    -2672, -2672, -2672, -2545, -2545, -2545, -2545, -2545, -2545, -2545, -2545,
                    -2545, -2545, -2545, -2545, -2500, -2500, -2500, -2500, -2500, -2500, -2500,
                    -2500, -2372, -2372, -2372, -2372, -2372, -2372, -2372, -2372, -2372, -2372,
                    -2372, -2372, -2327, -2327, -2327, -2327, -2327, -2327, -2327, -2327, -2199,
                    -2199, -2199, -2199, -2199, -2199, -2199, -2199, -2199, -2199, -2199, -2199,
                    -2154, -2154, -2154, -2154, -2154, -2154, -2154, -2154, -2027, -2027, -2027,
                    -2027, -2027, -2027, -2027, -2027, -2027, -2027, -2027, -2027, -1982, -1982,
                    -1982, -1982, -1982, -1982, -1982, -1982, -1854, -1854, -1854, -1854, -1854,
                    -1854, -1854, -1854, -1854, -1854, -1854, -1854, -1809, -1809, -1809, -1809,
                    -1809, -1809, -1809, -1809, -1681, -1681, -1681, -1681, -1681, -1681, -1681,
                    -1681, -1681, -1681, -1681, -1681, -1636, -1636, -1636, -1636, -1636, -1636,
                    -1636, -1636, -1509, -1509, -1509, -1509, -1509, -1509, -1509, -1509, -1509,
                    -1509, -1509, -1509, -1464, -1464, -1464, -1464, -1464, -1464, -1464, -1464,
                    -1336, -1336, -1336, -1336, -1336, -1336, -1336, -1336, -1336, -1336, -1336,
                    -1336, -1306, -1306, -1306, -1306, -1306, -1306, -1306, -1306, -1178, -1178,
                    -1178, -1178, -1178, -1178, -1178, -1178, -1178, -1178, -1178, -1178, -1148,
                    -1148, -1148, -1148, -1148, -1148, -1148, -1148, -1063, -1063, -1063, -1063,
                    -1063, -1063, -1063, -1063, -1063, -1063, -1063, -1063, -1033, -1033, -1033,
                    -1033, -1033, -1033, -1033, -1033, -948, -948, -948, -948, -948, -948, -948,
                    -948, -948, -948, -948, -948, -918, -918, -918, -918, -918, -918, -918, -918,
                    -833, -833, -833, -833, -833, -833, -833, -833, -833, -833, -833, -833, -803,
                    -803, -803, -803, -803, -803, -803, -803, -718, -718, -718, -718, -718, -718,
                    -718, -718, -718, -718, -718, -718, -688, -688, -688, -688, -688, -688, -688,
                    -688, -603, -603, -603, -603, -603, -603, -603, -603, -603, -603, -603, -603,
                    -573, -573, -573, -573, -573, -573, -573, -573, -488, -488, -488, -488, -488,
                    -488, -488, -488, -488, -488, -488, -488, -458, -458, -458, -458, -458, -458,
                    -458, -458, -372, -372, -372, -372, -372, -372, -372, -372, -372, -372, -372,
                    -372, -297, -297, -297, -297, -297, -297, -297, -297, -212, -212, -212, -212,
                    -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212, -212,
                    -212, -212, -212];

            assert_eq!(
                credits_per_epochs.clone().into_values().collect::<Vec<_>>(),
                reference_fees
            );

            let total_distributed: SignedCredits = credits_per_epochs.values().sum();

            assert_eq!(
                total_distributed.to_unsigned(),
                first_refund_amount + second_refund_amount
            );
        }
    }

    mod calculate_storage_fee_refund_amount_and_leftovers {
        use super::*;

        #[test]
        fn should_calculate_amount_and_leftovers() {
            let storage_fee = 10000;

            let (amount, leftovers) = calculate_storage_fee_refund_amount_and_leftovers(
                storage_fee,
                GENESIS_EPOCH_INDEX + 1,
                2,
            )
            .expect("should distribute storage fee");

            let first_two_epochs_amount = 50;

            assert_eq!(leftovers, 400);
            assert_eq!(amount, storage_fee - leftovers - first_two_epochs_amount);
        }
    }
}
