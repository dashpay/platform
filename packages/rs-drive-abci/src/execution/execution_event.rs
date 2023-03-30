use dpp::identity::PartialIdentity;
use drive::drive::batch::DriveOperation;

/// An execution event
pub enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentity,
        /// Verify with dry run
        verify_balance_with_dry_run: bool,
        /// the operations that the identity is requesting to perform
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
            verify_balance_with_dry_run: true,
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
            verify_balance_with_dry_run: true,
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
            verify_balance_with_dry_run: true,
            operations,
        }
    }
}
