mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent::{
    PaidDriveEvent, PaidFromAssetLockDriveEvent,
};
use dpp::block::epoch::Epoch;
use dpp::fee::Credits;

use dpp::identity::PartialIdentity;

use dpp::version::PlatformVersion;
use drive::state_transition_action::StateTransitionAction;

use crate::execution::types::validation_operation::ValidationOperation;
use drive::drive::batch::transitions::DriveHighLevelOperationConverter;
use drive::drive::batch::DriveOperation;

/// An execution event
#[derive(Clone)]
pub(in crate::execution) enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperation<'a>>,
        validation_operations: Vec<ValidationOperation>,
    },
    /// A drive event that is paid from an asset lock
    PaidFromAssetLockDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// The added balance
        added_balance: Credits,
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
        validation_operations: Vec<ValidationOperation>,
    },
    /// A drive event that is free
    FreeDriveEvent {
        /// the operations that should be performed
        operations: Vec<DriveOperation<'a>>,
    },
}

// TODO: none of these functions are used in the project. Remove?
impl<'a> ExecutionEvent<'a> {
    /// Creates a new identity Insertion Event
    pub fn new_document_operation(
        identity: PartialIdentity,
        operation: DriveOperation<'a>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            operations: vec![operation],
            validation_operations: vec![],
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
            validation_operations: vec![],
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
            validation_operations: vec![],
        }
    }
}

impl<'a> ExecutionEvent<'a> {
    pub(crate) fn create_from_state_transition_action(
        action: StateTransitionAction,
        identity: Option<PartialIdentity>,
        epoch: &Epoch,
        validation_operations: Vec<ValidationOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        match &action {
            StateTransitionAction::IdentityCreateAction(identity_create_action) => {
                let identity = identity_create_action.into();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                Ok(PaidFromAssetLockDriveEvent {
                    identity,
                    added_balance: 0,
                    operations,
                    validation_operations,
                })
            }
            StateTransitionAction::IdentityTopUpAction(identity_top_up_action) => {
                let added_balance = identity_top_up_action.top_up_balance_amount();
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidFromAssetLockDriveEvent {
                        identity,
                        added_balance,
                        operations,
                        validation_operations,
                    })
                } else {
                    Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                        "partial identity should be present",
                    )))
                }
            }
            _ => {
                let operations =
                    action.into_high_level_drive_operations(epoch, platform_version)?;
                if let Some(identity) = identity {
                    Ok(PaidDriveEvent {
                        identity,
                        operations,
                        validation_operations,
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
