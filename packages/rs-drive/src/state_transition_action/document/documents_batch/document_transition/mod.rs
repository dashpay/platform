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
/// token_base_transition_action
pub mod token_base_transition_action;
/// token_burn_transition_action
pub mod token_burn_transition_action;
/// token_issuance_transition_action
pub mod token_mint_transition_action;
/// token_transfer_transition_action
pub mod token_transfer_transition_action;

pub use dpp::state_transition::batch_transition::batched_transition::document_transition_action_type::DocumentTransitionActionType;

use derive_more::From;
use dpp::identifier::Identifier;
use crate::state_transition_action::document::documents_batch::document_transition::document_base_transition_action::{DocumentBaseTransitionAction, DocumentBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_create_transition_action::{DocumentCreateTransitionAction, DocumentCreateTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::DocumentDeleteTransitionAction;
use crate::state_transition_action::document::documents_batch::document_transition::document_replace_transition_action::{DocumentReplaceTransitionAction, DocumentReplaceTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_delete_transition_action::v0::DocumentDeleteTransitionActionAccessorsV0;
use crate::state_transition_action::document::documents_batch::document_transition::document_purchase_transition_action::{DocumentPurchaseTransitionAction, DocumentPurchaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_transfer_transition_action::{DocumentTransferTransitionAction, DocumentTransferTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::document_update_price_transition_action::{DocumentUpdatePriceTransitionAction, DocumentUpdatePriceTransitionActionAccessorsV0};
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_base_transition_action::{TokenBaseTransitionAction, TokenBaseTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_burn_transition_action::{TokenBurnTransitionAction, TokenBurnTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_mint_transition_action::{TokenMintTransitionAction, TokenMintTransitionActionAccessorsV0};
use crate::state_transition_action::document::documents_batch::document_transition::token_transfer_transition_action::{TokenTransferTransitionAction, TokenTransferTransitionActionAccessors};

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

/// token action
#[derive(Debug, Clone, From)]
pub enum TokenTransitionAction {
    /// burn
    BurnAction(TokenBurnTransitionAction),
    /// issuance
    MintAction(TokenMintTransitionAction),
    /// transfer
    TransferAction(TokenTransferTransitionAction),
}

impl TokenTransitionAction {
    /// Returns a reference to the base token transition action if available
    pub fn base(&self) -> &TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base(),
            TokenTransitionAction::MintAction(action) => action.base(),
            TokenTransitionAction::TransferAction(action) => action.base(),
        }
    }

    /// Consumes self and returns the base token transition action if available
    pub fn base_owned(self) -> TokenBaseTransitionAction {
        match self {
            TokenTransitionAction::BurnAction(action) => action.base_owned(),
            TokenTransitionAction::MintAction(action) => action.base_owned(),
            TokenTransitionAction::TransferAction(action) => action.base_owned(),
        }
    }
}

/// token action
#[derive(Debug, Clone, From)]
pub enum BatchedTransitionAction {
    /// document
    DocumentAction(DocumentTransitionAction),
    /// token
    TokenAction(TokenTransitionAction),
    /// bump identity data contract nonce
    BumpIdentityDataContractNonce(BumpIdentityDataContractNonceAction),
}

impl BatchedTransitionAction {
    /// Helper method to get the data contract id
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            BatchedTransitionAction::DocumentAction(document_action) => {
                document_action.base().data_contract_id()
            }
            BatchedTransitionAction::TokenAction(token_action) => {
                token_action.base().data_contract_id()
            }
            BatchedTransitionAction::BumpIdentityDataContractNonce(bump_action) => {
                bump_action.data_contract_id()
            }
        }
    }
}
