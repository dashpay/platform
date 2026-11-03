use dpp::consensus::basic::document::{InvalidDocumentTransitionActionError, InvalidDocumentTypeError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::property_constraints::{DocumentSystemValues, SystemChange};
use dpp::document::DocumentV0Getters;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0};
use dpp::version::PlatformVersion;
use crate::error::Error;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentUpdatePriceTransitionActionStructureValidationV0 {
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}
impl DocumentUpdatePriceTransitionActionStructureValidationV0
    for DocumentUpdatePriceTransitionAction
{
    fn validate_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let data_contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();

        // Make sure that the document type is defined in the contract
        let Some(document_type) = data_contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), data_contract.id())
                    .into(),
            ));
        };

        if !document_type.trade_mode().seller_sets_price() {
            Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTransitionActionError::new(format!(
                    "{} is in trade mode {} that does not support the seller setting the price",
                    document_type_name,
                    document_type.trade_mode(),
                ))
                .into(),
            ))
        } else {
            // Added in place at protocol version 14, inert for every earlier version this
            // generation serves: `validate_property_constraints` is `None` there, so the call
            // returns an empty result. From 14, a price update sets the document's update
            // time and heights, and is judged against the rules of `propertyConstraints`
            // reading them: the stored properties met every rule when they were written,
            // and those times and heights are all this action changes that a rule reads.
            // Such a rule reading a `countOf` or `sumOf` too reads the total the action
            // read from state.
            document_type
                .validate_property_constraints_for_system_change(
                    self.document().properties(),
                    &DocumentSystemValues {
                        aggregates: Some(self.property_constraint_aggregates().clone()),
                        ..DocumentSystemValues::of_document(self.document())
                    },
                    SystemChange::PriceUpdate,
                    platform_version,
                )
                .map_err(Error::Protocol)
        }
    }
}
