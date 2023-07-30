mod epoch_core_reward_credits_for_distribution;

use dpp::fee::Credits;


/// Actual number of core blocks per calendar year with DGW v3 is ~200700 (for example 449750 - 249050)
pub const CORE_SUBSIDY_HALVING_INTERVAL: u32 = 210240;

/// ORIGINAL CORE BLOCK DISTRIBUTION
/// STARTS AT 25 Dash
/// Take 60% for Masternodes
/// Take 37.5% of that for Platform
const CORE_GENESIS_BLOCK_SUBSIDY: Credits = 585000000000;
