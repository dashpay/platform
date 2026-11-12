use crate::data_contract::associated_token::token_pre_programmed_distribution::TokenPreProgrammedDistribution;
use crate::data_contract::TokenContractPosition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

impl TokenPreProgrammedDistribution {
    /// Validates that the amounts of every release can be stored.
    ///
    /// Drive stores each release as a sum tree of its recipients' amounts, so an amount above
    /// `i64::MAX`, or a release whose amounts total more than that, can not be written. Without
    /// this check such a distribution passes validation and then fails inside Drive as an
    /// internal error, which is never paid for and only makes the transition disappear.
    ///
    /// `token_position` is the position of the token in its contract, reported in the error.
    pub fn validate_amounts(
        &self,
        token_position: TokenContractPosition,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .contract_versions
            .token_versions
            .validate_pre_programmed_distribution_amounts
        {
            0 => Ok(self.validate_amounts_v0(token_position)),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenPreProgrammedDistribution::validate_amounts".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
