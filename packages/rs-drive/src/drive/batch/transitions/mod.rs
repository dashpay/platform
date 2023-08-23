// MIT LICENSE
//
// Copyright (c) 2022-2023 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Translation of State Transitions to Drive Operations
//!
//! This module defines general, commonly used functions in Drive.
//!

mod contract;
mod document;
mod identity;

use crate::drive::batch::DriveOperation;
use crate::error::Error;
use crate::state_transition_action::StateTransitionAction;
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

impl<'s> DriveHighLevelOperationConverter for StateTransitionAction {
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
        }
    }
}
