pub mod epoch_core_reward_credits_for_distribution;

use crate::fee::Credits;
use dashcore::Network;

/// ORIGINAL CORE BLOCK DISTRIBUTION
/// STARTS AT 5 Dash
/// Take 60% for Masternodes
/// Take 37.5% of that for Platform
const CORE_GENESIS_BLOCK_SUBSIDY: Credits = 112500000000;

pub trait NetworkCoreSubsidy {
    fn core_subsidy_halving_interval(&self) -> u32;
}

impl NetworkCoreSubsidy for Network {
    fn core_subsidy_halving_interval(&self) -> u32 {
        match self {
            Network::Dash => 210240,
            Network::Testnet => 210240,
            Network::Devnet => 210240,
            Network::Regtest => 150,
            _ => 210240,
        }
    }
}
