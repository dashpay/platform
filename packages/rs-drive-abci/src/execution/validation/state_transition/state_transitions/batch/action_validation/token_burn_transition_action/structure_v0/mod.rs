use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{DocumentCreationNotAllowedError, InvalidDocumentTypeError};
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::accessors::v1::DataContractV1Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;

pub(super) trait TokenBurnTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenBurnTransitionActionStructureValidationV0 for TokenBurnTransitionAction {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let token_configuration = self.base().token_configuration()?;

        Ok(SimpleConsensusValidationResult::default())
    }
}
