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

/// Distributes storage fees to epochs into `SignedCreditsPerEpoch` and returns leftovers
/// It skips epochs up to specified `skip_up_to_epoch_index`
pub fn distribute_storage_fee_to_epochs_collection(
    credits_per_epochs: &mut SignedCreditsPerEpoch,
    storage_fee: SignedCredits,
    start_epoch_index: EpochIndex,
    skip_up_to_epoch_index: Option<EpochIndex>,
) -> Result<DistributionLeftovers, Error> {
    distribution_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        |epoch_index, epoch_fee_share| {
            if let Some(skip_epoch_index) = skip_up_to_epoch_index {
                if epoch_index < skip_epoch_index {
                    return Ok(());
                }
            }

            let epoch_credits: SignedCredits = credits_per_epochs
                .get(&epoch_index)
                .map_or(0, |i| i.to_owned());

            let result_storage_fee: SignedCredits = epoch_credits
                .checked_add(epoch_fee_share)
                .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

            credits_per_epochs.insert(epoch_index, result_storage_fee);

            Ok(())
        },
    )
}

type DistributionAmount = SignedCredits;
type DistributionLeftovers = SignedCredits;

/// Calculates leftovers and amount of credits by distributing storage fees to epochs
pub fn calculate_storage_fee_distribution_amount_and_leftovers(
    storage_fee: SignedCredits,
    start_epoch_index: EpochIndex,
    skip_up_to_epoch_index: EpochIndex,
) -> Result<(DistributionAmount, DistributionLeftovers), Error> {
    let mut skipped_amount = 0;

    let leftovers = distribution_storage_fee_to_epochs_map(
        storage_fee,
        start_epoch_index,
        |epoch_index, epoch_fee_share| {
            if epoch_index < skip_up_to_epoch_index {
                skipped_amount += epoch_fee_share;
            }

            Ok(())
        },
    )?;

    Ok((storage_fee - skipped_amount - leftovers, leftovers))
}

/// Distributes storage fees to epochs and call function for each epoch.
/// Returns leftovers
fn distribution_storage_fee_to_epochs_map<F>(
    storage_fee: SignedCredits,
    start_epoch_index: EpochIndex,
    mut f: F,
) -> Result<DistributionLeftovers, Error>
where
    F: FnMut(EpochIndex, SignedCredits) -> Result<(), Error>,
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

        let epoch_fee_share: SignedCredits = epoch_fee_share_dec
            .floor()
            .to_i64()
            .ok_or_else(|| get_overflow_error("storage fees are not fitting in a u64"))?;

        let year_start_epoch_index = start_epoch_index + EPOCHS_PER_YEAR * year;

        for epoch_index in year_start_epoch_index..year_start_epoch_index + EPOCHS_PER_YEAR {
            f(epoch_index, epoch_fee_share)?;

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
    use super::*;

    use crate::fee::credits::{Creditable, MAX};
    use crate::fee::epoch::GENESIS_EPOCH_INDEX;

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

    mod distribute_storage_fee_to_epochs_collection {
        use super::*;
        use crate::fee::epoch::PERPETUAL_STORAGE_EPOCHS;

        #[test]
        fn should_distribute_nothing_if_storage_fee_are_zero() {
            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                0,
                GENESIS_EPOCH_INDEX,
                None,
            )
            .expect("should distribute storage fee");

            assert_eq!(credits_per_epochs.len(), 0);
            assert_eq!(leftovers, 0);
        }

        #[test]
        fn should_distribute_max_credits_value_without_overflow() {
            let storage_fee = MAX.to_signed().expect("should convert signed credits");

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                GENESIS_EPOCH_INDEX,
                None,
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
                None,
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
                credits_per_epochs
                    .clone()
                    .into_values()
                    .collect::<Vec<SignedCredits>>(),
                reference_fees
            );

            let total_distributed: SignedCredits = credits_per_epochs.values().sum();

            assert_eq!(total_distributed + leftovers, storage_fee);

            /*

            Repeat distribution to ensure deterministic results

             */

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                current_epoch_index,
                None,
            )
            .expect("should distribute storage fee");

            // assert that all the values doubled meaning that distribution is reproducible
            assert_eq!(
                credits_per_epochs
                    .into_values()
                    .collect::<Vec<SignedCredits>>(),
                reference_fees
                    .into_iter()
                    .map(|val| val * 2)
                    .collect::<Vec<SignedCredits>>()
            );

            assert_eq!(leftovers, 180);
        }

        #[test]
        fn should_add_to_collection_from_specific_epoch() {
            let storage_fee: SignedCredits = 1000000;
            let start_epoch_index: EpochIndex = 0;

            const SKIP_UP_TO_EPOCH_INDEX: EpochIndex = 42;

            let mut credits_per_epochs = SignedCreditsPerEpoch::default();

            let leftovers = distribute_storage_fee_to_epochs_collection(
                &mut credits_per_epochs,
                storage_fee,
                start_epoch_index,
                Some(SKIP_UP_TO_EPOCH_INDEX),
            )
            .expect("should distribute storage fee");

            // check leftover
            assert_eq!(leftovers, 180);

            // compare them with reference table
            #[rustfmt::skip]
                let reference_fees: [SignedCredits; (PERPETUAL_STORAGE_EPOCHS - SKIP_UP_TO_EPOCH_INDEX) as usize] = [
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

            let skipped_reference_fees: [SignedCredits; SKIP_UP_TO_EPOCH_INDEX as usize] = [
                2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500, 2500,
                2500, 2500, 2500, 2500, 2500, 2500, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400,
                2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2400, 2300, 2300,
            ];

            assert_eq!(
                credits_per_epochs
                    .clone()
                    .into_values()
                    .collect::<Vec<SignedCredits>>(),
                reference_fees
            );

            let total_distributed: SignedCredits = credits_per_epochs.values().sum();

            assert_eq!(
                total_distributed
                    + leftovers
                    + skipped_reference_fees.into_iter().sum::<SignedCredits>(),
                storage_fee
            );
        }
    }

    mod calculate_storage_fee_to_epochs_distribution_amount_and_leftovers {
        use super::*;

        #[test]
        fn should_calculate_amount_and_leftovers() {
            let storage_fee = 1000;

            let (amount, leftovers) = calculate_storage_fee_distribution_amount_and_leftovers(
                storage_fee,
                GENESIS_EPOCH_INDEX,
                2,
            )
            .expect("should distribute storage fee");

            let first_two_epochs_amount = 50;

            assert_eq!(leftovers, 400);
            assert_eq!(amount, storage_fee - leftovers - first_two_epochs_amount);
        }
    }
}
