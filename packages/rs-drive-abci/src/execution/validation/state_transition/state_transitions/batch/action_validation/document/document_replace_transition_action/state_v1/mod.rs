use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_immutable_property_changed_error::DocumentImmutablePropertyChangedError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::property_constraints::{
    DocumentSystemValues, STORED_DOCUMENT_KEY,
};
use dpp::identifier::Identifier;
use dpp::platform_value::Value;
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
        // check is an intersection with what the type's `immutable` keyword
        // freezes: the properties it lists by name, and those it lists with a
        // condition that holds for this replace. A condition is judged as a
        // `propertyConstraints` rule judges a replace, on the document the
        // replace writes (its `$updatedAt` this block's time), with the stored
        // properties under `$old`, rebuilt from the written ones and the stored
        // value of each changed one: an added property the stored document did
        // not hold, a removed one it did. A condition that cannot be evaluated
        // (an overflow, a division by zero) counts as holding: the property
        // stays frozen rather than freed by a fault. Conditions are evaluated
        // only for the properties the replace changes.
        //
        // One difference is allowed: clearing a `deletableDocument` reference
        // listed without a condition whose target has been deleted. Every
        // replace re-validates such a reference, so a document whose frozen
        // reference went dead could otherwise never be replaced again, and
        // clearing it is the only change that does not rewrite what the
        // document pointed at. The document type parser (generation 3) refuses
        // a condition on a `deletableDocument` reference by id, at
        // registration and on every contract update, which re-parses the whole
        // contract: once cleared, a replace the condition left free could set
        // it again, to another document. The clear alone reads state (the
        // stored target, by the identifier the removed property held);
        // everything else is decided without it, before the reference
        // validation. `changed_data_fields` is name-ordered, so the reported
        // property is deterministic.
        let contract_fetch_info = self.base().data_contract_fetch_info();
        let document_type_name = self.base().document_type_name();
        // V0 above has already refused an unknown document type.
        let document_type = contract_fetch_info
            .contract
            .document_type_for_name(document_type_name)?;
        let immutable_fields = document_type.immutable_fields();
        let conditions = document_type.immutable_field_conditions();
        if !immutable_fields.is_empty() || !conditions.is_empty() {
            let removed_identifier_fields = self.removed_identifier_fields();
            // Built on the first changed property a condition freezes
            let mut judged: Option<(Value, DocumentSystemValues)> = None;
            for property in self.changed_data_fields() {
                if let Some(condition) = conditions.get(property) {
                    let (data, system) = judged.get_or_insert_with(|| {
                        (
                            self.condition_data(),
                            self.condition_system_values(owner_id),
                        )
                    });
                    if !condition.holds(data, system).unwrap_or(true) {
                        continue;
                    }
                } else if !immutable_fields.contains(property) {
                    continue;
                } else {
                    let cleared_a_dead_reference = match removed_identifier_fields.get(property) {
                        Some(referenced_id) => {
                            self.base().deletable_document_reference_target_is_gone(
                                property,
                                *referenced_id,
                                platform,
                                block_info,
                                transaction,
                                execution_context,
                                platform_version,
                            )?
                        }
                        None => false,
                    };
                    if cleared_a_dead_reference {
                        continue;
                    }
                }
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
            owner_id,
            self.creator_id(),
            Some(self.changed_data_fields()),
            Some(self.stored_changed_values()),
            platform,
            block_info,
            // A replace consumes nothing: a lookup that could is judged on a create only
            &mut Vec::new(),
            None,
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

/// What a condition judging a replace is judged against: the `when` of an
/// `immutable` entry, here, and a type's `retractedWhen`, which the batch
/// transformer judges a barred owner's replace on.
pub(in crate::execution::validation::state_transition::state_transitions::batch) trait ReplaceConditionInputs
{
    /// The properties the replace writes, with the stored document's under
    /// `$old` for a condition reading them through `$old.`: the written ones,
    /// each changed one set back to its stored value, and an added one, which
    /// the stored document did not hold, left out.
    fn condition_data(&self) -> Value;

    /// The system values of the document the replace writes, as a
    /// `propertyConstraints` rule judges a replace: the stored creation and
    /// transfer, this block as the update, and `owner_id`, the writer.
    fn condition_system_values(&self, owner_id: Identifier) -> DocumentSystemValues;
}

impl ReplaceConditionInputs for DocumentReplaceTransitionAction {
    fn condition_data(&self) -> Value {
        let written = self.data();
        let stored_changed = self.stored_changed_values();
        let mut stored = written.clone();
        for property in self.changed_data_fields() {
            match stored_changed.get(property) {
                Some(value) => {
                    stored.insert(property.clone(), value.clone());
                }
                None => {
                    stored.remove(property);
                }
            }
        }
        let mut data: Vec<(Value, Value)> = written
            .iter()
            .map(|(property, value)| (Value::Text(property.clone()), value.clone()))
            .collect();
        data.push((
            Value::Text(STORED_DOCUMENT_KEY.to_string()),
            Value::Map(
                stored
                    .into_iter()
                    .map(|(property, value)| (Value::Text(property), value))
                    .collect(),
            ),
        ));
        Value::Map(data)
    }

    fn condition_system_values(&self, owner_id: Identifier) -> DocumentSystemValues {
        DocumentSystemValues {
            owner_id: Some(owner_id),
            created_at: self.created_at(),
            updated_at: self.updated_at(),
            transferred_at: self.transferred_at(),
            created_at_block_height: self.created_at_block_height(),
            updated_at_block_height: self.updated_at_block_height(),
            transferred_at_block_height: self.transferred_at_block_height(),
            created_at_core_block_height: self.created_at_core_block_height(),
            updated_at_core_block_height: self.updated_at_core_block_height(),
            transferred_at_core_block_height: self.transferred_at_core_block_height(),
            aggregates: None,
        }
    }
}
