use crate::error::Error;
use crate::execution::platform_events::core_subsidy::{
    CORE_GENESIS_BLOCK_SUBSIDY, CORE_SUBSIDY_HALVING_INTERVAL,
};
use crate::platform_types::platform::Platform;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;

use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
    /// The Core reward halving distribution table for 100 years
    /// Yearly decline of production by ~7.1% per year, projected ~18M coins max by year 2050+.
    pub static ref CORE_HALVING_DISTRIBUTION: HashMap<u16, Credits> = {
        let mut distribution = CORE_GENESIS_BLOCK_SUBSIDY;
        (0..100).map(|i| {
            let old_distribution = distribution;
            distribution -= distribution / 14;
            (i, old_distribution)
        }).collect()
    };
}

impl<C> Platform<C> {
    /// Gets the amount of core reward fees to be distributed for the Epoch.
    pub(super) fn epoch_core_reward_credits_for_distribution_v0(
        epoch_start_block_core_height: u32,
        next_epoch_start_block_core_height: u32,
    ) -> Result<Credits, Error> {
        // Core is halving block rewards every year so we need to pay
        // core block rewards according to halving ratio for the all years during
        // the platform epoch payout period (unpaid epoch)

        // Calculate start and end years for the platform epoch payout period
        // according to start and end core block heights
        let start_core_reward_year =
            (epoch_start_block_core_height / CORE_SUBSIDY_HALVING_INTERVAL) as EpochIndex;
        let end_core_reward_year =
            (next_epoch_start_block_core_height / CORE_SUBSIDY_HALVING_INTERVAL) as EpochIndex;

        let mut total_core_rewards = 0;

        // Calculate block rewards for each core reward year during the platform epoch payout period
        for core_reward_year in start_core_reward_year..=end_core_reward_year {
            // Calculate the block count per core reward year

            let core_reward_year_start_block = if core_reward_year == end_core_reward_year {
                next_epoch_start_block_core_height
            } else {
                (core_reward_year + 1) as u32 * CORE_SUBSIDY_HALVING_INTERVAL
            };

            let core_reward_year_end_block = if core_reward_year == start_core_reward_year {
                epoch_start_block_core_height
            } else {
                core_reward_year as u32 * CORE_SUBSIDY_HALVING_INTERVAL
            };

            let block_count = core_reward_year_start_block - core_reward_year_end_block;

            // Fetch the core block distribution for the corresponding epoch from the distribution table
            // Default to 0 if the core reward year is more than 100 years in the future
            let core_block_distribution_ratio = CORE_HALVING_DISTRIBUTION
                .get(&core_reward_year)
                .unwrap_or(&0);

            // Calculate the core rewards for this epoch and add to the total
            total_core_rewards += block_count as Credits * *core_block_distribution_ratio;
        }

        Ok(total_core_rewards)
    }
}
