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
//! Data is stored in Platform "forever" currently, which is 50 eras (50 years by default).
//! To incentivise masternodes to continue store and serve this data,
//! payments are distributed for entire period split into epochs.
//! Every epoch, new aggregated storage fees are distributed among epochs
//! and masternodes receive payouts for previous epoch.
//!

use crate::fee::epoch::{EpochIndex, SignedCreditsPerEpoch, PERPETUAL_STORAGE_ERAS};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::cmp::Ordering;

use crate::balances::credits::Credits;
use crate::ProtocolError;
use std::ops::Mul;

// TODO: Should be updated from the doc

/// The amount of the perpetual storage fee to be paid out to masternodes per era. Adds up to 1.
#[rustfmt::skip]
pub const FEE_DISTRIBUTION_TABLE: [Decimal; PERPETUAL_STORAGE_ERAS as usize] = [
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
    epochs_per_era: u16,
) -> Result<DistributionLeftovers, ProtocolError> {
    distribution_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        |epoch_index, epoch_fee_share| {
            let epoch_credits = credits_per_epochs.entry(epoch_index).or_default();

            *epoch_credits = epoch_credits
                .checked_add_unsigned(epoch_fee_share)
                .ok_or_else(|| {
                    ProtocolError::Overflow(
                        "updated epoch credits are not fitting to credits max size",
                    )
                })?;

            Ok(())
        },
        epochs_per_era,
    )
}

/// Distributes refunds to epochs into `SignedCreditsPerEpoch` and returns leftovers
/// It skips epochs up to specified `skip_until_epoch_index`
pub fn subtract_refunds_from_epoch_credits_collection(
    credits_per_epochs: &mut SignedCreditsPerEpoch,
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    current_epoch_index: EpochIndex,
    epochs_per_era: u16,
) -> Result<(), ProtocolError> {
    let leftovers = refund_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        current_epoch_index + 1,
        |epoch_index, epoch_fee_share| {
            let epoch_credits = credits_per_epochs.entry(epoch_index).or_default();

            *epoch_credits = epoch_credits
                .checked_sub_unsigned(epoch_fee_share)
                .ok_or_else(|| {
                    ProtocolError::Overflow(
                        "updated epoch credits are not fitting to credits min size",
                    )
                })?;

            Ok(())
        },
        epochs_per_era,
    )?;

    // We need to remove the leftovers from the current epoch
    if leftovers > 0 {
        let epoch_credits = credits_per_epochs.entry(current_epoch_index).or_default();

        *epoch_credits = epoch_credits
            .checked_sub_unsigned(leftovers)
            .ok_or_else(|| {
                ProtocolError::Overflow("updated epoch credits are not fitting to credits min size")
            })?;
    }

    Ok(())
}

/// Calculates leftovers and amount of credits by distributing storage fees to epochs
pub fn calculate_storage_fee_refund_amount_and_leftovers(
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    current_epoch_index: EpochIndex,
    epochs_per_era: u16,
) -> Result<(DistributionAmount, DistributionLeftovers), ProtocolError> {
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
        epochs_per_era,
    )?;

    Ok((storage_fee - skipped_amount - leftovers, leftovers))
}

fn original_removed_credits_multiplier_from(
    start_epoch_index: EpochIndex,
    start_repayment_from_epoch_index: EpochIndex,
    epochs_per_era: u16,
) -> Result<Decimal, ProtocolError> {
    // `start_repayment_from_epoch_index` is `current_epoch_index + 1` and
    // `start_epoch_index` is the (earlier) epoch the storage was originally
    // paid in, so this subtraction normally cannot underflow. Guard it anyway
    // so corrupted/unexpected inputs return an error rather than panicking
    // (debug) or wrapping (release) on the consensus path.
    let paid_epochs = start_repayment_from_epoch_index
        .checked_sub(start_epoch_index)
        .ok_or(ProtocolError::Overflow(
            "start repayment epoch is before the original storage epoch",
        ))?;

    let current_era = (paid_epochs / epochs_per_era) as usize;

    let ratio_used: Decimal =
        FEE_DISTRIBUTION_TABLE
            .iter()
            .enumerate()
            .filter_map(|(era, epoch_multiplier)| match era.cmp(&current_era) {
                Ordering::Less => None,
                Ordering::Equal => {
                    let amount_epochs_left_in_era = epochs_per_era - paid_epochs % epochs_per_era;
                    Some(epoch_multiplier.mul(
                        Decimal::from(amount_epochs_left_in_era) / Decimal::from(epochs_per_era),
                    ))
                }
                Ordering::Greater => Some(*epoch_multiplier),
            })
            .sum();

    // `FEE_DISTRIBUTION_TABLE` has exactly `PERPETUAL_STORAGE_ERAS` entries.
    // Once the refund's original storage epoch is at least that whole window
    // behind the repayment epoch (`current_era >= PERPETUAL_STORAGE_ERAS`),
    // every table era compares `Ordering::Less`, the iterator yields nothing,
    // and `ratio_used` sums to zero. `rust_decimal::Decimal`'s `/` operator
    // PANICS on a zero divisor (unlike integer/`f64` division and unlike its
    // own `checked_div`), which on the consensus path would abort every node
    // simultaneously and halt the chain. Return a propagable error instead.
    if ratio_used.is_zero() {
        return Err(ProtocolError::DivideByZero(
            "storage fee refund is older than the entire perpetual storage window",
        ));
    }

    Ok(dec!(1) / ratio_used)
}

/// Let's imagine that we are refunding something from epoch 5
/// We are at Epoch 12
/// The refund amount is from Epoch 13 (current + 1) to Epoch 1005 (5 + 1000)
/// We need to figure out the amount extra those 8 costed
fn restore_original_removed_credits_amount(
    refund_amount: Decimal,
    start_epoch_index: EpochIndex,
    start_repayment_from_epoch_index: EpochIndex,
    epochs_per_era: u16,
) -> Result<Decimal, ProtocolError> {
    let multiplier = original_removed_credits_multiplier_from(
        start_epoch_index,
        start_repayment_from_epoch_index,
        epochs_per_era,
    )?;

    refund_amount
        .checked_mul(multiplier)
        .ok_or(ProtocolError::Overflow(
            "overflow when multiplying with the multiplier (this should be impossible)",
        ))
}

/// Distributes storage fees to epochs and call function for each epoch.
/// Returns leftovers
fn distribution_storage_fee_to_epochs_map<F>(
    storage_fee: Credits,
    start_epoch_index: EpochIndex,
    mut map_function: F,
    epochs_per_era: u16,
) -> Result<DistributionLeftovers, ProtocolError>
where
    F: FnMut(EpochIndex, Credits) -> Result<(), ProtocolError>,
{
    if storage_fee == 0 {
        return Ok(0);
    }

    let storage_fee_dec: Decimal = storage_fee.into();

    let mut distribution_leftover_credits = storage_fee;

    let epochs_per_era_dec = Decimal::from(epochs_per_era);

    for era in 0..PERPETUAL_STORAGE_ERAS {
        let distribution_for_that_era_ratio = FEE_DISTRIBUTION_TABLE[era as usize];

        let era_fee_share = storage_fee_dec * distribution_for_that_era_ratio;

        let epoch_fee_share_dec = era_fee_share / epochs_per_era_dec;

        let epoch_fee_share: Credits = epoch_fee_share_dec
            .floor()
            .to_u64()
            .ok_or_else(|| ProtocolError::Overflow("storage fees are not fitting in a u64"))?;

        let era_start_epoch_index = start_epoch_index + epochs_per_era * era;

        for epoch_index in era_start_epoch_index..era_start_epoch_index + epochs_per_era {
            //todo: this can lead to many many calls once we are further along in epochs
            map_function(epoch_index, epoch_fee_share)?;

            distribution_leftover_credits = distribution_leftover_credits
                .checked_sub(epoch_fee_share)
                .ok_or(ProtocolError::Overflow(
                    "leftovers bigger than initial value",
                ))?;
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
    epochs_per_era: u16,
) -> Result<DistributionLeftovers, ProtocolError>
where
    F: FnMut(EpochIndex, Credits) -> Result<(), ProtocolError>,
{
    if storage_fee == 0 {
        return Ok(0);
    }

    let storage_fee_dec: Decimal = storage_fee.into();

    let mut distribution_leftover_credits = storage_fee;

    let epochs_per_era_dec = Decimal::from(epochs_per_era);

    let start_era: u16 = (skip_until_epoch_index - start_epoch_index) / epochs_per_era;

    // The perpetual storage window for this data ends `PERPETUAL_STORAGE_ERAS`
    // eras after `start_epoch_index`. Once the distribution epoch reaches or
    // passes that end (`start_era >= PERPETUAL_STORAGE_ERAS`) there are no future
    // epoch pools left to claw the refund back from: the per-era loop below is
    // empty, and `original_removed_credits_multiplier_from` returns
    // `DivideByZero` (`ratio_used == 0`) computing a multiplier that is never
    // used. Returning that error here still halts the chain — it propagates up
    // the consensus path to a Tenderdash `ResponseException`. Instead treat the
    // whole refund as leftovers so the caller removes it from the current
    // epoch's pool, keeping the chain live.
    //
    // This is reachable on the consensus path: the refund amount is computed at
    // the removal epoch (`FeeRefunds::from_storage_removal`) but this clawback
    // runs at least one epoch later, at the next epoch change, against the
    // then-current epoch. So a non-zero refund for data removed just before
    // expiry can be distributed just after the window boundary is crossed.
    if start_era >= PERPETUAL_STORAGE_ERAS {
        return Ok(storage_fee);
    }

    // Let's imagine that we are refunding something from epoch 5
    // We are at Epoch 12
    // The refund amount is from Epoch 13 (current + 1) to Epoch 1005 (5 + 1000)
    // We need to figure out the amount extra those 8 costed
    let estimated_storage_fee_dec = restore_original_removed_credits_amount(
        storage_fee_dec,
        start_epoch_index,
        skip_until_epoch_index,
        epochs_per_era,
    )?;

    for era in start_era..PERPETUAL_STORAGE_ERAS {
        let distribution_for_that_era_ratio = FEE_DISTRIBUTION_TABLE[era as usize];

        let estimated_era_fee_share = estimated_storage_fee_dec * distribution_for_that_era_ratio;

        let estimated_epoch_fee_share_dec = estimated_era_fee_share / epochs_per_era_dec;

        let estimated_epoch_fee_share: Credits = estimated_epoch_fee_share_dec
            .floor()
            .to_u64()
            .ok_or_else(|| ProtocolError::Overflow("storage fees are not fitting in a u64"))?;

        let era_start_epoch_index = if era == start_era {
            skip_until_epoch_index
        } else {
            start_epoch_index + epochs_per_era * era
        };

        let era_end_epoch_index = start_epoch_index + ((era + 1) * epochs_per_era);

        for epoch_index in era_start_epoch_index..era_end_epoch_index {
            map_function(epoch_index, estimated_epoch_fee_share)?;

            distribution_leftover_credits = distribution_leftover_credits
                .checked_sub(estimated_epoch_fee_share)
                .ok_or(ProtocolError::Overflow(
                    "leftovers bigger than initial value",
                ))?;
        }
    }
    Ok(distribution_leftover_credits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fee::epoch::GENESIS_EPOCH_INDEX;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    mod original_removed_credits_multiplier_from {
        use super::*;

        #[test]
        fn should_create_multiplier_for_epochs_since_the_beginning() {
            // the multiplier should be
            let epoch_0_cost = dec!(0.05000) / dec!(20.0);
            let multiplier_should_be = dec!(1.0) / (dec!(1.0) - epoch_0_cost);

            let multiplier = original_removed_credits_multiplier_from(0, 1, 20)
                .expect("multiplier within perpetual storage window");

            assert_eq!(multiplier_should_be, multiplier);
        }

        #[test]
        fn should_create_multiplier_for_epochs_since_24_and_repaid_since_43() {
            // there were 19 epochs
            let epoch_0_cost = dec!(19.0) * dec!(0.05000) / dec!(20.0);

            let multiplier_should_be = dec!(1.0) / (dec!(1.0) - epoch_0_cost);

            let multiplier = original_removed_credits_multiplier_from(24, 43, 20)
                .expect("multiplier within perpetual storage window");

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

            let leftovers = distribution_storage_fee_to_epochs_map(
                0,
                GENESIS_EPOCH_INDEX,
                |_, _| {
                    calls += 1;

                    Ok(())
                },
                20,
            )
            .expect("should distribute storage fee");

            assert_eq!(calls, 0);
            assert_eq!(leftovers, 0);
        }

        #[test]
        fn should_call_function_for_each_epoch_for_50_eras_sequentially() {
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
                20,
            )
            .expect("should distribute storage fee");

            assert_eq!(calls, 1000); //20*50
            assert_eq!(leftovers, 360);
        }
    }

    mod distribute_storage_fee_to_epochs_collection {
        use super::*;
        use crate::balances::credits::{Creditable, MAX_CREDITS};
        use crate::fee::SignedCredits;

        #[test]
        fn should_distribute_max_credits_value_without_overflow() {
            let storage_fee = MAX_CREDITS;

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                GENESIS_EPOCH_INDEX,
                20,
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
                20,
            )
            .expect("should distribute storage fee");

            // check leftover
            assert_eq!(leftovers, 180);

            // compare them with reference table for 20 epochs per era (1000)
            #[rustfmt::skip]
                let reference_fees: [SignedCredits; 1000] = [
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
                20,
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
        use crate::balances::credits::Creditable;
        use crate::fee::SignedCredits;

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
                20,
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
                20,
            )
            .expect("should distribute storage fee");

            // compare them with reference table
            // we expect to get 0 for the change of the current epochs balance
            // this is because there was only 1 refund so leftovers wouldn't have any effect
            #[rustfmt::skip]
            let reference_fees: [SignedCredits;
                (1000 - REFUNDED_EPOCH_INDEX - 1) as usize] = [-2760, -2760, -2760,
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

        /// Regression test for the decoupled-epoch storage-fee refund
        /// divide-by-zero (chain-halt) bug.
        ///
        /// In production the refund AMOUNT and the clawback DISTRIBUTION are
        /// computed at two different epochs:
        ///   * `FeeRefunds::from_storage_removal` computes the amount at the
        ///     removal epoch (`c_store`) and persists it keyed by the original
        ///     write epoch — the removal epoch is then discarded.
        ///   * one epoch later, at the next epoch change,
        ///     `add_distribute_storage_fee_to_epochs_operations_v0` consumes it
        ///     via `subtract_refunds_from_epoch_credits_collection` using the
        ///     *new* current epoch (`c_consume = c_store + 1`), which re-derives
        ///     the `1 / ratio_used` multiplier against a different window
        ///     position.
        ///
        /// Every other test in this module feeds the SAME epoch to both calls
        /// (the self-consistent case), which is exactly why this was never
        /// exercised. Here we reproduce the real wiring: data written at epoch
        /// 0 is removed two epochs before its 50-era window expires — so the
        /// refund is legitimately non-zero (it is the share of the single
        /// remaining in-window epoch) — and then distributed one epoch later,
        /// precisely as the window boundary is crossed. At that point
        /// `current_era == PERPETUAL_STORAGE_ERAS`, so `ratio_used == 0`.
        /// Without the `start_era >= PERPETUAL_STORAGE_ERAS` guard in
        /// `refund_storage_fee_to_epochs_map`, this returns `DivideByZero` (or,
        /// before that error existed, panicked) and halts every node.
        #[test]
        fn should_not_halt_when_refund_distributed_as_window_boundary_is_crossed() {
            const EPOCHS_PER_ERA: u16 = 40; // production default
            const WRITE_EPOCH: EpochIndex = 0;

            // The perpetual storage window for data written at WRITE_EPOCH spans
            // epochs [0, EPOCHS_PER_ERA * PERPETUAL_STORAGE_ERAS) = [0, 2000).
            let window_end_epoch = WRITE_EPOCH + EPOCHS_PER_ERA * PERPETUAL_STORAGE_ERAS; // 2000

            // Remove the data two epochs before the window fully elapses, so the
            // refund still covers exactly one in-window epoch (the final one,
            // 1999) and is therefore NON-ZERO.
            let removal_epoch = window_end_epoch - 2; // c_store = 1998

            // One epoch change later the refund is distributed with the THEN
            // current epoch index (the natural +1-epoch lag). This value flows
            // into original_removed_credits_multiplier_from via skip_until =
            // current + 1, which now spans the full window.
            let distribution_epoch = removal_epoch + 1; // c_consume = 1999

            let original_storage_fee: Credits = 10_000_000_000;

            let (refund_amount, _leftovers) = calculate_storage_fee_refund_amount_and_leftovers(
                original_storage_fee,
                WRITE_EPOCH,
                removal_epoch,
                EPOCHS_PER_ERA,
            )
            .expect("refund amount computation should succeed");

            assert!(
                refund_amount > 0,
                "removing data before the final in-window epoch ({}) must leave a \
                 non-zero refund; got {refund_amount}",
                window_end_epoch - 1,
            );

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            // Without the guard this returns Err(DivideByZero) (which still
            // halts the chain via Tenderdash) — or panicked before that error
            // existed — inside original_removed_credits_multiplier_from, because
            // the multiplier is recomputed at the distribution epoch and now
            // spans the entire perpetual-storage window.
            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                refund_amount,
                WRITE_EPOCH,
                distribution_epoch,
                EPOCHS_PER_ERA,
            )
            .expect("refund distribution must not halt the chain at the window boundary");

            // With the window fully elapsed there are no future epoch pools left
            // to claw the refund back from, so the entire refund must come out of
            // the current (distribution) epoch's pool — and nothing else should
            // be touched.
            let entries: Vec<(EpochIndex, SignedCredits)> =
                credits_per_epochs.into_iter().collect();
            assert_eq!(
                entries,
                vec![(distribution_epoch, -(refund_amount as SignedCredits))],
                "the full refund should be clawed back from only the current epoch",
            );
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
                    20,
                )
                .expect("should distribute storage fee");

                assert_eq!(refund_amount, 3277305);
                assert_eq!(leftovers, 507);

                let multiplier = original_removed_credits_multiplier_from(
                    start_epoch_index,
                    current_epoch_index_where_refund_occurred + 1,
                    20,
                )
                .expect("multiplier within perpetual storage window");

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
                    20,
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
                    20,
                )
                .expect("should distribute storage fee");

            assert_eq!(first_refund_amount, 1074120);
            assert_eq!(leftovers, 5);

            subtract_refunds_from_epoch_credits_collection(
                &mut credits_per_epochs,
                first_refund_amount,
                first_start_epoch_index,
                CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED,
                20,
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
                    20,
                )
                .expect("should distribute storage fee");

            assert_eq!(second_refund_amount, 3277305);
            assert_eq!(leftovers, 507);

            let multiplier = original_removed_credits_multiplier_from(
                SECOND_START_EPOCH_INDEX,
                CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED + 1,
                20,
            )
            .expect("multiplier within perpetual storage window");

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
                20,
            )
            .expect("should distribute storage fee");
            // compare them with reference table
            // we expect to get 0 for the change of the current epochs balance
            // this is because there was only 1 refund so leftovers wouldn't have any effect
            #[rustfmt::skip]
                let reference_fees: [SignedCredits;
                (SECOND_START_EPOCH_INDEX + 1000 - CURRENT_EPOCH_INDEX_WHERE_REFUND_OCCURRED) as usize] =
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
                20,
            )
            .expect("should distribute storage fee");

            let first_two_epochs_amount = 50;

            assert_eq!(leftovers, 400);
            assert_eq!(amount, storage_fee - leftovers - first_two_epochs_amount);
        }

        #[test]
        fn should_return_zero_amount_and_zero_leftovers_for_zero_storage_fee() {
            let (amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(0, GENESIS_EPOCH_INDEX, 10, 20)
                    .expect("should handle zero storage fee");

            assert_eq!(amount, 0);
            assert_eq!(leftovers, 0);
        }

        #[test]
        fn should_return_zero_refund_when_start_epoch_equals_current_epoch() {
            // When start == current, skipped_amount covers epoch 0 only (the one epoch
            // between start_epoch_index and current_epoch_index + 1 = 1).
            let storage_fee = 1000000;
            let epoch = 0;

            let (amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(storage_fee, epoch, epoch, 20)
                    .expect("should distribute storage fee");

            // Only epoch 0 is skipped (cost = floor(1000000 * 0.05 / 20) = 2500).
            // The refund amount is everything except the skipped epoch and leftovers.
            assert_eq!(amount, storage_fee - 2500 - leftovers);
        }

        #[test]
        fn should_calculate_correctly_with_non_genesis_start() {
            let storage_fee = 500000;
            let start = 100;
            let current = 110;

            let (amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(storage_fee, start, current, 20)
                    .expect("should distribute storage fee");

            // Verify invariant: amount + skipped + leftovers = storage_fee
            assert_eq!(
                amount + (storage_fee - amount - leftovers) + leftovers,
                storage_fee
            );
            // Amount must be less than total
            assert!(amount < storage_fee);
            assert!(leftovers < storage_fee);
        }

        #[test]
        fn should_handle_large_epoch_gap() {
            // current_epoch far from start
            let storage_fee = 10_000_000;
            let start = 0;
            let current = 500; // halfway through the 1000 total epochs

            let (amount, leftovers) =
                calculate_storage_fee_refund_amount_and_leftovers(storage_fee, start, current, 20)
                    .expect("should handle large epoch gap");

            // Refund amount should be smaller because most epochs have been paid out
            assert!(amount < storage_fee / 2);
            assert!(leftovers < storage_fee);
        }
    }

    mod additional_original_removed_credits_multiplier_from {
        use super::*;

        #[test]
        fn should_create_multiplier_of_one_when_no_epochs_have_passed() {
            // When start_repayment == start, paid_epochs = 0, ratio_used = full table sum = 1.0
            // So multiplier = 1/1 = 1
            let multiplier = original_removed_credits_multiplier_from(0, 0, 20)
                .expect("multiplier within perpetual storage window");
            assert_eq!(multiplier, dec!(1));
        }

        #[test]
        fn should_increase_multiplier_as_more_epochs_pass() {
            let m1 = original_removed_credits_multiplier_from(0, 5, 20)
                .expect("multiplier within perpetual storage window");
            let m2 = original_removed_credits_multiplier_from(0, 10, 20)
                .expect("multiplier within perpetual storage window");
            let m3 = original_removed_credits_multiplier_from(0, 19, 20)
                .expect("multiplier within perpetual storage window");

            // More paid epochs means less ratio remaining, so multiplier increases
            assert!(m1 < m2);
            assert!(m2 < m3);
        }

        #[test]
        fn should_handle_era_boundary_crossing() {
            // paid_epochs = 20 means we enter the second era exactly
            let m_at_boundary = original_removed_credits_multiplier_from(0, 20, 20)
                .expect("multiplier within perpetual storage window");
            let m_before_boundary = original_removed_credits_multiplier_from(0, 19, 20)
                .expect("multiplier within perpetual storage window");
            let m_after_boundary = original_removed_credits_multiplier_from(0, 21, 20)
                .expect("multiplier within perpetual storage window");

            // At the boundary, the entire first era (0.05) is consumed
            assert!(m_at_boundary > m_before_boundary);
            assert!(m_after_boundary > m_at_boundary);
        }

        #[test]
        fn should_handle_different_epochs_per_era() {
            // With 40 epochs per era (the default), 40 paid epochs = 1 full era
            let m_40 = original_removed_credits_multiplier_from(0, 40, 40)
                .expect("multiplier within perpetual storage window");
            // With 20 epochs per era, 20 paid epochs = 1 full era
            let m_20 = original_removed_credits_multiplier_from(0, 20, 20)
                .expect("multiplier within perpetual storage window");

            // Both consume exactly one full era of 0.05, so multipliers should be equal
            assert_eq!(m_40, m_20);
        }

        #[test]
        fn should_produce_same_multiplier_regardless_of_absolute_epoch_offset() {
            // The multiplier depends only on the difference, not absolute indices
            let m1 = original_removed_credits_multiplier_from(0, 15, 20)
                .expect("multiplier within perpetual storage window");
            let m2 = original_removed_credits_multiplier_from(100, 115, 20)
                .expect("multiplier within perpetual storage window");
            let m3 = original_removed_credits_multiplier_from(5000, 5015, 20)
                .expect("multiplier within perpetual storage window");

            assert_eq!(m1, m2);
            assert_eq!(m2, m3);
        }

        #[test]
        fn should_return_error_instead_of_panicking_when_window_fully_elapsed() {
            // PERPETUAL_STORAGE_ERAS == 50 and the distribution table has 50
            // entries. With epochs_per_era = 20, `current_era` reaches 50 (the
            // full window) at paid_epochs = 50 * 20 = 1000, at which point every
            // table era compares `Less`, `ratio_used` sums to zero, and the old
            // code panicked on `dec!(1) / 0`. It must now return a DivideByZero
            // error so the consensus path can propagate it instead of aborting.
            let result = original_removed_credits_multiplier_from(0, 1000, 20);
            assert!(
                matches!(result, Err(ProtocolError::DivideByZero(_))),
                "expected DivideByZero error once the window is fully elapsed, got {:?}",
                result
            );

            // The caller propagates the error rather than panicking.
            let restored = restore_original_removed_credits_amount(dec!(1_000_000), 0, 1000, 20);
            assert!(
                matches!(restored, Err(ProtocolError::DivideByZero(_))),
                "restore_original_removed_credits_amount must propagate the error, got {:?}",
                restored
            );
        }

        #[test]
        fn should_return_error_when_repayment_epoch_precedes_start() {
            // Defensive guard: a repayment epoch before the original storage
            // epoch must not underflow the `paid_epochs` subtraction.
            let result = original_removed_credits_multiplier_from(10, 5, 20);
            assert!(
                matches!(result, Err(ProtocolError::Overflow(_))),
                "expected Overflow error on underflowing epoch difference, got {:?}",
                result
            );
        }
    }

    mod additional_restore_original_removed_credits_amount {
        use super::*;

        #[test]
        fn should_restore_to_original_when_no_epochs_passed() {
            // If start_repayment == start, multiplier is 1.0, so restored == refund_amount
            let refund = dec!(1000000);
            let restored = restore_original_removed_credits_amount(refund, 0, 0, 20)
                .expect("should not overflow");
            assert_eq!(restored, refund);
        }

        #[test]
        fn should_increase_amount_when_epochs_have_passed() {
            // After some epochs, the multiplier > 1, so restored > refund
            let refund = dec!(500000);
            let restored = restore_original_removed_credits_amount(refund, 0, 10, 20)
                .expect("should not overflow");
            assert!(restored > refund);
        }

        #[test]
        fn should_handle_zero_refund_amount() {
            let restored = restore_original_removed_credits_amount(dec!(0), 0, 10, 20)
                .expect("should handle zero");
            assert_eq!(restored, dec!(0));
        }
    }

    mod additional_refund_storage_fee_to_epochs_map {
        use super::*;

        #[test]
        fn should_return_zero_leftovers_for_zero_storage_fee() {
            let leftovers = refund_storage_fee_to_epochs_map(0, 0, 1, |_, _| Ok(()), 20)
                .expect("should handle zero");
            assert_eq!(leftovers, 0);
        }

        #[test]
        fn should_skip_epochs_before_skip_until_index() {
            let storage_fee = 1000000u64;
            let start = 0u16;
            let skip_until = 10u16;

            let mut min_epoch_seen = u16::MAX;

            let _leftovers = refund_storage_fee_to_epochs_map(
                storage_fee,
                start,
                skip_until,
                |epoch_index, _amount| {
                    if epoch_index < min_epoch_seen {
                        min_epoch_seen = epoch_index;
                    }
                    Ok(())
                },
                20,
            )
            .expect("should distribute refund");

            // The first epoch called should be >= skip_until
            assert!(min_epoch_seen >= skip_until);
        }

        #[test]
        fn should_distribute_to_single_remaining_epoch_in_era() {
            // skip_until is 19 (last epoch of era 0), start is 0
            // This means only 1 epoch remains in era 0
            let storage_fee = 100000u64;

            let mut epoch_count = 0u32;

            let leftovers = refund_storage_fee_to_epochs_map(
                storage_fee,
                0,
                19,
                |_epoch_index, _amount| {
                    epoch_count += 1;
                    Ok(())
                },
                20,
            )
            .expect("should distribute");

            // Total epochs = (1000 - 19) = 981 epochs should be called
            assert_eq!(epoch_count, 981);
            assert!(leftovers < storage_fee);
        }

        #[test]
        fn should_handle_skip_at_era_boundary() {
            // skip_until exactly at era 1 start
            let storage_fee = 500000u64;
            let start = 0u16;
            let skip_until = 20u16; // era 1 starts here

            let mut epochs_called = Vec::new();

            let _leftovers = refund_storage_fee_to_epochs_map(
                storage_fee,
                start,
                skip_until,
                |epoch_index, _amount| {
                    epochs_called.push(epoch_index);
                    Ok(())
                },
                20,
            )
            .expect("should distribute");

            // First epoch called should be exactly skip_until
            assert_eq!(*epochs_called.first().unwrap(), skip_until);
            // Total = 1000 - 20 = 980
            assert_eq!(epochs_called.len(), 980);
        }
    }
}
