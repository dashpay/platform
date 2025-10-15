/// document_base_transition_action
pub mod document_base_transition_action;
/// document_create_transition_action
pub mod document_create_transition_action;
/// document_delete_transition_action
pub mod document_delete_transition_action;
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
use crate::state_transition_action::batch::batched_transition::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::batch::batched_transition::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use crate::state_transition_action::batch::batched_transition::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
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
        }
    }
}
