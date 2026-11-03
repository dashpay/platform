use crate::consensus::basic::token::{
    InvalidTokenDistributionBlockIntervalTooShortError,
    InvalidTokenDistributionEpochIntervalTooShortError,
    InvalidTokenDistributionTimeIntervalNotMinuteAlignedError,
    InvalidTokenDistributionTimeIntervalTooShortError,
};
use crate::consensus::basic::BasicError;
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::validation::SimpleConsensusValidationResult;
use dashcore::Network;

impl RewardDistributionType {
    /// Validates the interval structure for reward distribution. Selected from protocol
    /// version 14.
    ///
    /// - For `BlockBasedDistribution`, ensures the interval is at least 100 blocks on mainnet.
    /// - For `TimeBasedDistribution`, ensures the interval is at least 1 hour (3,600,000 ms) on
    ///   mainnet and divisible evenly by one minute (60,000 ms).
    /// - For `EpochBasedDistribution`, ensures the interval is at least 1 epoch. Version 0
    ///   accepted 0, and a claim on such a contract then failed as an internal error, since no
    ///   cycle can be computed from a zero step.
    pub fn validate_structure_interval_v1(
        &self,
        network_type: Network,
    ) -> SimpleConsensusValidationResult {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                let min_block_interval = match network_type {
                    Network::Mainnet => 100,
                    Network::Testnet => 5,
                    Network::Devnet => 2,
                    Network::Regtest => 1,
                };
                if *interval < min_block_interval {
                    SimpleConsensusValidationResult::new_with_error(
                        BasicError::InvalidTokenDistributionBlockIntervalTooShortError(
                            InvalidTokenDistributionBlockIntervalTooShortError::new(*interval),
                        )
                        .into(),
                    )
                } else {
                    SimpleConsensusValidationResult::new()
                }
            }
            RewardDistributionType::TimeBasedDistribution { interval, .. } => {
                let min_time_interval = match network_type {
                    Network::Mainnet => 3_600_000,
                    Network::Testnet => 600_000,
                    Network::Devnet => 60_000,
                    Network::Regtest => 60_000,
                };
                if *interval < min_time_interval {
                    return SimpleConsensusValidationResult::new_with_error(
                        BasicError::InvalidTokenDistributionTimeIntervalTooShortError(
                            InvalidTokenDistributionTimeIntervalTooShortError::new(*interval),
                        )
                        .into(),
                    );
                }
                if *interval % 60_000 != 0 {
                    return SimpleConsensusValidationResult::new_with_error(
                        BasicError::InvalidTokenDistributionTimeIntervalNotMinuteAlignedError(
                            InvalidTokenDistributionTimeIntervalNotMinuteAlignedError::new(
                                *interval,
                            ),
                        )
                        .into(),
                    );
                }
                SimpleConsensusValidationResult::new()
            }
            RewardDistributionType::EpochBasedDistribution { interval, .. } => {
                if *interval < 1 {
                    SimpleConsensusValidationResult::new_with_error(
                        BasicError::InvalidTokenDistributionEpochIntervalTooShortError(
                            InvalidTokenDistributionEpochIntervalTooShortError::new(*interval),
                        )
                        .into(),
                    )
                } else {
                    SimpleConsensusValidationResult::new()
                }
            }
        }
    }
}
