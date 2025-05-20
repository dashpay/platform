mod v0;

use crate::data_contract::associated_token::token_perpetual_distribution::reward_distribution_type::RewardDistributionType;
use crate::version::PlatformVersion;
use crate::ProtocolError;
use crate::validation::SimpleConsensusValidationResult;

impl RewardDistributionType {
    /// Validates the interval structure for reward distribution.
    ///
    /// - For `BlockBasedDistribution`, ensures the interval is at least 100 blocks.
    /// - For `TimeBasedDistribution`, ensures the interval is at least 1 hour (3,600,000 ms)
    ///   and divisible evenly by one minute (60,000 ms).
    /// - For `EpochBasedDistribution`, no specific validation is enforced at this time.
    pub fn validate_structure_interval(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .token_versions
            .validate_structure_interval
        {
            0 => Ok(self.validate_structure_interval_v0()),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "RewardDistributionType::validate_structure_interval".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
