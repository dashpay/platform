use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::ProtocolVersionValidator;

pub(crate) fn validate_protocol_version_v0(
    protocol_version: u32,
) -> SimpleConsensusValidationResult {
    let protocol_version_validator = ProtocolVersionValidator::default();
    protocol_version_validator
        .validate(protocol_version)
        .expect("TODO: again, how this will ever fail, why do we even need a validator trait")
}
