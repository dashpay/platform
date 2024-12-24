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
use drive::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::{TokenMintTransitionAction, TokenIssuanceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionActionAccessorsV0;
use crate::error::Error;

pub(super) trait TokenIssuanceTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl TokenIssuanceTransitionActionStructureValidationV0 for TokenMintTransitionAction {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let token_configuration = self.base().token_configuration()?;

        token_configuration.Ok(SimpleConsensusValidationResult::default())
    }
}
