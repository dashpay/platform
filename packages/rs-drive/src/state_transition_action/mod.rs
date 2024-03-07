/// contract
pub mod contract;
/// documents
pub mod document;
/// identity
pub mod identity;

/// system
pub mod system;

use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::document::documents_batch::DocumentsBatchTransitionAction;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0};
use crate::state_transition_action::system::bump_identity_nonce_action::{BumpIdentityNonceAction, BumpIdentityNonceActionAccessorsV0};
use derive_more::From;
use dpp::prelude::FeeMultiplier;

/// ST action
#[derive(Debug, Clone, From)]
pub enum StateTransitionAction {
    /// data contract create
    DataContractCreateAction(DataContractCreateTransitionAction),
    /// data contract update
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    /// documents batch
    DocumentsBatchAction(DocumentsBatchTransitionAction),
    /// identity create
    IdentityCreateAction(IdentityCreateTransitionAction),
    /// identity topup
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    /// identity credit withdrawal
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    /// identity update
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    /// identity credit transfer
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
    /// bump identity nonce action
    /// this can only come in this form from identity state transitions that do not use asset locks
    /// it will also only happen if the state validation fails
    BumpIdentityNonceAction(BumpIdentityNonceAction),
    /// bump identity contract nonce action
    /// this can only come in this form from the document contract update state transition
    /// it will also only happen if the state validation fails
    BumpIdentityDataContractNonceAction(BumpIdentityDataContractNonceAction),
}

impl StateTransitionAction {
    /// The fee multiplier for the action
    pub fn fee_multiplier(&self) -> FeeMultiplier {
        match self {
            StateTransitionAction::DataContractCreateAction(action) => action.fee_multiplier(),
            StateTransitionAction::DataContractUpdateAction(action) => action.fee_multiplier(),
            StateTransitionAction::DocumentsBatchAction(action) => action.fee_multiplier(),
            StateTransitionAction::IdentityCreateAction(action) => action.fee_multiplier(),
            StateTransitionAction::IdentityTopUpAction(action) => action.fee_multiplier(),
            StateTransitionAction::IdentityCreditWithdrawalAction(action) => action.fee_multiplier(),
            StateTransitionAction::IdentityUpdateAction(action) => action.fee_multiplier(),
            StateTransitionAction::IdentityCreditTransferAction(action) => action.fee_multiplier(),
            StateTransitionAction::BumpIdentityNonceAction(action) => action.fee_multiplier(),
            StateTransitionAction::BumpIdentityDataContractNonceAction(action) => action.fee_multiplier(),
        }
    }
}
