/// document_base_transition_action
pub mod document_base_transition_action;
/// document_create_transition_action
pub mod document_create_transition_action;
/// document_delete_transition_action
pub mod document_delete_transition_action;
/// document_index_only_delete_transition_action
pub mod document_index_only_delete_transition_action;
/// document_purchase_transition_action
pub mod document_purchase_transition_action;
/// document_replace_transition_action
pub mod document_replace_transition_action;
/// document_transfer_transition_action
pub mod document_transfer_transition_action;
mod document_transition_action_type;
/// document_update_price_transition_action
pub mod document_update_price_transition_action;

pub use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;

use derive_more::From;
use dpp::platform_value::Value;
use std::collections::{BTreeMap, BTreeSet};
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::v0::DocumentIndexOnlyDeleteTransitionActionAccessorsV0;
use crate::state_transition_action::batch::batched_transition::document_transition::document_index_only_delete_transition_action::DocumentIndexOnlyDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0};

/// version
pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

/// action
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, From)]
pub enum DocumentTransitionAction {
    /// create
    CreateAction(DocumentCreateTransitionAction),
    /// replace
    ReplaceAction(DocumentReplaceTransitionAction),
    /// delete
    DeleteAction(DocumentDeleteTransitionAction),
    /// transfer
    TransferAction(DocumentTransferTransitionAction),
    /// purchase
    PurchaseAction(DocumentPurchaseTransitionAction),
    /// update price
    UpdatePriceAction(DocumentUpdatePriceTransitionAction),
    /// indexOnly delete-by-values — carries the document's property
    /// values, since there is no primary-storage row to fetch them from
    IndexOnlyDeleteAction(DocumentIndexOnlyDeleteTransitionAction),
}

impl DocumentTransitionAction {
    /// base
    pub fn base(&self) -> &DocumentBaseTransitionAction {
        match self {
            DocumentTransitionAction::CreateAction(d) => d.base(),
            DocumentTransitionAction::DeleteAction(d) => d.base(),
            DocumentTransitionAction::ReplaceAction(d) => d.base(),
            DocumentTransitionAction::TransferAction(d) => d.base(),
            DocumentTransitionAction::PurchaseAction(d) => d.base(),
            DocumentTransitionAction::UpdatePriceAction(d) => d.base(),
            DocumentTransitionAction::IndexOnlyDeleteAction(d) => d.base(),
        }
    }

    /// The action's name as the document type's token costs and consensus errors spell it
    pub fn action_name(&self) -> &'static str {
        match self {
            DocumentTransitionAction::CreateAction(_) => "create",
            DocumentTransitionAction::ReplaceAction(_) => "replace",
            DocumentTransitionAction::DeleteAction(_)
            | DocumentTransitionAction::IndexOnlyDeleteAction(_) => "delete",
            DocumentTransitionAction::TransferAction(_) => "transfer",
            DocumentTransitionAction::PurchaseAction(_) => "purchase",
            DocumentTransitionAction::UpdatePriceAction(_) => "update_price",
        }
    }

    /// base owned
    pub fn base_owned(self) -> DocumentBaseTransitionAction {
        match self {
            DocumentTransitionAction::CreateAction(d) => d.base_owned(),
            DocumentTransitionAction::DeleteAction(d) => d.base_owned(),
            DocumentTransitionAction::ReplaceAction(d) => d.base_owned(),
            DocumentTransitionAction::TransferAction(d) => d.base_owned(),
            DocumentTransitionAction::PurchaseAction(d) => d.base_owned(),
            DocumentTransitionAction::UpdatePriceAction(d) => d.base_owned(),
            DocumentTransitionAction::IndexOnlyDeleteAction(d) => d.base_owned(),
        }
    }
}

/// Drops the values of `transient_fields` from a document's `data`, by
/// top-level name: a transient property is judged on the transition and never
/// stored. The create action and, from protocol version 14, the replace action
/// both go through here, so they store the same data.
pub(crate) fn drop_transient_values(
    data: &mut BTreeMap<String, Value>,
    transient_fields: &BTreeSet<String>,
) {
    if !transient_fields.is_empty() {
        data.retain(|key, _| !transient_fields.contains(key));
    }
}
