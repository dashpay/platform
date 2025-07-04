use platform_version::version::PlatformVersion;
use crate::consensus::basic::token::{InvalidTokenConfigUpdateNoChangeError, InvalidTokenNoteTooBigError, TokenNoteOnlyAllowedWhenProposerError};
use crate::consensus::basic::{BasicError, UnsupportedFeatureError};
use crate::consensus::ConsensusError;
use crate::data_contract::associated_token::token_configuration_item::TokenConfigurationChangeItem;
use crate::state_transition::batch_transition::token_config_update_transition::v0::v0_methods::TokenConfigUpdateTransitionV0Methods;
use crate::state_transition::batch_transition::TokenConfigUpdateTransition;
use crate::tokens::MAX_TOKEN_NOTE_LEN;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;
use crate::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;

pub(super) trait TokenConfigUpdateTransitionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError>;
}
impl TokenConfigUpdateTransitionStructureValidationV0 for TokenConfigUpdateTransition {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        if matches!(
            self.update_token_configuration_item(),
            TokenConfigurationChangeItem::TokenConfigurationNoChange
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::InvalidTokenConfigUpdateNoChangeError(
                    InvalidTokenConfigUpdateNoChangeError::new(),
                )),
            ));
        }

        if matches!(
            self.update_token_configuration_item(),
            TokenConfigurationChangeItem::PerpetualDistribution(_)
        ) {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::BasicError(BasicError::UnsupportedFeatureError(
                    UnsupportedFeatureError::new(
                        "of changing perpetual distribution".to_string(),
                        platform_version.protocol_version,
                    ),
                )),
            ));
        }

        if let Some(public_note) = self.public_note() {
            if public_note.len() > MAX_TOKEN_NOTE_LEN {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    ConsensusError::BasicError(BasicError::InvalidTokenNoteTooBigError(
                        InvalidTokenNoteTooBigError::new(
                            MAX_TOKEN_NOTE_LEN as u32,
                            "public_note",
                            public_note.len() as u32,
                        ),
                    )),
                ));
            }
            if let Some(group_state_transition_info) = self.base().using_group_info() {
                if !group_state_transition_info.action_is_proposer {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::BasicError(
                            BasicError::TokenNoteOnlyAllowedWhenProposerError(
                                TokenNoteOnlyAllowedWhenProposerError::new(),
                            ),
                        ),
                    ));
                }
            }
        }

        Ok(SimpleConsensusValidationResult::default())
    }
}
