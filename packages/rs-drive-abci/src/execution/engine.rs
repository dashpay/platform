use drive::dpp::identity::Identity;
use drive::drive::batch::DriveOperationType;
use drive::drive::block_info::BlockInfo;
use drive::error::drive::DriveError;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee::result::FeeResult;
use drive::grovedb::Transaction;
use crate::abci::handlers::TenderdashAbci;
use crate::abci::messages::{AfterFinalizeBlockRequest, BlockBeginRequest, BlockEndRequest, BlockFees};
use crate::error::Error;
use crate::execution::engine::ExecutionEvent::{FreeDriveEvent, PaidDriveEvent};
use crate::platform::Platform;

/// An execution event
pub enum ExecutionEvent<'a> {
    /// A drive event that is paid by an identity
    PaidDriveEvent {
        /// The identity requesting the event
        identity: Identity,
        /// Verify with dry run
        verify_balance_with_dry_run: bool,
        /// the operations that the identity is requesting to perform
        operations: Vec<DriveOperationType<'a>>
    },
    /// A drive event that is free
    FreeDriveEvent {
        /// the operations that should be performed
        operations: Vec<DriveOperationType<'a>>
    }
}

impl <'a> ExecutionEvent<'a> {
    /// Creates a new identity Insertion Event
    pub fn new_document_operation(identity: Identity, operation: DriveOperationType<'a>) -> Self {
        PaidDriveEvent {
            identity,
            verify_balance_with_dry_run: true,
            operations: vec![operation],
        }
    }
    /// Creates a new identity Insertion Event
    pub fn new_identity_insertion(operations: Vec<DriveOperationType<'a>>) -> Self {
        FreeDriveEvent { operations }
    }
}

impl Platform {
    fn run_events(&self, events : Vec<ExecutionEvent>, block_info: &BlockInfo, transaction: &Transaction) -> Result<FeeResult, Error> {
        let mut total_fees = FeeResult::default();
        for event in events {
            match event {
                ExecutionEvent::PaidDriveEvent { identity, verify_balance_with_dry_run, operations } => {
                    let mut enough_balance = true;
                    if verify_balance_with_dry_run {
                        let estimated_fee_result = self.drive.apply_drive_operations(operations.clone(), false, block_info, Some(transaction)).map_err(Error::Drive)?;
                        if identity.balance < estimated_fee_result.total_fee() {
                            enough_balance = false
                        }
                    }
                    if enough_balance {
                        let individual_fee_result = self.drive.apply_drive_operations(operations, true, block_info, Some(transaction)).map_err(Error::Drive)?;
                        self.drive.remove_from_identity_balance(
                            identity.id.to_buffer(),
                            individual_fee_result.required_removed_balance(),
                            individual_fee_result.desired_removed_balance(),
                            block_info,
                            true,
                            Some(transaction),
                        )?;
                        total_fees.checked_add_assign(individual_fee_result).map_err(Error::Drive)?;
                    }
                }
                ExecutionEvent::FreeDriveEvent { operations } => {
                    self.drive.apply_drive_operations(operations, true, block_info, Some(transaction)).map_err(Error::Drive)?;
                }
            }
        }
        Ok(total_fees)
    }

    /// Execute a block with various state transitions
    pub fn execute_block(&mut self, proposer: [u8;32], block_info: &BlockInfo, state_transitions: Vec<ExecutionEvent>) -> Result<(), Error> {
        let transaction = self.drive.grove.start_transaction();
        // Processing block
        let block_begin_request = BlockBeginRequest {
            block_height: block_info.height,
            block_time_ms: block_info.time_ms,
            previous_block_time_ms: self.state.last_block_info.as_ref().map(|block_info| block_info.time_ms),
            proposer_pro_tx_hash: proposer,
            validator_set_quorum_hash: Default::default(),
        };

        let block_begin_response = self
            .block_begin(block_begin_request, Some(&transaction))
            .expect(
                    format!("should begin process block #{} at time #{}",
                    block_info.height, block_info.time_ms).as_str());

        let total_fees = self.run_events(state_transitions, block_info, &transaction)?;

        let block_end_request = BlockEndRequest {
            fees: BlockFees::from_fee_result(total_fees),
        };

        let block_end_response = self
            .block_end(block_end_request, Some(&transaction))
            .expect(
                format!(
                    "should end process block #{} at time #{}",
                    block_info.height, block_info.time_ms
                )
                    .as_str(),
            );

        let after_finalize_block_request = AfterFinalizeBlockRequest {
            updated_data_contract_ids: Vec::new(),
        };

        self
            .after_finalize_block(after_finalize_block_request)
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