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
    pub fn validate_structure_interval_v0(&self) -> SimpleConsensusValidationResult {
        match self {
            RewardDistributionType::BlockBasedDistribution { interval, .. } => {
                if *interval < 100 {
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
                if *interval < 3_600_000 {
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
