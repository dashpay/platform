use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::execution_event::ExecutionEvent::{
    PaidDriveEvent, PaidFromAssetLockDriveEvent,
};
use dpp::block::epoch::Epoch;
use dpp::identity::PartialIdentity;
use dpp::state_transition::StateTransitionAction;
use dpp::validation::SimpleConsensusValidationResult;
use drive::drive::batch::transitions::DriveHighLevelOperationConverter;
use drive::drive::batch::DriveOperation;
use drive::fee::credits::SignedCredits;
use drive::fee::result::FeeResult;
use tenderdash_abci::proto::abci::ExecTxResult;

/// The Fee Result for a Dry Run (without state)
pub type DryRunFeeResult = FeeResult;

/// An execution result
#[derive(Debug)]
pub enum ExecutionResult {
    /// Successfully executed a paid event
    SuccessfulPaidExecution(DryRunFeeResult, FeeResult),
    /// Successfully executed a free event
    SuccessfulFreeExecution,
    /// There were consensus errors when trying to execute an event
    ConsensusExecutionError(SimpleConsensusValidationResult),
}

// State transitions are never free, so we should filter out SuccessfulFreeExecution
// So we use an option
impl From<ExecutionResult> for ExecTxResult {
    fn from(value: ExecutionResult) -> Self {
        match value {
            ExecutionResult::SuccessfulPaidExecution(dry_run_fee_result, fee_result) => {
                ExecTxResult {
                    code: 0,
                    data: vec![],
                    log: "".to_string(),
                    info: "".to_string(),
                    gas_wanted: dry_run_fee_result.total_base_fee() as SignedCredits,
                    gas_used: fee_result.total_base_fee() as SignedCredits,
                    events: vec![],
                    codespace: "".to_string(),
                }
            }
            ExecutionResult::SuccessfulFreeExecution => ExecTxResult {
                code: 0,
                data: vec![],
                log: "".to_string(),
                info: "".to_string(),
                gas_wanted: 0,
                gas_used: 0,
                events: vec![],
                codespace: "".to_string(),
            },
            ExecutionResult::ConsensusExecutionError(validation_result) => {
                let code = validation_result
                    .errors
                    .first()
                    .map(|error| error.code())
                    .unwrap_or(1);
                ExecTxResult {
                    code,
                    data: vec![],
                    log: "".to_string(),
                    info: "".to_string(),
                    gas_wanted: 0,
                    gas_used: 0,
                    events: vec![],
                    codespace: "".to_string(),
                }
            }
        }
    }
}

// impl From<ValidationResult<ExecutionResult>> for ExecutionResult {
//     fn from(value: ValidationResult<ExecutionResult>) -> Self {
//         let ValidationResult { errors, data } = value;
//         if let Some(result) = data {
//             if !errors.is_empty() {
//                 ConsensusExecutionError(SimpleValidationResult::new_with_errors(errors))
//             } else {
//                 result
//             }
//         } else {
//             ConsensusExecutionError(SimpleValidationResult::new_with_errors(errors))
//         }
//     }
// }

/// An execution event
#[derive(Clone)]
pub enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperation<'a>>,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLockDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
    /// A drive event that is free
    FreeDriveEvent {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
}

impl<'a> ExecutionEvent<'a> {
    /// Creates a new identity Insertion Event
    pub fn new_document_operation(
        identity: PartialIdentity,
        operation: DriveOperation<'a>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            operations: vec![operation],
        }
    }
    /// Creates a new identity Insertion Event
    pub fn new_contract_operation(
        identity: PartialIdentity,
        operation: DriveOperation<'a>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            operations: vec![operation],
        }
    }
    /// Creates a new identity Insertion Event
    pub fn new_identity_insertion(
        identity: PartialIdentity,
        operations: Vec<DriveOperation<'a>>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            operations,
        }
    }
}

impl<'a> TryFrom<(Option<PartialIdentity>, StateTransitionAction, &Epoch)> for ExecutionEvent<'a> {
    type Error = Error;

    fn try_from(
        value: (Option<PartialIdentity>, StateTransitionAction, &Epoch),
    ) -> Result<Self, Self::Error> {
        let (identity, action, epoch) = value;
        match &action {
            StateTransitionAction::IdentityCreateAction(identity_create_action) => {
                let identity = identity_create_action.into();
                let operations = action.into_high_level_drive_operations(epoch)?;
                Ok(PaidFromAssetLockDriveEvent {
                    identity,
                    operations,
                })
            }
            StateTransitionAction::IdentityTopUpAction(_) => {
                let operations = action.into_high_level_drive_operations(epoch)?;
                if let Some(identity) = identity {
                    Ok(PaidFromAssetLockDriveEvent {
                        identity,
                        operations,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
            _ => {
                let operations = action.into_high_level_drive_operations(epoch)?;
                if let Some(identity) = identity {
                    Ok(PaidDriveEvent {
                        identity,
                        operations,
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
