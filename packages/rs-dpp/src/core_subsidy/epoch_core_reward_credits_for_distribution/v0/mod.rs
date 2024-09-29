use crate::block::epoch::EpochIndex;
use crate::fee::Credits;

use crate::core_subsidy::CORE_GENESIS_BLOCK_SUBSIDY;
use crate::ProtocolError;
use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    /// The Core reward halving distribution table for 100 years
    /// Yearly decline of production by ~7.1% per year, projected ~18M coins max by year 2050+.
    pub static ref CORE_HALVING_DISTRIBUTION: HashMap<u16, Credits> = {
        let mut distribution = CORE_GENESIS_BLOCK_SUBSIDY;
        (0..200).map(|i| {
            let old_distribution = distribution;
            distribution -= distribution / 14;
            (i, old_distribution)
        }).collect()
    };
}

/// Gets the amount of core reward fees to be distributed for the Epoch.
pub(super) fn epoch_core_reward_credits_for_distribution_v0(
    from_start_block_core_height: u32,
    to_end_block_core_height_included: u32,
    core_subsidy_halving_interval: u32,
) -> Result<Credits, ProtocolError> {
    if from_start_block_core_height > to_end_block_core_height_included {
        return Ok(0);
    }
    // Core is halving block rewards every year so we need to pay
    // core block rewards according to halving ratio for the all years during
    // the platform epoch payout period (unpaid epoch)

    // In Core there is an off by 1 compared to what we would expect, for if the halving interval is 1000
    // We would see a new reward year on block 1001.

    let previous_from_start_block_core_height = from_start_block_core_height.saturating_sub(1);

    let previous_to_end_block_core_height = to_end_block_core_height_included.saturating_sub(1);

    // Calculate start and end years for the platform epoch payout period
    // according to start and end core block heights

    // 1000 would be on core year 0, as we would have 1000 - 1/1000 => 0, this is correct
    let start_core_reward_year =
        (previous_from_start_block_core_height / core_subsidy_halving_interval) as EpochIndex;
    // 2000 would be on core year 1, as we would have 2000 - 1/1000 => 1, this is correct
    let end_core_reward_year =
        (previous_to_end_block_core_height / core_subsidy_halving_interval) as EpochIndex;

    let mut total_core_rewards = 0;

    // Calculate block rewards for each core reward year during the platform epoch payout period
    for core_reward_epoch in start_core_reward_year..=end_core_reward_year {
        // Calculate the block count per core reward year

        // If we are on the end core reward year
        // We use the previous_from_start_block_core_height
        // For example if we are calculating 2000 to 2001
        // We should have one block on start core year and one block on end core year
        let core_reward_year_start_block = if core_reward_epoch == start_core_reward_year {
            previous_from_start_block_core_height
        } else {
            core_reward_epoch as u32 * core_subsidy_halving_interval
        };

        let core_reward_year_end_block = if core_reward_epoch == end_core_reward_year {
            to_end_block_core_height_included
        } else {
            (core_reward_epoch + 1) as u32 * core_subsidy_halving_interval
        };

        let block_count = core_reward_year_end_block - core_reward_year_start_block;

        // Fetch the core block distribution for the corresponding epoch from the distribution table
        // Default to 0 if the core reward year is more than 100 years in the future
        let core_block_distribution_ratio = CORE_HALVING_DISTRIBUTION
            .get(&core_reward_epoch)
            .ok_or(ProtocolError::NotSupported(format!(
                "having distribution not supported for core reward epoch {}",
                core_reward_epoch
            )))?;

        // Calculate the core rewards for this epoch and add to the total
        total_core_rewards += block_count as Credits * *core_block_distribution_ratio;
    }

    Ok(total_core_rewards)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_simple_case_at_first_core_epoch() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 1;
        let to_end_block_core_height_included = 150;
        let expected_reward = CORE_GENESIS_BLOCK_SUBSIDY * 150; // Since all blocks are in the first epoch

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, expected_reward);
    }

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_simple_case_at_eighth_core_epoch() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 1201;
        let to_end_block_core_height_included = 1350;
        let expected_reward = CORE_HALVING_DISTRIBUTION[&8] * 150; // 1200 / 150 = 8

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, expected_reward);
    }

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_across_two_epochs() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 149;
        let to_end_block_core_height_included = 151;
        let halved_subsidy = CORE_GENESIS_BLOCK_SUBSIDY - CORE_GENESIS_BLOCK_SUBSIDY / 14;
        let expected_reward = (CORE_GENESIS_BLOCK_SUBSIDY * 2) + halved_subsidy;

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, expected_reward);
    }

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_across_three_epochs() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 149;
        let to_end_block_core_height_included = 301;
        let halved_subsidy = CORE_GENESIS_BLOCK_SUBSIDY - CORE_GENESIS_BLOCK_SUBSIDY / 14;
        let next_halved_subsidy = halved_subsidy - halved_subsidy / 14;
        let expected_reward =
            (CORE_GENESIS_BLOCK_SUBSIDY * 2) + halved_subsidy * 150 + next_halved_subsidy;

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, expected_reward);
    }

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_inner_epoch() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 1303;
        let to_end_block_core_height_included = 1305;
        let expected_reward = CORE_HALVING_DISTRIBUTION[&8] * 3; // 1200 / 150 = 8

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, expected_reward);
    }

    #[test]
    fn test_epoch_core_reward_credits_for_distribution_long_test() {
        let core_subsidy_halving_interval = 150;
        let from_start_block_core_height = 1320;
        let to_end_block_core_height_included = 1320;

        let result = epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        )
        .unwrap();

        assert_eq!(result, 62183484655);
    }
}
