mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent::{
    PaidDriveEvent, PaidFromAssetLockDriveEvent,
};
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use dpp::identity::PartialIdentity;
use dpp::prelude::UserFeeIncrease;

use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use drive::drive::batch::transitions::DriveHighLevelOperationConverter;
use drive::drive::batch::DriveOperation;

/// An execution event
#[derive(Clone)]
pub(in crate::execution) enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// The removed balance in the case of a transfer or withdrawal
        removed_balance: Option<Credits>,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
        /// the fee multiplier that the user agreed to, 0 means 100% of the base fee, 1 means 101%
        user_fee_increase: UserFeeIncrease,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLockDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// The added balance
        added_balance: Credits,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// the execution operations that we must also pay for
        execution_operations: Vec<ValidationOperation>,
        /// the fee multiplier that the user agreed to, 0 means 100% of the base fee, 1 means 101%
        user_fee_increase: UserFeeIncrease,
    },
    /// A drive event that is free
    FreeDriveEvent {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
}

impl<'a> ExecutionEvent<'a> {
    pub(crate) fn create_from_state_transition_action(
        action: StateTransitionAction,
        identity: Option<PartialIdentity>,
        epoch: &Epoch,
        execution_context: StateTransitionExecutionContext,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match &action {
            StateTransitionAction::IdentityCreateAction(identity_create_action) => {
                let user_fee_increase = identity_create_action.user_fee_increase();
                let identity = identity_create_action.into();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(PaidFromAssetLockDriveEvent {
                    identity,
                    added_balance: 0,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_action) => {
                let user_fee_increase = identity_top_up_action.user_fee_increase();
                let added_balance = identity_top_up_action.top_up_balance_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidFromAssetLockDriveEvent {
                        identity,
                        added_balance,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(identity_credit_withdrawal) => {
                let user_fee_increase = identity_credit_withdrawal.user_fee_increase();
                let removed_balance = identity_credit_withdrawal.amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidDriveEvent {
                        identity,
                        removed_balance: Some(removed_balance),
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
            StateTransitionAction::IdentityCreditTransferAction(identity_credit_transfer) => {
                let user_fee_increase = identity_credit_transfer.user_fee_increase();
                let removed_balance = identity_credit_transfer.transfer_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidDriveEvent {
                        identity,
                        removed_balance: Some(removed_balance),
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
            _ => {
                let user_fee_increase = action.user_fee_increase();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidDriveEvent {
                        identity,
                        removed_balance: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
        }
    }
}
