use dashcore::Network;
use crate::consensus::basic::BasicError;
use crate::consensus::basic::token::{InvalidTokenDistributionBlockIntervalTooShortError, InvalidTokenDistributionTimeIntervalNotMinuteAlignedError, InvalidTokenDistributionTimeIntervalTooShortError};
use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::validation::SimpleConsensusValidationResult;

impl RewardDistributionType {
    /// Validates the interval structure for reward distribution.
    ///
    /// - For `BlockBasedDistribution`, ensures the interval is at least 100 blocks.
    /// - For `TimeBasedDistribution`, ensures the interval is at least 1 hour (3,600,000 ms)
    ///   and divisible evenly by one minute (60,000 ms).
    /// - For `EpochBasedDistribution`, no specific validation is enforced at this time.
    pub fn validate_structure_interval_v0(
        &self,
        network_type: Network,
    ) -> SimpleConsensusValidationResult {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                let min_block_interval = match network_type {
                    Network::Dash => 100,
                    Network::Testnet => 5,
                    Network::Devnet => 2,
                    Network::Regtest => 1,
                    _ => 100,
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
                let min_block_interval = match network_type {
                    Network::Dash => 3_600_000,
                    Network::Testnet => 600_000,
                    Network::Devnet => 60_000,
                    Network::Regtest => 60_000,
                    _ => 3_600_000,
                };
                if *interval < min_block_interval {
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
            RewardDistributionType::EpochBasedDistribution { .. } => {
                SimpleConsensusValidationResult::new()
            }
        }
    }
}
