use crate::consensus::basic::UnsupportedFeatureError;
use crate::data_contract::associated_token::token_distribution_key::TokenDistributionType;
use crate::state_transition::batch_transition::token_claim_transition::v0::v0_methods::TokenClaimTransitionV0Methods;
use crate::state_transition::batch_transition::token_claim_transition::validate_structure::v0::TokenClaimTransitionActionStructureValidationV0;
use crate::state_transition::batch_transition::TokenClaimTransition;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

mod v0;

pub trait TokenClaimTransitionStructureValidation {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}

impl TokenClaimTransitionStructureValidation for TokenClaimTransition {
    fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        // The once-per-identity distribution type joined the wire at protocol version 14.
        // Older software can not decode it at all, so no historical block can contain one;
        // this check keeps new software in agreement with old software while an earlier
        // protocol version is still active, where the claim would otherwise decode here and
        // be charged while being undecodable on older nodes.
        if self.distribution_type() == TokenDistributionType::OncePerIdentity
            && platform_version
                .dpp
                .validation
                .data_contract
                .validate_once_per_identity_distribution
                .is_none()
        {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                UnsupportedFeatureError::new(
                    "once-per-identity token distribution claim".to_string(),
                    platform_version.protocol_version,
                )
                .into(),
            ));
        }

        match platform_version
            .drive_abci
            .validation_and_processing
            .state_transitions
            .batch_state_transition
            .token_claim_transition_structure_validation
        {
            0 => self.validate_structure_v0(),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "TokenClaimTransition::validate_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}
