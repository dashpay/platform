mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use dpp::asset_lock::reduced_asset_lock_value::AssetLockValueGettersV0;
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
use drive::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use drive::state_transition_action::system::partially_use_asset_lock_action::PartiallyUseAssetLockActionAccessorsV0;
use drive::util::batch::DriveOperation;

/// An execution event
#[derive(Clone)]
pub(in crate::execution) enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    Paid {
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
    /// A drive event that has a fixed cost that will be taken out in the operations
    PaidFixedCost {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        /// fees to add
        fees_to_add_to_pool: Credits,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLock {
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
    /// A drive event that is paid from an asset lock
    PaidFromAssetLockWithoutIdentity {
        /// The processing fees that should be distributed to validators
        processing_fees: Credits,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
    /// A drive event that is free
    #[allow(dead_code)] // TODO investigate why `variant `Free` is never constructed`
    Free {
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
                Ok(ExecutionEvent::PaidFromAssetLock {
                    identity,
                    added_balance: 0,
                    operations,
                    execution_operations: execution_context.operations_consume(),
                    user_fee_increase,
                })
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_action) => {
                let user_fee_increase = identity_top_up_action.user_fee_increase();
                let added_balance = identity_top_up_action
                    .top_up_asset_lock_value()
                    .remaining_credit_value();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::PaidFromAssetLock {
                        identity,
                        added_balance,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity top up action",
                    )))
                }
            }
            StateTransitionAction::PartiallyUseAssetLockAction(partially_used_asset_lock) => {
                let used_credits = partially_used_asset_lock.used_credits();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                // We mark it as a free operation because the event itself is paying for itself
                Ok(ExecutionEvent::PaidFromAssetLockWithoutIdentity {
                    processing_fees: used_credits,
                    operations,
                })
            }
            StateTransitionAction::IdentityCreditWithdrawalAction(identity_credit_withdrawal) => {
                let user_fee_increase = identity_credit_withdrawal.user_fee_increase();
                let removed_balance = identity_credit_withdrawal.amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: Some(removed_balance),
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity credit withdrawal action",
                    )))
                }
            }
            StateTransitionAction::IdentityCreditTransferAction(identity_credit_transfer) => {
                let user_fee_increase = identity_credit_transfer.user_fee_increase();
                let removed_balance = identity_credit_transfer.transfer_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: Some(removed_balance),
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for identity credit transfer action",
                    )))
                }
            }
            StateTransitionAction::DocumentsBatchAction(document_batch_action) => {
                let user_fee_increase = action.user_fee_increase();
                let removed_balance = document_batch_action.all_used_balances()?;
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
            StateTransitionAction::MasternodeVoteAction(_) => {
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;

                Ok(ExecutionEvent::PaidFixedCost {
                    operations,
                    fees_to_add_to_pool: platform_version
                        .fee_version
                        .vote_resolution_fund_fees
                        .contested_document_single_vote_cost,
                })
            }
            _ => {
                let user_fee_increase = action.user_fee_increase();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(ExecutionEvent::Paid {
                        identity,
                        removed_balance: None,
                        operations,
                        execution_operations: execution_context.operations_consume(),
                        user_fee_increase,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present for other state transitions",
                    )))
                }
            }
        }
    }
}
