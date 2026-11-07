/// transformer
pub mod transformer;
/// v0
pub mod v0;

use crate::state_transition_action::shielded::private_document_create_from_shielded_pool::v0::PrivateDocumentCreateFromShieldedPoolTransitionActionV0;
use crate::state_transition_action::shielded::ShieldedActionNote;
use derive_more::From;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Private document create from shielded pool transition action
#[derive(Debug, Clone, From)]
pub enum PrivateDocumentCreateFromShieldedPoolTransitionAction {
    /// v0
    V0(PrivateDocumentCreateFromShieldedPoolTransitionActionV0),
}

impl PrivateDocumentCreateFromShieldedPoolTransitionAction {
    /// The contract that defines the private document type
    pub fn data_contract_id(&self) -> Identifier {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => t.data_contract_id,
        }
    }
    /// The private document type the entries are appended to
    pub fn document_type_name(&self) -> &str {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => &t.document_type_name,
        }
    }
    /// The entries, in the order they are appended
    pub fn entries(&self) -> &[Vec<u8>] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => &t.entries,
        }
    }
    /// Notes (spent nullifiers plus change outputs)
    pub fn notes(&self) -> &[ShieldedActionNote] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => &t.notes,
        }
    }
    /// The anchor the spend was proven against
    pub fn anchor(&self) -> &[u8; 32] {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => &t.anchor,
        }
    }
    /// The fee, the whole amount leaving the pool, routed to the fee pools
    pub fn fee_amount(&self) -> Credits {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => t.fee_amount,
        }
    }
    /// Pool total balance read at transform time
    pub fn current_total_balance(&self) -> Credits {
        match self {
            PrivateDocumentCreateFromShieldedPoolTransitionAction::V0(t) => t.current_total_balance,
        }
    }
}
