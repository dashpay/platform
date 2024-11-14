//! Translation of State Transitions to Drive Operations
//!
//! This module defines general, commonly used functions in Drive.
//!

mod contract;
mod document;
mod identity;
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
            StateTransitionAction::DocumentsBatchAction(documents_batch_transition) => {
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
        }
    }
}
