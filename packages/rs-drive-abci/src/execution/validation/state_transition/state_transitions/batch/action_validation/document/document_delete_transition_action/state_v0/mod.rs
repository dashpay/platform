use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::InvalidDocumentTypeError;
use dpp::consensus::ConsensusError;
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::document::document_owner_id_mismatch_error::DocumentOwnerIdMismatchError;
use dpp::consensus::state::state_error::StateError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters};
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::validation::SimpleConsensusValidationResult;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use dpp::version::PlatformVersion;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::action_validation::document::document_base_transaction_action::DocumentBaseTransitionActionValidation;
use crate::execution::validation::state_transition::batch::state::v0::fetch_documents::fetch_document_with_id;
use crate::platform_types::platform::PlatformStateRef;

pub(in crate::execution::validation::state_transition::state_transitions::batch::action_validation) trait DocumentDeleteTransitionActionStateValidationV0 {
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
impl DocumentDeleteTransitionActionStateValidationV0 for DocumentDeleteTransitionAction {
    fn validate_state_v0(
        &self,
        platform: &PlatformStateRef,
        owner_id: Identifier,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, Error> {
        let owned = fetch_owned_document(
            self,
            platform,
            owner_id,
            block_info,
            execution_context,
            transaction,
            platform_version,
        )?;
        Ok(SimpleConsensusValidationResult::new_with_errors(
            owned.errors,
        ))
    }
}

/// The checks of state validation 0, in its order: the base transition, the document type, the
/// stored document (a billed fetch) and its owner. The stored document when every check passes,
/// the first refusal otherwise. Shared with state validation 1, which judges the type's delete
/// rules on the document it returns; split out of version 0 with its outcome and its billing
/// unchanged.
pub(super) fn fetch_owned_document(
    action: &DocumentDeleteTransitionAction,
    platform: &PlatformStateRef,
    owner_id: Identifier,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    transaction: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<Document>, Error> {
    let validation_result = action.base().validate_state(
        platform,
        owner_id,
        block_info,
        "delete",
        execution_context,
        transaction,
        platform_version,
    )?;
    if !validation_result.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(
            validation_result.errors,
        ));
    }

    let contract_fetch_info = action.base().data_contract_fetch_info();

    let contract = &contract_fetch_info.contract;

    let document_type_name = action.base().document_type_name();

    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        return Ok(ConsensusValidationResult::new_with_error(
            InvalidDocumentTypeError::new(document_type_name.clone(), contract.id()).into(),
        ));
    };

    // TODO: Use multi get https://github.com/facebook/rocksdb/wiki/MultiGet-Performance
    // `fetch_document_with_id` bills internally on transform_into_action: 1+.
    // PV11 byte-safe: v0 forces epoch=None inside, no add_operation,
    // same net effect as pre-PR's explicit zero-fee add_operation call.
    let original_document = fetch_document_with_id(
        platform.drive,
        contract,
        document_type,
        action.base().id(),
        &block_info.epoch,
        execution_context,
        transaction,
        platform_version,
    )?;

    let Some(document) = original_document else {
        return Ok(ConsensusValidationResult::new_with_error(
            ConsensusError::StateError(StateError::DocumentNotFoundError(
                DocumentNotFoundError::new(action.base().id()),
            )),
        ));
    };

    let ownership = check_ownership(action, &document, &owner_id);
    if !ownership.is_valid() {
        return Ok(ConsensusValidationResult::new_with_errors(ownership.errors));
    }
    Ok(ConsensusValidationResult::new_with_data(document))
}

/// A [`DocumentOwnerIdMismatchError`] unless `owner_id` owns `fetched_document`.
fn check_ownership(
    document_transition: &DocumentDeleteTransitionAction,
    fetched_document: &Document,
    owner_id: &Identifier,
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    if fetched_document.owner_id() != owner_id {
        result.add_error(ConsensusError::StateError(
            StateError::DocumentOwnerIdMismatchError(DocumentOwnerIdMismatchError::new(
                document_transition.base().id(),
                owner_id.to_owned(),
                fetched_document.owner_id(),
            )),
        ));
    }
    result
}
