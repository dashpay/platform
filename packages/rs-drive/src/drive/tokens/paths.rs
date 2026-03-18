use crate::drive::RootTree;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::EpochIndex;
use dpp::data_contract::associated_token::token_perpetual_distribution::methods::v0::TokenPerpetualDistributionV0Accessors;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_moment::RewardDistributionMoment;
use dpp::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use dpp::data_contract::associated_token::token_perpetual_distribution::TokenPerpetualDistribution;
use dpp::prelude::{BlockHeight, TimestampMillis};

// ROOT TOKEN LEVEL

/// Key for accessing token status information.
pub const TOKEN_STATUS_INFO_KEY: u8 = 64;
/// Key for accessing token identity information tree.
pub const TOKEN_IDENTITY_INFO_KEY: u8 = 192;
/// The contract info is used to figure out a contract id from a token id.
pub const TOKEN_CONTRACT_INFO_KEY: u8 = 160;
/// Key for accessing token balances tree.
pub const TOKEN_BALANCES_KEY: u8 = 128;
/// Key that sets the pricing schedule for directly buying the token.
pub const TOKEN_DIRECT_SELL_PRICE_KEY: u8 = 92;

/// Key for token distributions sub level
pub const TOKEN_DISTRIBUTIONS_KEY: u8 = 32;

// The Token Merk tree looks like
//                                                       TOKEN_BALANCES_KEY
//                                           /                                                       \
//                             TOKEN_STATUS_INFO_KEY                                   TOKEN_IDENTITY_INFO_KEY
//                              /             \                                                    /
//           TOKEN_DISTRIBUTIONS_KEY    TOKEN_DIRECT_SELL_PRICE_KEY                  TOKEN_CONTRACT_INFO_KEY

// The token distribution Tree level

/// Key for the perpetual distributions.
pub const TOKEN_TIMED_DISTRIBUTIONS_KEY: u8 = 128;

/// Key for the perpetual distributions.
pub const TOKEN_PERPETUAL_DISTRIBUTIONS_KEY: u8 = 64;

/// Key for the pre-programmed distributions
pub const TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY: u8 = 192;

// The Token Distribution Merk tree looks like
//                                                       TOKEN_TIMED_DISTRIBUTIONS_KEY
//                                           /                                                       \
//                             TOKEN_PERPETUAL_DISTRIBUTIONS_KEY                                   TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY

// Then inside the timed distribution Merk tree we have

/// Key for the millisecond timed token distributions.
pub const TOKEN_MS_TIMED_DISTRIBUTIONS_KEY: u8 = 128;

/// Key for the block timed token distributions.
pub const TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY: u8 = 64;

/// Key for the epoch timed token distributions.
pub const TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY: u8 = 192;

/// Key for the perpetual distribution info.
pub const TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY: u8 = 128;

/// Key for the perpetual distribution last claim for identities key.
pub const TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY: u8 = 192;

/// Key for the perpetual distribution last claim for identities key.
pub const TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY: u8 = 192;

/// The path for the balances tree
pub fn tokens_root_path() -> [&'static [u8]; 1] {
    [Into::<&[u8; 1]>::into(RootTree::Tokens)]
}

/// The path for the balances tree
pub fn tokens_root_path_vec() -> Vec<Vec<u8>> {
    vec![Into::<&[u8; 1]>::into(RootTree::Tokens).to_vec()]
}

/// The root path of token balances tree, this refers to a big sum tree
pub fn token_balances_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
    ]
}

/// The root path of token balances tree, this refers to a big sum tree
pub fn token_balances_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_BALANCES_KEY]]
}

/// The root path of token direct selling price tree
pub fn token_direct_purchase_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DIRECT_SELL_PRICE_KEY],
    ]
}

/// The root path of token direct selling price tree
pub fn token_direct_purchase_root_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DIRECT_SELL_PRICE_KEY],
    ]
}

/// Returns the root path for token identity information as a fixed-size array of byte slices.
pub fn token_identity_infos_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
    ]
}

/// Returns the root path for token identity information as a vector of byte vectors.
pub fn token_identity_infos_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_IDENTITY_INFO_KEY]]
}

/// Returns the root path for token contract information as a fixed-size array of byte slices.
pub fn token_contract_infos_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_CONTRACT_INFO_KEY],
    ]
}

/// Returns the root path for token contract information as a vector of byte vectors.
pub fn token_contract_infos_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_CONTRACT_INFO_KEY]]
}

/// Returns the root path for token statuses as a fixed-size array of byte slices.
pub fn token_statuses_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_STATUS_INFO_KEY],
    ]
}

/// Returns the root path for token statuses as a vector of byte vectors.
pub fn token_statuses_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_STATUS_INFO_KEY]]
}

/// Returns the root path for token distributions as a fixed-size array of byte slices.
pub fn token_distributions_root_path() -> [&'static [u8]; 2] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
    ]
}

/// Returns the root path for token distributions as a vector of byte vectors.
pub fn token_distributions_root_path_vec() -> Vec<Vec<u8>> {
    vec![vec![RootTree::Tokens as u8], vec![TOKEN_DISTRIBUTIONS_KEY]]
}

/// The path for the token timed distributions tree
pub fn token_timed_distributions_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token timed distributions tree as a vector
pub fn token_timed_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token perpetual distributions tree
pub fn token_root_perpetual_distributions_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token perpetual distributions tree as a vector
pub fn token_root_perpetual_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token perpetual distributions tree for a token
pub fn token_perpetual_distributions_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id,
    ]
}

/// The path for the token perpetual distributions tree for a token as a vector
pub fn token_perpetual_distributions_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token perpetual distributions tree for a token
pub fn token_perpetual_distributions_identity_last_claimed_time_path(
    token_id: &[u8; 32],
) -> [&[u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id,
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
    ]
}

/// The path for the token perpetual distributions tree for a token as a vector
pub fn token_perpetual_distributions_identity_last_claimed_time_path_vec(
    token_id: [u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
    ]
}

/// The path for the token perpetual distributions tree for a token
pub fn token_pre_programmed_distributions_identity_last_claimed_time_path(
    token_id: &[u8; 32],
) -> [&[u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id,
        &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
    ]
}

/// The path for the token perpetual distributions tree for a token as a vector
pub fn token_pre_programmed_distributions_identity_last_claimed_time_path_vec(
    token_id: [u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
        vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
    ]
}

/// The path for the token perpetual distributions tree for a token
pub fn token_perpetual_distributions_identity_last_claimed_by_identity_path<'a>(
    token_id: &'a [u8; 32],
    identity_id: &'a [u8; 32],
) -> [&'a [u8]; 6] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id,
        &[TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
        identity_id,
    ]
}

/// The path for the token perpetual distributions tree for a token as a vector
pub fn token_perpetual_distributions_identity_last_claimed_by_identity_path_vec(
    token_id: [u8; 32],
    identity_id: [u8; 32],
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
        vec![TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY],
        identity_id.to_vec(),
    ]
}

/// The path for the token pre-programmed distributions tree
pub fn token_root_pre_programmed_distributions_path() -> [&'static [u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token pre-programmed distributions tree as a vector
pub fn token_root_pre_programmed_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the token pre-programmed distributions tree
pub fn token_pre_programmed_distributions_path(token_id: &[u8; 32]) -> [&[u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id,
    ]
}

/// The path for the token pre-programmed distributions tree as a vector
pub fn token_pre_programmed_distributions_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token pre-programmed distribution tree at a given time
/// These refer to sum trees
pub fn token_pre_programmed_at_time_distribution_path<'a>(
    token_id: &'a [u8; 32],
    time_bytes: &'a [u8; 4],
) -> [&'a [u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id,
        time_bytes,
    ]
}

/// The path for the token pre-programmed distribution tree at a given time as a vector
/// These refer to sum trees
pub fn token_pre_programmed_at_time_distribution_path_vec(
    token_id: [u8; 32],
    timestamp: TimestampMillis,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY],
        token_id.to_vec(),
        timestamp.to_be_bytes().to_vec(),
    ]
}

/// The path for the millisecond timed token distributions tree
pub fn token_ms_timed_distributions_path() -> [&'static [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_MS_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the millisecond timed token distributions tree as a vector
pub fn token_ms_timed_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_MS_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the millisecond timed token distributions tree
pub fn token_ms_timed_at_time_distributions_path(timestamp_bytes: &[u8; 4]) -> [&[u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_MS_TIMED_DISTRIBUTIONS_KEY],
        timestamp_bytes,
    ]
}

/// The path for the millisecond timed token distributions tree as a vector
pub fn token_ms_timed_at_time_distributions_path_vec(time: TimestampMillis) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_MS_TIMED_DISTRIBUTIONS_KEY],
        time.to_be_bytes().to_vec(),
    ]
}

/// The path for the block timed token distributions tree
pub fn token_block_timed_distributions_path() -> [&'static [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the block timed token distributions tree as a vector
pub fn token_block_timed_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the block timed at a specific block height token distributions tree
pub fn token_block_timed_at_block_distributions_path(block_height_bytes: &[u8; 4]) -> [&[u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY],
        block_height_bytes,
    ]
}

/// The path for the block timed at a specific block height token distributions tree as a vector
pub fn token_block_timed_at_block_distributions_path_vec(
    block_height: BlockHeight,
) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY],
        block_height.to_be_bytes().to_vec(),
    ]
}

/// The path for the epoch timed token distributions tree
pub fn token_epoch_timed_distributions_path() -> [&'static [u8]; 4] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the epoch timed token distributions tree as a vector
pub fn token_epoch_timed_distributions_path_vec() -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY],
    ]
}

/// The path for the epoch timed at a specific epoch token distributions tree
pub fn token_epoch_timed_at_epoch_distributions_path(epoch_index_bytes: &[u8; 2]) -> [&[u8]; 5] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_DISTRIBUTIONS_KEY],
        &[TOKEN_TIMED_DISTRIBUTIONS_KEY],
        &[TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY],
        epoch_index_bytes,
    ]
}

/// The path for the epoch timed at a specific epoch token distributions tree as a vector
pub fn token_epoch_timed_at_epoch_distributions_path_vec(epoch_index: EpochIndex) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_DISTRIBUTIONS_KEY],
        vec![TOKEN_TIMED_DISTRIBUTIONS_KEY],
        vec![TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY],
        epoch_index.to_be_bytes().to_vec(),
    ]
}

/// The path for the token balances tree
pub fn token_balances_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_BALANCES_KEY],
        token_id,
    ]
}

/// The path for the token balances tree
pub fn token_balances_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_BALANCES_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token identity info tree
pub fn token_identity_infos_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_IDENTITY_INFO_KEY],
        token_id,
    ]
}

/// The path for the token identity info tree
pub fn token_identity_infos_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_IDENTITY_INFO_KEY],
        token_id.to_vec(),
    ]
}

/// The path for the token contract info tree
pub fn token_contract_infos_path(token_id: &[u8; 32]) -> [&[u8]; 3] {
    [
        Into::<&[u8; 1]>::into(RootTree::Tokens),
        &[TOKEN_CONTRACT_INFO_KEY],
        token_id,
    ]
}

/// The path for the token contract info tree
pub fn token_contract_infos_path_vec(token_id: [u8; 32]) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::Tokens as u8],
        vec![TOKEN_CONTRACT_INFO_KEY],
        token_id.to_vec(),
    ]
}

/// Paths for the token perpetual distribution
pub trait TokenPerpetualDistributionPaths {
    /// Returns the path where the perpetual distribution times should be stored.
    fn root_distribution_path(&self) -> Vec<Vec<u8>>;

    /// Returns the path where the perpetual distribution should be stored.
    fn distribution_path(&self, unit: u64) -> Vec<Vec<u8>>;
    /// Returns the path where the perpetual distribution should be stored.
    fn distribution_path_for_next_interval_from_block_info(
        &self,
        block_info: &BlockInfo,
    ) -> Vec<Vec<u8>>;
}

impl TokenPerpetualDistributionPaths for TokenPerpetualDistribution {
    fn root_distribution_path(&self) -> Vec<Vec<u8>> {
        match self.distribution_type() {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                token_block_timed_distributions_path_vec()
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                token_ms_timed_distributions_path_vec()
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                token_epoch_timed_distributions_path_vec()
            }
        }
    }

    fn distribution_path(&self, unit: u64) -> Vec<Vec<u8>> {
        match self.distribution_type() {
            RewardDistributionType::BlockBasedDistribution { .. } => {
                token_block_timed_at_block_distributions_path_vec(unit)
            }
            RewardDistributionType::TimeBasedDistribution { .. } => {
                token_ms_timed_at_time_distributions_path_vec(unit)
            }
            RewardDistributionType::EpochBasedDistribution { .. } => {
                token_epoch_timed_at_epoch_distributions_path_vec(unit as EpochIndex)
            }
        }
    }

    fn distribution_path_for_next_interval_from_block_info(
        &self,
        block_info: &BlockInfo,
    ) -> Vec<Vec<u8>> {
        match self.distribution_type() {
            // If the distribution is based on block height, return the next height where emissions occur.
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                let height = block_info.height - block_info.height % interval + interval;
                token_block_timed_at_block_distributions_path_vec(height)
            }

            // If the distribution is based on time, return the next timestamp in milliseconds.
            RewardDistributionType::TimeBasedDistribution { interval, .. } => {
                let time = block_info.time_ms - block_info.time_ms % interval + interval;
                token_ms_timed_at_time_distributions_path_vec(time)
            }

            // If the distribution is based on epochs, return the next epoch index.
            RewardDistributionType::EpochBasedDistribution { interval, .. } => {
                let index = block_info.epoch.index - block_info.epoch.index % interval + interval;
                token_epoch_timed_at_epoch_distributions_path_vec(index)
            }
        }
    }
}

/// Paths for the token perpetual distribution moment
pub trait TokenPerpetualDistributionMomentPaths {
    /// The distribution path for a moment
    fn distribution_path(&self) -> Vec<Vec<u8>>;
}

impl TokenPerpetualDistributionMomentPaths for RewardDistributionMoment {
    fn distribution_path(&self) -> Vec<Vec<u8>> {
        match self {
            RewardDistributionMoment::BlockBasedMoment(height) => {
                token_block_timed_at_block_distributions_path_vec(*height)
            }
            RewardDistributionMoment::TimeBasedMoment(time_ms) => {
                token_ms_timed_at_time_distributions_path_vec(*time_ms)
            }
            RewardDistributionMoment::EpochBasedMoment(epoch) => {
                token_epoch_timed_at_epoch_distributions_path_vec(*epoch)
            }
        }
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::RootTree;

    fn tokens_root_byte() -> u8 {
        RootTree::Tokens as u8
    }

    // ---- tokens_root_path / tokens_root_path_vec ----

    #[test]
    fn test_tokens_root_path() {
        let path = tokens_root_path();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], &[tokens_root_byte()]);
    }

    #[test]
    fn test_tokens_root_path_vec() {
        let path = tokens_root_path_vec();
        assert_eq!(path.len(), 1);
        assert_eq!(path[0], vec![tokens_root_byte()]);
    }

    // ---- token_balances_root_path / token_balances_root_path_vec ----

    #[test]
    fn test_token_balances_root_path() {
        let path = token_balances_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_BALANCES_KEY]);
    }

    #[test]
    fn test_token_balances_root_path_vec() {
        let path = token_balances_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_BALANCES_KEY]);
    }

    // ---- token_direct_purchase_root_path / token_direct_purchase_root_path_vec ----

    #[test]
    fn test_token_direct_purchase_root_path() {
        let path = token_direct_purchase_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DIRECT_SELL_PRICE_KEY]);
    }

    #[test]
    fn test_token_direct_purchase_root_path_vec() {
        let path = token_direct_purchase_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DIRECT_SELL_PRICE_KEY]);
    }

    // ---- token_identity_infos_root_path / token_identity_infos_root_path_vec ----

    #[test]
    fn test_token_identity_infos_root_path() {
        let path = token_identity_infos_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_IDENTITY_INFO_KEY]);
    }

    #[test]
    fn test_token_identity_infos_root_path_vec() {
        let path = token_identity_infos_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_IDENTITY_INFO_KEY]);
    }

    // ---- token_contract_infos_root_path / token_contract_infos_root_path_vec ----

    #[test]
    fn test_token_contract_infos_root_path() {
        let path = token_contract_infos_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_CONTRACT_INFO_KEY]);
    }

    #[test]
    fn test_token_contract_infos_root_path_vec() {
        let path = token_contract_infos_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_CONTRACT_INFO_KEY]);
    }

    // ---- token_statuses_root_path / token_statuses_root_path_vec ----

    #[test]
    fn test_token_statuses_root_path() {
        let path = token_statuses_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_STATUS_INFO_KEY]);
    }

    #[test]
    fn test_token_statuses_root_path_vec() {
        let path = token_statuses_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_STATUS_INFO_KEY]);
    }

    // ---- token_distributions_root_path / token_distributions_root_path_vec ----

    #[test]
    fn test_token_distributions_root_path() {
        let path = token_distributions_root_path();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_distributions_root_path_vec() {
        let path = token_distributions_root_path_vec();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DISTRIBUTIONS_KEY]);
    }

    // ---- token_timed_distributions_path / token_timed_distributions_path_vec ----

    #[test]
    fn test_token_timed_distributions_path() {
        let path = token_timed_distributions_path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_TIMED_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_timed_distributions_path_vec() {
        let path = token_timed_distributions_path_vec();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], vec![TOKEN_TIMED_DISTRIBUTIONS_KEY]);
    }

    // ---- token_root_perpetual_distributions_path / _vec ----

    #[test]
    fn test_token_root_perpetual_distributions_path() {
        let path = token_root_perpetual_distributions_path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_root_perpetual_distributions_path_vec() {
        let path = token_root_perpetual_distributions_path_vec();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
    }

    // ---- token_perpetual_distributions_path with token_id ----

    #[test]
    fn test_token_perpetual_distributions_path() {
        let token_id = [0xAAu8; 32];
        let path = token_perpetual_distributions_path(&token_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &token_id);
    }

    #[test]
    fn test_token_perpetual_distributions_path_vec() {
        let token_id = [0xBBu8; 32];
        let path = token_perpetual_distributions_path_vec(token_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], vec![TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], token_id.to_vec());
    }

    // ---- token_perpetual_distributions_identity_last_claimed_time_path ----

    #[test]
    fn test_token_perpetual_distributions_identity_last_claimed_time_path() {
        let token_id = [0xCCu8; 32];
        let path = token_perpetual_distributions_identity_last_claimed_time_path(&token_id);
        assert_eq!(path.len(), 5);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &token_id);
        assert_eq!(
            path[4],
            &[TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY]
        );
    }

    #[test]
    fn test_token_perpetual_distributions_identity_last_claimed_time_path_vec() {
        let token_id = [0xDDu8; 32];
        let path = token_perpetual_distributions_identity_last_claimed_time_path_vec(token_id);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], token_id.to_vec());
        assert_eq!(
            path[4],
            vec![TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY]
        );
    }

    // ---- token_pre_programmed_distributions_identity_last_claimed_time_path ----

    #[test]
    fn test_token_pre_programmed_distributions_identity_last_claimed_time_path() {
        let token_id = [0x11u8; 32];
        let path = token_pre_programmed_distributions_identity_last_claimed_time_path(&token_id);
        assert_eq!(path.len(), 5);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &token_id);
        assert_eq!(
            path[4],
            &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY]
        );
    }

    #[test]
    fn test_token_pre_programmed_distributions_identity_last_claimed_time_path_vec() {
        let token_id = [0x22u8; 32];
        let path = token_pre_programmed_distributions_identity_last_claimed_time_path_vec(token_id);
        assert_eq!(path.len(), 5);
        assert_eq!(path[2], vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], token_id.to_vec());
        assert_eq!(
            path[4],
            vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY]
        );
    }

    // ---- token_perpetual_distributions_identity_last_claimed_by_identity_path ----

    #[test]
    fn test_token_perpetual_distributions_identity_last_claimed_by_identity_path() {
        let token_id = [0xAAu8; 32];
        let identity_id = [0xBBu8; 32];
        let path = token_perpetual_distributions_identity_last_claimed_by_identity_path(
            &token_id,
            &identity_id,
        );
        assert_eq!(path.len(), 6);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PERPETUAL_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &token_id);
        assert_eq!(
            path[4],
            &[TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY]
        );
        assert_eq!(path[5], &identity_id);
    }

    #[test]
    fn test_token_perpetual_distributions_identity_last_claimed_by_identity_path_vec() {
        let token_id = [0xCCu8; 32];
        let identity_id = [0xDDu8; 32];
        let path = token_perpetual_distributions_identity_last_claimed_by_identity_path_vec(
            token_id,
            identity_id,
        );
        assert_eq!(path.len(), 6);
        assert_eq!(path[3], token_id.to_vec());
        assert_eq!(path[5], identity_id.to_vec());
    }

    // ---- token_root_pre_programmed_distributions_path ----

    #[test]
    fn test_token_root_pre_programmed_distributions_path() {
        let path = token_root_pre_programmed_distributions_path();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_root_pre_programmed_distributions_path_vec() {
        let path = token_root_pre_programmed_distributions_path_vec();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], vec![TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY]);
    }

    // ---- token_pre_programmed_distributions_path with token_id ----

    #[test]
    fn test_token_pre_programmed_distributions_path() {
        let token_id = [0x55u8; 32];
        let path = token_pre_programmed_distributions_path(&token_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], &token_id);
    }

    #[test]
    fn test_token_pre_programmed_distributions_path_vec() {
        let token_id = [0x66u8; 32];
        let path = token_pre_programmed_distributions_path_vec(token_id);
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], token_id.to_vec());
    }

    // ---- token_pre_programmed_at_time_distribution_path ----

    #[test]
    fn test_token_pre_programmed_at_time_distribution_path() {
        let token_id = [0x77u8; 32];
        let time_bytes: [u8; 4] = [0, 0, 0, 42];
        let path = token_pre_programmed_at_time_distribution_path(&token_id, &time_bytes);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], &token_id);
        assert_eq!(path[4], &time_bytes);
    }

    #[test]
    fn test_token_pre_programmed_at_time_distribution_path_vec() {
        let token_id = [0x88u8; 32];
        let timestamp: u64 = 1234567890;
        let path = token_pre_programmed_at_time_distribution_path_vec(token_id, timestamp);
        assert_eq!(path.len(), 5);
        assert_eq!(path[3], token_id.to_vec());
        assert_eq!(path[4], timestamp.to_be_bytes().to_vec());
    }

    // ---- token_ms_timed_distributions_path ----

    #[test]
    fn test_token_ms_timed_distributions_path() {
        let path = token_ms_timed_distributions_path();
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_TIMED_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &[TOKEN_MS_TIMED_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_ms_timed_distributions_path_vec() {
        let path = token_ms_timed_distributions_path_vec();
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], vec![TOKEN_MS_TIMED_DISTRIBUTIONS_KEY]);
    }

    // ---- token_ms_timed_at_time_distributions_path ----

    #[test]
    fn test_token_ms_timed_at_time_distributions_path() {
        let ts_bytes: [u8; 4] = [0, 0, 1, 0];
        let path = token_ms_timed_at_time_distributions_path(&ts_bytes);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], &ts_bytes);
    }

    #[test]
    fn test_token_ms_timed_at_time_distributions_path_vec() {
        let time: u64 = 42000;
        let path = token_ms_timed_at_time_distributions_path_vec(time);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], time.to_be_bytes().to_vec());
    }

    // ---- token_block_timed_distributions_path ----

    #[test]
    fn test_token_block_timed_distributions_path() {
        let path = token_block_timed_distributions_path();
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_TIMED_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &[TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_block_timed_distributions_path_vec() {
        let path = token_block_timed_distributions_path_vec();
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], vec![TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY]);
    }

    // ---- token_block_timed_at_block_distributions_path ----

    #[test]
    fn test_token_block_timed_at_block_distributions_path() {
        let height_bytes: [u8; 4] = 100u32.to_be_bytes();
        let path = token_block_timed_at_block_distributions_path(&height_bytes);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], &height_bytes);
    }

    #[test]
    fn test_token_block_timed_at_block_distributions_path_vec() {
        let height: u64 = 500;
        let path = token_block_timed_at_block_distributions_path_vec(height);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], height.to_be_bytes().to_vec());
    }

    // ---- token_epoch_timed_distributions_path ----

    #[test]
    fn test_token_epoch_timed_distributions_path() {
        let path = token_epoch_timed_distributions_path();
        assert_eq!(path.len(), 4);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_DISTRIBUTIONS_KEY]);
        assert_eq!(path[2], &[TOKEN_TIMED_DISTRIBUTIONS_KEY]);
        assert_eq!(path[3], &[TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY]);
    }

    #[test]
    fn test_token_epoch_timed_distributions_path_vec() {
        let path = token_epoch_timed_distributions_path_vec();
        assert_eq!(path.len(), 4);
        assert_eq!(path[3], vec![TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY]);
    }

    // ---- token_epoch_timed_at_epoch_distributions_path ----

    #[test]
    fn test_token_epoch_timed_at_epoch_distributions_path() {
        let epoch_bytes: [u8; 2] = 5u16.to_be_bytes();
        let path = token_epoch_timed_at_epoch_distributions_path(&epoch_bytes);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], &epoch_bytes);
    }

    #[test]
    fn test_token_epoch_timed_at_epoch_distributions_path_vec() {
        let epoch_index: u16 = 10;
        let path = token_epoch_timed_at_epoch_distributions_path_vec(epoch_index);
        assert_eq!(path.len(), 5);
        assert_eq!(path[4], epoch_index.to_be_bytes().to_vec());
    }

    // ---- token_balances_path with token_id ----

    #[test]
    fn test_token_balances_path() {
        let token_id = [0x01u8; 32];
        let path = token_balances_path(&token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_BALANCES_KEY]);
        assert_eq!(path[2], &token_id);
    }

    #[test]
    fn test_token_balances_path_vec() {
        let token_id = [0x02u8; 32];
        let path = token_balances_path_vec(token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], vec![tokens_root_byte()]);
        assert_eq!(path[1], vec![TOKEN_BALANCES_KEY]);
        assert_eq!(path[2], token_id.to_vec());
    }

    // ---- token_identity_infos_path with token_id ----

    #[test]
    fn test_token_identity_infos_path() {
        let token_id = [0x03u8; 32];
        let path = token_identity_infos_path(&token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_IDENTITY_INFO_KEY]);
        assert_eq!(path[2], &token_id);
    }

    #[test]
    fn test_token_identity_infos_path_vec() {
        let token_id = [0x04u8; 32];
        let path = token_identity_infos_path_vec(token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[1], vec![TOKEN_IDENTITY_INFO_KEY]);
        assert_eq!(path[2], token_id.to_vec());
    }

    // ---- token_contract_infos_path with token_id ----

    #[test]
    fn test_token_contract_infos_path() {
        let token_id = [0x05u8; 32];
        let path = token_contract_infos_path(&token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[0], &[tokens_root_byte()]);
        assert_eq!(path[1], &[TOKEN_CONTRACT_INFO_KEY]);
        assert_eq!(path[2], &token_id);
    }

    #[test]
    fn test_token_contract_infos_path_vec() {
        let token_id = [0x06u8; 32];
        let path = token_contract_infos_path_vec(token_id);
        assert_eq!(path.len(), 3);
        assert_eq!(path[1], vec![TOKEN_CONTRACT_INFO_KEY]);
        assert_eq!(path[2], token_id.to_vec());
    }

    // ---- Consistency: array and vec versions produce same data ----

    #[test]
    fn array_and_vec_root_paths_are_consistent() {
        let arr = tokens_root_path();
        let v = tokens_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_balances_root_paths_are_consistent() {
        let arr = token_balances_root_path();
        let v = token_balances_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_balances_paths_are_consistent() {
        let token_id = [0xFFu8; 32];
        let arr = token_balances_path(&token_id);
        let v = token_balances_path_vec(token_id);
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_identity_infos_root_paths_are_consistent() {
        let arr = token_identity_infos_root_path();
        let v = token_identity_infos_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_contract_infos_root_paths_are_consistent() {
        let arr = token_contract_infos_root_path();
        let v = token_contract_infos_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_statuses_root_paths_are_consistent() {
        let arr = token_statuses_root_path();
        let v = token_statuses_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_distributions_root_paths_are_consistent() {
        let arr = token_distributions_root_path();
        let v = token_distributions_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_direct_purchase_root_paths_are_consistent() {
        let arr = token_direct_purchase_root_path();
        let v = token_direct_purchase_root_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_timed_distributions_paths_are_consistent() {
        let arr = token_timed_distributions_path();
        let v = token_timed_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_perpetual_distributions_root_paths_are_consistent() {
        let arr = token_root_perpetual_distributions_path();
        let v = token_root_perpetual_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_pre_programmed_distributions_root_paths_are_consistent() {
        let arr = token_root_pre_programmed_distributions_path();
        let v = token_root_pre_programmed_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_ms_timed_distributions_paths_are_consistent() {
        let arr = token_ms_timed_distributions_path();
        let v = token_ms_timed_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_block_timed_distributions_paths_are_consistent() {
        let arr = token_block_timed_distributions_path();
        let v = token_block_timed_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    #[test]
    fn array_and_vec_epoch_timed_distributions_paths_are_consistent() {
        let arr = token_epoch_timed_distributions_path();
        let v = token_epoch_timed_distributions_path_vec();
        assert_eq!(arr.len(), v.len());
        for (a, b) in arr.iter().zip(v.iter()) {
            assert_eq!(*a, b.as_slice());
        }
    }

    // ---- Key constant values ----

    #[test]
    fn key_constants_have_expected_values() {
        // Verify the tree structure constants are as documented
        assert_eq!(TOKEN_STATUS_INFO_KEY, 64);
        assert_eq!(TOKEN_IDENTITY_INFO_KEY, 192);
        assert_eq!(TOKEN_CONTRACT_INFO_KEY, 160);
        assert_eq!(TOKEN_BALANCES_KEY, 128);
        assert_eq!(TOKEN_DIRECT_SELL_PRICE_KEY, 92);
        assert_eq!(TOKEN_DISTRIBUTIONS_KEY, 32);

        // Distribution sub-keys
        assert_eq!(TOKEN_TIMED_DISTRIBUTIONS_KEY, 128);
        assert_eq!(TOKEN_PERPETUAL_DISTRIBUTIONS_KEY, 64);
        assert_eq!(TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_KEY, 192);

        // Timed distribution sub-keys
        assert_eq!(TOKEN_MS_TIMED_DISTRIBUTIONS_KEY, 128);
        assert_eq!(TOKEN_BLOCK_TIMED_DISTRIBUTIONS_KEY, 64);
        assert_eq!(TOKEN_EPOCH_TIMED_DISTRIBUTIONS_KEY, 192);

        // Perpetual distribution sub-keys
        assert_eq!(TOKEN_PERPETUAL_DISTRIBUTIONS_INFO_KEY, 128);
        assert_eq!(
            TOKEN_PERPETUAL_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
            192
        );
        assert_eq!(
            TOKEN_PRE_PROGRAMMED_DISTRIBUTIONS_FOR_IDENTITIES_LAST_CLAIM_KEY,
            192
        );
    }

    // ---- Different token_ids produce different paths ----

    #[test]
    fn different_token_ids_produce_different_balance_paths() {
        let id_a = [0x01u8; 32];
        let id_b = [0x02u8; 32];
        let path_a = token_balances_path_vec(id_a);
        let path_b = token_balances_path_vec(id_b);
        // First two segments should be the same, third should differ
        assert_eq!(path_a[0], path_b[0]);
        assert_eq!(path_a[1], path_b[1]);
        assert_ne!(path_a[2], path_b[2]);
    }

    // ---- RewardDistributionMoment paths ----

    #[test]
    fn reward_distribution_moment_block_based_path() {
        let moment = RewardDistributionMoment::BlockBasedMoment(100);
        let path = moment.distribution_path();
        let expected = token_block_timed_at_block_distributions_path_vec(100);
        assert_eq!(path, expected);
    }

    #[test]
    fn reward_distribution_moment_time_based_path() {
        let moment = RewardDistributionMoment::TimeBasedMoment(5000);
        let path = moment.distribution_path();
        let expected = token_ms_timed_at_time_distributions_path_vec(5000);
        assert_eq!(path, expected);
    }

    #[test]
    fn reward_distribution_moment_epoch_based_path() {
        let moment = RewardDistributionMoment::EpochBasedMoment(7);
        let path = moment.distribution_path();
        let expected = token_epoch_timed_at_epoch_distributions_path_vec(7);
        assert_eq!(path, expected);
    }
}
