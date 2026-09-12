use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_keep_history_document_lifecycle;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{
    InvalidDocumentTransitionActionError, InvalidDocumentTypeError,
};
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::validation::SimpleConsensusValidationResult;
use dpp::version::PlatformVersion;
use drive::drive::document::lifecycle::DocumentLifecycleState;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::v0::DocumentEraseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_erase_transition_action::DocumentEraseTransitionAction;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentEraseTransitionActionStateValidationV0 {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error>;
}

impl DocumentEraseTransitionActionStateValidationV0 for DocumentEraseTransitionAction {
    /// An erasure is authorized once, when it starts.
    ///
    /// A document that is merely deleted may only be committed to erasure by
    /// its owner. Once committed, the record left in state is itself the
    /// evidence that destruction was authorized, so any identity may submit the
    /// remaining chunks and pay for them: an owner who loses their keys, their
    /// funds or their permission can no longer strand a half-erased document.
    /// A document that is still current cannot be erased at all — deleting it
    /// is a separate intent, with its own permission and its own token cost.
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let validation_result = self.base().validate_state(
            platform,
            owner_id,
            block_info,
            "erase",
            execution_context,
            transaction,
            platform_version,
        )?;
        if !validation_result.is_valid() {
            return Ok(validation_result);
        }

        let contract_fetch_info = self.base().data_contract_fetch_info();
        let contract = &contract_fetch_info.contract;
        let document_type_name = self.base().document_type_name();
        let Some(document_type) = contract.document_type_optional_for_name(document_type_name)
        else {
            return Ok(SimpleConsensusValidationResult::new_with_error(
                InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
            ));
        };

        let lifecycle = fetch_keep_history_document_lifecycle(
            platform.drive,
            contract,
            document_type,
            self.base().id(),
            &block_info.epoch,
            execution_context,
            transaction,
            platform_version,
        )?;

        // Removing revisions credits whoever paid for each of them, and those
        // balance updates are applied after this transition's fee result is
        // formed, against identities that had nothing to do with it. The chunk
        // bound limits how many there can be; this is what pays for them. It is
        // billed on every path the erase can take, including the refusals,
        // because the estimate that admits the transition is formed before the
        // lifecycle is known.
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
            platform
                .drive
                .erase_refund_recipient_cost(&block_info.epoch, platform_version)
                .map_err(Error::Drive)?,
        ));

        match lifecycle {
            DocumentLifecycleState::Active(_) => {
                Ok(SimpleConsensusValidationResult::new_with_error(
                    InvalidDocumentTransitionActionError::new(format!(
                        "document {} must be deleted before it can be erased",
                        self.base().id()
                    ))
                    .into(),
                ))
            }
            // The single ownership call site for an erase.
            DocumentLifecycleState::Deleted(newest) => {
                if newest.owner_id() != owner_id {
                    return Ok(SimpleConsensusValidationResult::new_with_error(
                        ConsensusError::StateError(StateError::DocumentOwnerIdMismatchError(
                            DocumentOwnerIdMismatchError::new(
                                self.base().id(),
                                owner_id,
                                newest.owner_id(),
                            ),
                        )),
                    ));
                }
                Ok(SimpleConsensusValidationResult::new())
            }
            // A continuation of an erasure that is already committed: nothing
            // to authorize, because the committed record already carries the
            // authorization.
            DocumentLifecycleState::Erasing => Ok(SimpleConsensusValidationResult::new()),
            DocumentLifecycleState::Absent => Ok(SimpleConsensusValidationResult::new_with_error(
                ConsensusError::StateError(StateError::DocumentNotFoundError(
                    DocumentNotFoundError::new(self.base().id()),
                )),
            )),
        }
    }
}
