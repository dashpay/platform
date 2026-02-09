//! Translation of State Transitions to Drive Operations
//!
//! This module defines general, commonly used functions in Drive.
//!

mod address_funds;
mod batch;
mod contract;
mod identity;
mod shielded;
mod system;

use crate::error::Error;
use crate::state_transition_action::StateTransitionAction;
use crate::util::batch::DriveOperation;
use dpp::block::epoch::Epoch;
use dpp::version::PlatformVersion;

/// A converter that will get High Level Drive Operations from State transitions
pub trait DriveHighLevelOperationConverter {
    /// This will get a list of atomic drive operations from a high level operations
    fn into_high_level_drive_operations<'a>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error>;
}

impl DriveHighLevelOperationConverter for StateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match self {
            StateTransitionAction::DataContractCreateAction(data_contract_create_transition) => {
                data_contract_create_transition
                    .into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::DataContractUpdateAction(data_contract_update_transition) => {
                data_contract_update_transition
                    .into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::BatchAction(documents_batch_transition) => {
                documents_batch_transition.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::IdentityCreateAction(identity_create_transition) => {
                identity_create_transition.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_transition) => {
                identity_top_up_transition.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(
                identity_credit_withdrawal_transition,
            ) => identity_credit_withdrawal_transition
                .into_high_level_drive_operations(epoch, platform_version),
            StateTransitionAction::IdentityUpdateAction(identity_update_transition) => {
                identity_update_transition.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::IdentityCreditTransferAction(
                identity_credit_transfer_transition,
            ) => identity_credit_transfer_transition
                .into_high_level_drive_operations(epoch, platform_version),
            StateTransitionAction::MasternodeVoteAction(masternode_vote_transition) => {
                masternode_vote_transition.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::BumpIdentityNonceAction(bump_identity_nonce_transition) => {
                bump_identity_nonce_transition
                    .into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                bump_identity_data_contract_nonce_transition,
            ) => bump_identity_data_contract_nonce_transition
                .into_high_level_drive_operations(epoch, platform_version),
            StateTransitionAction::PartiallyUseAssetLockAction(
                partially_used_asset_lock_action,
            ) => partially_used_asset_lock_action
                .into_high_level_drive_operations(epoch, platform_version),
            StateTransitionAction::IdentityCreateFromAddressesAction(
                identity_create_from_addresses_transition,
            ) => identity_create_from_addresses_transition
                .into_high_level_drive_operations(epoch, platform_version),

            StateTransitionAction::IdentityTopUpFromAddressesAction(
                identity_top_up_from_addresses_transition,
            ) => identity_top_up_from_addresses_transition
                .into_high_level_drive_operations(epoch, platform_version),

            StateTransitionAction::IdentityCreditTransferToAddressesAction(
                identity_credit_transfer_to_addresses_transition,
            ) => identity_credit_transfer_to_addresses_transition
                .into_high_level_drive_operations(epoch, platform_version),

            StateTransitionAction::AddressFundsTransfer(address_funds_transfer_transition) => {
                address_funds_transfer_transition
                    .into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::BumpAddressInputNoncesAction(
                bump_address_input_nonces_action,
            ) => bump_address_input_nonces_action
                .into_high_level_drive_operations(epoch, platform_version),
            StateTransitionAction::AddressCreditWithdrawal(address_credit_withdrawal) => {
                address_credit_withdrawal.into_high_level_drive_operations(epoch, platform_version)
            }

            StateTransitionAction::AddressFundingFromAssetLock(address_funding_from_asset_lock) => {
                address_funding_from_asset_lock
                    .into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::ShieldAction(shield_action) => {
                shield_action.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::ShieldedTransferAction(shielded_transfer_action) => {
                shielded_transfer_action.into_high_level_drive_operations(epoch, platform_version)
            }
            StateTransitionAction::UnshieldAction(unshield_action) => {
                unshield_action.into_high_level_drive_operations(epoch, platform_version)
            }
        }
    }
}
