use dpp::block::epoch::Epoch;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::document::documents_batch::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::platform_types::platform::PlatformStateRef;

pub(super) trait DocumentUpdatePriceTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentUpdatePriceTransitionActionStateValidationV0 for DocumentUpdatePriceTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        _epoch: &Epoch,
        _execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();

        let contract = &contract_fetch_info.contract;

        let document_type_name = self.base().document_type_name();

        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        // There is no need to verify that the document already existed, since this is done when
        // transforming into an action

        if document_type.indices().iter().any(|index| index.unique) {
            platform
                .drive
                .validate_document_update_price_transition_action_uniqueness(
                    contract,
                    document_type,
                    self,
                    owner_id,
                    transaction,
                    platform_version,
                )
                .map_err(Error::Drive)
        } else {
            Ok(SimpleConsensusValidationResult::new())
        }
    }
}
