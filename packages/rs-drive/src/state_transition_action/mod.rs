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
use crate::state_transition_action::system::bump_identity_data_contract_nonce_action::{
    BumpIdentityDataContractNonceAction, BumpIdentityDataContractNonceActionAccessorsV0,
};
use crate::state_transition_action::system::bump_identity_nonce_action::{
    BumpIdentityNonceAction, BumpIdentityNonceActionAccessorsV0,
};
use crate::state_transition_action::system::partially_use_asset_lock_action::{
    PartiallyUseAssetLockAction, PartiallyUseAssetLockActionAccessorsV0,
};
use derive_more::From;
use dpp::prelude::UserFeeIncrease;

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
    /// partially use the asset lock for funding invalid asset lock transactions like
    /// identity top up and identity create
    PartiallyUseAssetLockAction(PartiallyUseAssetLockAction),
}

impl StateTransitionAction {
    /// The fee multiplier for the action
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            StateTransitionAction::DataContractCreateAction(action) => action.user_fee_increase(),
            StateTransitionAction::DataContractUpdateAction(action) => action.user_fee_increase(),
            StateTransitionAction::DocumentsBatchAction(action) => action.user_fee_increase(),
            StateTransitionAction::IdentityCreateAction(action) => action.user_fee_increase(),
            StateTransitionAction::IdentityTopUpAction(action) => action.user_fee_increase(),
            StateTransitionAction::IdentityCreditWithdrawalAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::IdentityUpdateAction(action) => action.user_fee_increase(),
            StateTransitionAction::IdentityCreditTransferAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::BumpIdentityNonceAction(action) => action.user_fee_increase(),
            StateTransitionAction::BumpIdentityDataContractNonceAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::PartiallyUseAssetLockAction(action) => {
                action.user_fee_increase()
            }
        }
    }
}
