/// contract
pub mod contract;
/// identity
pub mod identity;

/// system
pub mod system;
// TODO: Must crate only but we need to remove of use it first
pub mod action_convert_to_operations;
/// address funds
pub mod address_funds;
/// documents_batch
pub mod batch;
/// shielded
pub mod shielded;

use crate::state_transition_action::address_funds::address_credit_withdrawal::AddressCreditWithdrawalTransitionAction;
use crate::state_transition_action::address_funds::address_funding_from_asset_lock::AddressFundingFromAssetLockTransitionAction;
use crate::state_transition_action::address_funds::address_funds_transfer::AddressFundsTransferTransitionAction;
use crate::state_transition_action::batch::BatchTransitionAction;
use crate::state_transition_action::contract::data_contract_create::DataContractCreateTransitionAction;
use crate::state_transition_action::contract::data_contract_update::DataContractUpdateTransitionAction;
use crate::state_transition_action::identity::identity_create::IdentityCreateTransitionAction;
use crate::state_transition_action::identity::identity_create_from_addresses::IdentityCreateFromAddressesTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer::IdentityCreditTransferTransitionAction;
use crate::state_transition_action::identity::identity_credit_transfer_to_addresses::IdentityCreditTransferToAddressesTransitionAction;
use crate::state_transition_action::identity::identity_credit_withdrawal::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition_action::identity::identity_topup::IdentityTopUpTransitionAction;
use crate::state_transition_action::identity::identity_topup_from_addresses::IdentityTopUpFromAddressesTransitionAction;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use crate::state_transition_action::identity::masternode_vote::MasternodeVoteTransitionAction;
use crate::state_transition_action::shielded::shield::ShieldTransitionAction;
use crate::state_transition_action::shielded::shield_from_asset_lock::ShieldFromAssetLockTransitionAction;
use crate::state_transition_action::shielded::shielded_transfer::ShieldedTransferTransitionAction;
use crate::state_transition_action::shielded::shielded_withdrawal::ShieldedWithdrawalTransitionAction;
use crate::state_transition_action::shielded::unshield::UnshieldTransitionAction;
use crate::state_transition_action::system::bump_address_input_nonces_action::{
    BumpAddressInputNonceActionAccessorsV0, BumpAddressInputNoncesAction,
};
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
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, From)]
pub enum StateTransitionAction {
    /// data contract create
    DataContractCreateAction(DataContractCreateTransitionAction),
    /// data contract update
    DataContractUpdateAction(DataContractUpdateTransitionAction),
    /// batch
    BatchAction(BatchTransitionAction),
    /// identity create
    IdentityCreateAction(IdentityCreateTransitionAction),
    /// identity create from addresses
    IdentityCreateFromAddressesAction(IdentityCreateFromAddressesTransitionAction),
    /// identity topup
    IdentityTopUpAction(IdentityTopUpTransitionAction),
    /// identity topup from addresses
    IdentityTopUpFromAddressesAction(IdentityTopUpFromAddressesTransitionAction),
    /// identity credit withdrawal
    IdentityCreditWithdrawalAction(IdentityCreditWithdrawalTransitionAction),
    /// identity update
    IdentityUpdateAction(IdentityUpdateTransitionAction),
    /// identity credit transfer
    IdentityCreditTransferAction(IdentityCreditTransferTransitionAction),
    /// identity credit transfer to addresses
    IdentityCreditTransferToAddressesAction(IdentityCreditTransferToAddressesTransitionAction),
    /// address funds transfer
    AddressFundsTransfer(AddressFundsTransferTransitionAction),
    /// address credit withdrawal
    AddressCreditWithdrawal(AddressCreditWithdrawalTransitionAction),
    /// address funding from asset lock
    AddressFundingFromAssetLock(AddressFundingFromAssetLockTransitionAction),
    /// masternode vote action
    MasternodeVoteAction(MasternodeVoteTransitionAction),
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
    /// partially use the asset lock for funding invalid asset lock transactions like
    /// identity top up and identity create
    BumpAddressInputNoncesAction(BumpAddressInputNoncesAction),
    /// shield (address -> shielded pool)
    ShieldAction(ShieldTransitionAction),
    /// shielded transfer (shielded -> shielded)
    ShieldedTransferAction(ShieldedTransferTransitionAction),
    /// unshield (shielded pool -> address)
    UnshieldAction(UnshieldTransitionAction),
    /// shield from asset lock (L1 asset lock -> shielded pool)
    ShieldFromAssetLockAction(ShieldFromAssetLockTransitionAction),
    /// shielded withdrawal (shielded pool -> L1 core address)
    ShieldedWithdrawalAction(ShieldedWithdrawalTransitionAction),
}

impl StateTransitionAction {
    /// The fee multiplier for the action
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        match self {
            StateTransitionAction::DataContractCreateAction(action) => action.user_fee_increase(),
            StateTransitionAction::DataContractUpdateAction(action) => action.user_fee_increase(),
            StateTransitionAction::BatchAction(action) => action.user_fee_increase(),
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
            StateTransitionAction::MasternodeVoteAction(_) => {
                UserFeeIncrease::default() // 0 (or none)
            }
            StateTransitionAction::IdentityCreateFromAddressesAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::IdentityTopUpFromAddressesAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::IdentityCreditTransferToAddressesAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::AddressFundsTransfer(action) => action.user_fee_increase(),
            StateTransitionAction::AddressCreditWithdrawal(action) => action.user_fee_increase(),
            StateTransitionAction::AddressFundingFromAssetLock(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::BumpAddressInputNoncesAction(action) => {
                action.user_fee_increase()
            }
            StateTransitionAction::ShieldAction(action) => action.user_fee_increase(),
            StateTransitionAction::ShieldedTransferAction(action) => action.user_fee_increase(),
            StateTransitionAction::UnshieldAction(action) => action.user_fee_increase(),
            StateTransitionAction::ShieldFromAssetLockAction(action) => action.user_fee_increase(),
            StateTransitionAction::ShieldedWithdrawalAction(action) => action.user_fee_increase(),
        }
    }
}
