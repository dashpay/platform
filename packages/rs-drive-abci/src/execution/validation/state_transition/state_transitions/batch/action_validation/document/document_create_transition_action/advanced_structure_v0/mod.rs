use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{DocumentCreationNotAllowedError, InvalidDocumentTypeError};
use dpp::consensus::state::document::document_contest_not_paid_for_error::DocumentContestNotPaidForError;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::identifier::Identifier;
use dpp::validation::{SimpleConsensusValidationResult};
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentCreateTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentCreateTransitionActionStructureValidationV0 for DocumentCreateTransitionAction {
    fn validate_structure_v0(
        &self,
        owner_id: Identifier,
        block_info: &BlockInfo,
        network: Network,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        // Make sure that the document type is defined in the contract
        let document_type_name = self.base().document_type_name();

        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        // Don't do the following validation on testnet before epoch 2080
        // As state transitions already happened that would break this validation
        // We want to keep both if-s for better readability
        #[allow(clippy::collapsible_if)]
        if !(network == Network::Testnet && block_info.epoch.index < 2080) {
            // Only for contested documents
            if document_type
                .contested_vote_poll_for_document_properties(self.data(), platform_version)?
                .is_some()
            {
                let expected_amount = platform_version
                    .fee_version
                    .vote_resolution_fund_fees
                    .contested_document_vote_resolution_fund_required_amount;
                if let Some((_, paid_amount)) = self.prefunded_voting_balance() {
                    if expected_amount != *paid_amount {
                        return Ok(SimpleConsensusValidationResult::new_with_error(
                            DocumentContestNotPaidForError::new(
                                self.base().id(),
                                expected_amount,
                                *paid_amount,
                            )
                            .into(),
                        ));
                    }
                } else {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentContestNotPaidForError::new(self.base().id(), expected_amount, 0)
                            .into(),
                    ));
                }
            }
        }

        match document_type.creation_restriction_mode() {
            CreationRestrictionMode::NoRestrictions => {}
            CreationRestrictionMode::OwnerOnly => {
                if owner_id != data_contract.owner_id() {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        DocumentCreationNotAllowedError::new(
                            self.base().data_contract_id(),
                            document_type_name.clone(),
                            document_type.creation_restriction_mode(),
                        )
                        .into(),
                    ));
                }
            }
            CreationRestrictionMode::NoCreationAllowed => {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentCreationNotAllowedError::new(
                        self.base().data_contract_id(),
                        document_type_name.clone(),
                        document_type.creation_restriction_mode(),
                    )
                    .into(),
                ));
            }
        }
        // Validate user defined properties

        data_contract
            .validate_document_properties(document_type_name, self.data().into(), platform_version)
            .map_err(Error::Protocol)
    }
}
