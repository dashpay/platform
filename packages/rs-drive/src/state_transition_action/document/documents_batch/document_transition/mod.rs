mod document_transition_action_type;
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
/// document_update_price_transition_action
pub mod document_update_price_transition_action;
/// token_base_transition_action
pub mod token_base_transition_action;
/// token_burn_transition_action
pub mod token_burn_transition_action;
/// token_issuance_transition_action
pub mod token_issuance_transition_action;
/// token_transfer_transition_action
pub mod token_transfer_transition_action;

pub use dpp::state_transition::documents_batch_transition::document_transition::document_transition_action_type::DocumentTransitionActionType;

use derive_more::From;

use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::DocumentBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0};
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;

/// version
pub const DOCUMENT_TRANSITION_ACTION_VERSION: u32 = 0;

/// action
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
    /// bump identity data contract nonce
    BumpIdentityDataContractNonce(BumpIdentityDataContractNonceAction),
}

impl DocumentTransitionAction {
    /// base
    pub fn base(&self) -> Option<&DocumentBaseTransitionAction> {
        match self {
            DocumentTransitionAction::CreateAction(d) => Some(d.base()),
            DocumentTransitionAction::DeleteAction(d) => Some(d.base()),
            DocumentTransitionAction::ReplaceAction(d) => Some(d.base()),
            DocumentTransitionAction::TransferAction(d) => Some(d.base()),
            DocumentTransitionAction::PurchaseAction(d) => Some(d.base()),
            DocumentTransitionAction::UpdatePriceAction(d) => Some(d.base()),
            DocumentTransitionAction::BumpIdentityDataContractNonce(_) => None,
        }
    }

    /// base owned
    pub fn base_owned(self) -> Option<DocumentBaseTransitionAction> {
        match self {
            DocumentTransitionAction::CreateAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::DeleteAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::ReplaceAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::TransferAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::PurchaseAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::UpdatePriceAction(d) => Some(d.base_owned()),
            DocumentTransitionAction::BumpIdentityDataContractNonce(_) => None,
        }
    }
}


use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::TokenBaseTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_issuance_transition_action::{TokenIssuanceTransitionAction, TokenIssuanceTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::{TokenTransferTransitionAction, TokenTransferTransitionActionAccessors, TokenTransferTransitionActionAccessorsV0};

/// token action
#[derive(Debug, Clone, From)]
pub enum TokenTransitionAction {
    /// burn
    BurnAction(TokenBurnTransitionAction),
    /// issuance
    IssuanceAction(TokenIssuanceTransitionAction),
    /// transfer
    TransferAction(TokenTransferTransitionAction),
}

impl TokenTransitionAction {
    /// Returns a reference to the base token transition action if available
    pub fn base(&self) -> Option<&TokenBaseTransitionAction> {
        match self {
            TokenTransitionAction::BurnAction(action) => Some(action.base()),
            TokenTransitionAction::IssuanceAction(action) => Some(action.base()),
            TokenTransitionAction::TransferAction(action) => Some(action.base()),
        }
    }

    /// Consumes self and returns the base token transition action if available
    pub fn base_owned(self) -> Option<TokenBaseTransitionAction> {
        match self {
            TokenTransitionAction::BurnAction(action) => Some(action.base_owned()),
            TokenTransitionAction::IssuanceAction(action) => Some(action.base_owned()),
            TokenTransitionAction::TransferAction(action) => Some(action.base_owned()),
        }
    }
}