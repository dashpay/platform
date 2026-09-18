use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_immutable_property_changed_error::DocumentImmutablePropertyChangedError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{
    DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0,
};

use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_reference_validation::DocumentReferenceValidation;
use crate::execution::validation::state_transition::batch::action_validation::document::document_replace_transition_action::state_v0::DocumentReplaceTransitionActionStateValidationV0;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentReplaceTransitionActionStateValidationV1
{
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentReplaceTransitionActionStateValidationV1 for DocumentReplaceTransitionAction {
    fn validate_state_v1(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.validate_state_v0(
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        // Immutable properties (protocol version 14). The action already
        // knows which top-level properties differ from the stored document
        // (a changed value, a property added, or a property removed), so the
        // check is an intersection with the type's `immutable` list. The one
        // allowed difference is a first-time set of a property listed under
        // `immutableAllowSetting`: the stored document had no value for it
        // (`added_data_fields`), so nothing was frozen yet. It reads no
        // state, so it runs before the reference validation, which does.
        // `changed_data_fields` is name-ordered, so the reported property is
        // deterministic.
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let document_type_name = self.base().document_type_name();
        // V0 above has already refused an unknown document type.
        let document_type = contract_fetch_info
            .contract
            .document_type_for_name(document_type_name)?;
        let immutable_fields = document_type.immutable_fields();
        if !immutable_fields.is_empty() {
            let allow_setting = document_type.immutable_fields_allow_setting();
            let added_fields = self.added_data_fields();
            if let Some(property) = self.changed_data_fields().iter().find(|field| {
                immutable_fields.contains(*field)
                    && !(allow_setting.contains(*field) && added_fields.contains(*field))
            }) {
                return Ok(SimpleConsensusValidationResult::new_with_error(
                    DocumentImmutablePropertyChangedError::new(
                        self.base().id(),
                        document_type_name.clone(),
                        property.clone(),
                    )
                    .into(),
                ));
            }
        }

        let reference_result = self.base().validate_document_references(
            self.data(),
            Some(self.changed_data_fields()),
            platform,
            block_info,
            transaction,
            execution_context,
            platform_version,
        )?;
        if !reference_result.is_valid() {
            return Ok(reference_result);
        }

        Ok(SimpleConsensusValidationResult::new())
    }
}
