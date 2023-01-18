use crate::abci::handlers::TenderdashAbci;
use crate::abci::messages::{
    AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees,
};
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use drive::dpp::identity::{Identity, PartialIdentityInfo};
use drive::drive::batch::DriveOperationType;
use drive::drive::block_info::BlockInfo;
use drive::error::Error::GroveDB;
use drive::fee::result::FeeResult;
use drive::grovedb::Transaction;

/// An execution event
pub enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: PartialIdentityInfo,
        /// Verify with dry run
        verify_balance_with_dry_run: bool,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperationType<'a>>,
    },
    /// A drive event that is free
    FreeDriveEvent {
        /// the operations that should be performed
        operations: Vec<DriveOperationType<'a>>,
    },
}

impl<'a> ExecutionEvent<'a> {
    /// Creates a new identity Insertion Event
    pub fn new_document_operation(
        identity: PartialIdentityInfo,
        operation: DriveOperationType<'a>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            verify_balance_with_dry_run: true,
            operations: vec![operation],
        }
    }
    /// Creates a new identity Insertion Event
    pub fn new_contract_operation(
        identity: PartialIdentityInfo,
        operation: DriveOperationType<'a>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            verify_balance_with_dry_run: true,
            operations: vec![operation],
        }
    }
    /// Creates a new identity Insertion Event
    pub fn new_identity_insertion(
        identity: PartialIdentityInfo,
        operations: Vec<DriveOperationType<'a>>,
    ) -> Self {
        Self::PaidDriveEvent {
            identity,
            verify_balance_with_dry_run: true,
            operations,
        }
    }
}

impl Platform {
    fn run_events(
        &self,
        events: Vec<ExecutionEvent>,
        block_info: &BlockInfo,
        transaction: &Transaction,
    ) -> Result<FeeResult, Error> {
        let mut total_fees = FeeResult::default();
        for event in events {
            match event {
                ExecutionEvent::PaidDriveEvent {
                    identity,
                    verify_balance_with_dry_run,
                    operations,
                } => {
                    let balance = identity.balance.ok_or(Error::Execution(
                        ExecutionError::CorruptedCodeExecution(
                            "partial identity info with no balance",
                        ),
                    ))?;
                    let enough_balance = if verify_balance_with_dry_run {
                        let estimated_fee_result = self
                            .drive
                            .apply_drive_operations(
                                operations.clone(),
                                false,
                                block_info,
                                Some(transaction),
                            )
                            .map_err(Error::Drive)?;

                        // TODO: Should take into account refunds as well
                        balance >= estimated_fee_result.total_base_fee()
                    } else {
                        true
                    };

                    if enough_balance {
                        let individual_fee_result = self
                            .drive
                            .apply_drive_operations(operations, true, block_info, Some(transaction))
                            .map_err(Error::Drive)?;

                        let balance_change = individual_fee_result
                            .into_balance_change(identity.id.to_buffer())
                            .map_err(Error::Drive)?;

                        let outcome = self.drive.apply_balance_change_from_fee_to_identity(
                            balance_change.clone(),
                            Some(transaction),
                        )?;

                        total_fees
                            .checked_add_assign(outcome.actual_fee_paid)
                            .map_err(Error::Drive)?;
                    }
                }
                ExecutionEvent::FreeDriveEvent { operations } => {
                    self.drive
                        .apply_drive_operations(operations, true, block_info, Some(transaction))
                        .map_err(Error::Drive)?;
                }
            }
        }
        Ok(total_fees)
    }

    /// Execute a block with various state transitions
    pub fn execute_block(
        &mut self,
        proposer: [u8; 32],
        block_info: &BlockInfo,
        state_transitions: Vec<ExecutionEvent>,
    ) -> Result<(), Error> {
        let transaction = self.drive.grove.start_transaction();
        // Processing block
        let block_begin_request = BlockBeginRequest {
            block_height: block_info.height,
            block_time_ms: block_info.time_ms,
            previous_block_time_ms: self
                .state
                .last_block_info
                .as_ref()
                .map(|block_info| block_info.time_ms),
            proposer_pro_tx_hash: proposer,
            validator_set_quorum_hash: Default::default(),
        };

        // println!("Block #{}", block_info.height);

        let block_begin_response = self
            .block_begin(block_begin_request, Some(&transaction))
            .unwrap_or_else(|_| {
                panic!(
                    "should begin process block #{} at time #{}",
                    block_info.height, block_info.time_ms
                )
            });

        // println!("{:#?}", block_begin_response);

        let total_fees = self.run_events(state_transitions, block_info, &transaction)?;

        let fees = BlockFees::from_fee_result(total_fees);

        let block_end_request = BlockEndRequest { fees };

        let block_end_response = self
            .block_end(block_end_request, Some(&transaction))
            .unwrap_or_else(|e| {
                panic!(
                    "engine should end process block #{} at time #{} : {}",
                    block_info.height, block_info.time_ms, e
                )
            });

        // println!("{:#?}", block_end_response);

        self.drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .map_err(|e| Error::Drive(GroveDB(e)))?;

        let after_finalize_block_request = AfterFinalizeBlockRequest {
            updated_data_contract_ids: Vec::new(),
        };

        self.after_finalize_block(after_finalize_block_request)
            .unwrap_or_else(|_| {
                panic!(
                    "should begin process block #{} at time #{}",
                    block_info.height, block_info.time_ms
                )
            });

        self.state.last_block_info = Some(block_info.clone());

        Ok(())
    }
}
