use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_expired_error::DocumentExpiredError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::document::expiration::pricing::document_expires_at;
use drive::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::DocumentPurchaseTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::DocumentReplaceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::DocumentTransferTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::DocumentUpdatePriceTransitionActionAccessorsV0;
use drive::state_transition_action::batch::batched_transition::document_transition::DocumentTransitionAction;
use drive::state_transition_action::batch::batched_transition::BatchedTransitionAction;

/// Refuses an action on a document whose type declares a `ttl` once that has passed:
/// `$createdAt` plus the time to live at or before block time, the moment the cleanup after
/// the block's state transitions may delete it. The expiry is read from the document itself
/// and its type (drive's `document_expires_at`, the rule its expirations tree is keyed by),
/// never from the documents expirations tree, so it holds whether or not the cleanup has
/// reached the document yet.
///
/// Reachable from protocol version 14 only: `documents_ttl_seconds` is `Some` only on a
/// document type parsed from the `ttl` keyword, which earlier versions do not read.
///
/// `created_at` is the document's stored `$createdAt`, which every action on an existing
/// document carries unchanged.
pub(crate) fn validate_document_not_expired(
    contract_id: Identifier,
    document_type: DocumentTypeRef,
    document_id: Identifier,
    created_at: Option<TimestampMillis>,
    block_info: &BlockInfo,
) -> Result<SimpleConsensusValidationResult, Error> {
    let Some(ttl_seconds) = document_type.documents_ttl_seconds() else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    // The parser requires `$createdAt` on such a type and every write keeps it; a document
    // without one has no expiry to judge.
    let Some(created_at) = created_at else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    let expired_at = document_expires_at(created_at, ttl_seconds)?;
    if block_info.time_ms < expired_at {
        return Ok(SimpleConsensusValidationResult::new());
    }
    Ok(SimpleConsensusValidationResult::new_with_error(
        DocumentExpiredError::new(
            contract_id,
            document_type.name().clone(),
            document_id,
            expired_at,
            block_info.time_ms,
        )
        .into(),
    ))
}

/// [`validate_document_not_expired`] for one action of a batch: replacing, transferring,
/// buying and repricing an expired document are refused. A create makes a new document, and
/// its owner's deletion of an expired one only removes it sooner; token actions touch no
/// document.
pub(crate) fn validate_batched_action_not_expired(
    transition: &BatchedTransitionAction,
    block_info: &BlockInfo,
) -> Result<SimpleConsensusValidationResult, Error> {
    let BatchedTransitionAction::DocumentAction(document_action) = transition else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    let (base, created_at) = match document_action {
        DocumentTransitionAction::ReplaceAction(action) => (action.base(), action.created_at()),
        DocumentTransitionAction::TransferAction(action) => {
            (action.base(), action.document().created_at())
        }
        DocumentTransitionAction::PurchaseAction(action) => {
            (action.base(), action.document().created_at())
        }
        DocumentTransitionAction::UpdatePriceAction(action) => {
            (action.base(), action.document().created_at())
        }
        DocumentTransitionAction::CreateAction(_)
        | DocumentTransitionAction::DeleteAction(_)
        | DocumentTransitionAction::IndexOnlyDeleteAction(_) => {
            return Ok(SimpleConsensusValidationResult::new());
        }
    };
    let contract = &base.data_contract_fetch_info_ref().contract;
    // An unknown document type is refused by the action's own validation.
    let Some(document_type) = contract.document_type_optional_for_name(base.document_type_name())
    else {
        return Ok(SimpleConsensusValidationResult::new());
    };
    validate_document_not_expired(
        contract.id(),
        document_type,
        base.id(),
        created_at,
        block_info,
    )
}
