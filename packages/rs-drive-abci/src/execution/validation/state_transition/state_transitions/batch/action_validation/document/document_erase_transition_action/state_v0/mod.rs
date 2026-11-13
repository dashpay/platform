use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::state::fetch_keep_history_document_lifecycle::fetch_keep_history_document_lifecycle;
use crate::platform_types::platform::PlatformStateRef;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
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
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;
use drive::state_transition_action::StateTransitionAction;
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

/// What check tx reserves, on top of the estimated fee of `action`, for the
/// erases of a batch whose state it does not validate: for every erase, the
/// work its refunds cause for other identities
/// ([`Drive::erase_refund_recipient_cost`]), which state validation 0 bills on
/// every path an erase takes. Nothing for anything else. Without it, a signer
/// covering an erase's estimated fee but not that work would be admitted, then
/// refused unpaid in a block, its nonce left to replay the work. A state
/// validation generation that bills an erase otherwise has to reserve what it
/// bills, so any other generation is refused here.
pub(in crate::execution::validation::state_transition) fn erase_refund_recipient_reservation(
    action: &StateTransitionAction,
    drive: &Drive,
    epoch: &Epoch,
    platform_version: &PlatformVersion,
) -> Result<Vec<ValidationOperation>, Error> {
    let StateTransitionAction::BatchAction(batch) = action else {
        return Ok(Vec::new());
    };
    let erases = batch
        .transitions()
        .iter()
        .filter(|transition| {
            matches!(
                transition,
                BatchedTransitionAction::DocumentAction(DocumentTransitionAction::EraseAction(_))
            )
        })
        .count();
    if erases == 0 {
        return Ok(Vec::new());
    }
    match platform_version
        .drive_abci
        .validation_and_processing
        .state_transitions
        .batch_state_transition
        .document_erase_transition_state_validation
    {
        Some(0) => {}
        Some(version) => {
            return Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "erase_refund_recipient_reservation".to_string(),
                known_versions: vec![0],
                received: version,
            }))
        }
        None => {
            return Err(Error::Execution(ExecutionError::VersionNotActive {
                method: "erase_refund_recipient_reservation".to_string(),
                known_versions: vec![0],
            }))
        }
    }
    let cost = drive
        .erase_refund_recipient_cost(epoch, platform_version)
        .map_err(Error::Drive)?;
    Ok(vec![
        ValidationOperation::PrecalculatedOperation(cost);
        erases
    ])
}
