use crate::error::Error;
use crate::execution::types::execution_event::ExecutionEvent;
use crate::platform_types::event_execution_result::EventExecutionResult;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::address_funds::PlatformAddress;
use dpp::balances::credits::CreditOperation;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::document::document_sum_overflow_error::DocumentSumOverflowError;
use dpp::consensus::ConsensusError;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use drive::grovedb::Error as GroveError;
use drive::grovedb::Transaction;
use drive::util::batch::{ContractModerationOperationType, DriveOperation, IdentityOperationType};
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Generation 1, except that a gas sponsor who pays is named as the owner of the storage
    /// the batch's documents write, so the refunds of that storage go back to them.
    ///
    /// A document write whose operations would take a document sum past what a sum tree holds
    /// (GroveDB refuses to apply them) is refused as a paid consensus error,
    /// [`DocumentSumOverflowError`]: everything the attempt wrote is rolled back, and the event
    /// is executed again as the nonce bump a refused state validation of the batch leaves
    /// ([`SumOverflowRefusal`]). The signer pays for the bump, the work validation billed, and
    /// the processing of the refused write as fee validation estimates it; no storage. Whether a
    /// write overflows is a function of the state it is applied to, so every node refuses the
    /// same writes.
    ///
    /// The rollback needs a savepoint on the block transaction, which stays on the transaction
    /// when the write applies (RocksDB pops a savepoint only by rolling back to it). The genesis
    /// height rewinds each re-proposal of its block to the one savepoint `init_chain` set, so no
    /// savepoint is set there and a write overflowing a sum at the genesis height stays the
    /// internal error it was. At every other height a savepoint left on the transaction is inert:
    /// the block's transaction is committed or dropped as a whole.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn execute_event_v2(
        &self,
        event: ExecutionEvent,
        consensus_errors: Vec<ConsensusError>,
        block_info: &BlockInfo,
        transaction: &Transaction,
        mut address_balances_in_update: Option<&mut BTreeMap<PlatformAddress, CreditOperation>>,
        block_credit_mints: &mut Credits,
        platform_version: &PlatformVersion,
    ) -> Result<EventExecutionResult, Error> {
        // An event that already carries consensus errors is the nonce bump of a refused batch,
        // which writes no document.
        let refusal = if consensus_errors.is_empty()
            && block_info.height != self.config.abci.genesis_height
        {
            SumOverflowRefusal::of(&event)
        } else {
            None
        };
        let Some(refusal) = refusal else {
            return self.execute_event_v1_body(
                event,
                consensus_errors,
                block_info,
                transaction,
                address_balances_in_update,
                block_credit_mints,
                true,
                platform_version,
            );
        };

        transaction.set_savepoint();
        match self.execute_event_v1_body(
            event,
            consensus_errors,
            block_info,
            transaction,
            address_balances_in_update.as_deref_mut(),
            block_credit_mints,
            true,
            platform_version,
        ) {
            Err(Error::Drive(error)) if error.is_sum_tree_overflow() => {
                // Applying a write may take several GroveDB batches and direct writes (the
                // clean-up of expired time range buckets before them), and only the refused
                // batch is undone by GroveDB. Rolling back to the savepoint undoes the rest, so
                // the state is as if the write had never been attempted. The fee and the
                // balance changes are written only after the operations applied.
                transaction
                    .rollback_to_savepoint()
                    .map_err(|error| Error::Drive(GroveError::StorageError(error).into()))?;
                let SumOverflowRefusal {
                    event: refusal_event,
                    attempted_operations,
                    contract_id,
                } = refusal;
                let attempted_processing_fee = self
                    .drive
                    .apply_drive_operations(
                        attempted_operations,
                        false,
                        block_info,
                        Some(transaction),
                        platform_version,
                    )
                    .map_err(Error::Drive)?
                    .processing_fee;
                self.execute_event_v1_body(
                    refusal_event.with_additional_fixed_fee_cost(attempted_processing_fee),
                    vec![DocumentSumOverflowError::new(contract_id).into()],
                    block_info,
                    transaction,
                    address_balances_in_update,
                    block_credit_mints,
                    true,
                    platform_version,
                )
            }
            result => result,
        }
    }
}

/// What an identity-paid document write is executed as when its operations would take a
/// document sum past what a sum tree holds.
struct SumOverflowRefusal<'a> {
    /// The write's event with only the operations a refused state validation of the batch
    /// leaves: the identity contract nonce bumps and the removal of a suspension that lapsed.
    /// Nothing else of the write runs, so it moves no balance, owes no action fee and is never
    /// sponsored, like any batch that failed.
    event: ExecutionEvent<'a>,
    /// The operations of the refused write, which price the processing it cost.
    attempted_operations: Vec<DriveOperation<'a>>,
    /// The contract the write is to.
    contract_id: Identifier,
}

impl<'a> SumOverflowRefusal<'a> {
    /// The refusal of `event`, when it is an identity-paid write of documents to a contract.
    fn of(event: &ExecutionEvent<'a>) -> Option<Self> {
        let ExecutionEvent::Paid {
            identity,
            operations,
            execution_operations,
            user_fee_increase,
            signing_key_limits,
            ..
        } = event
        else {
            return None;
        };
        if !operations
            .iter()
            .any(|operation| matches!(operation, DriveOperation::DocumentOperation(_)))
        {
            return None;
        }
        let contract_id = operations.iter().find_map(|operation| match operation {
            DriveOperation::IdentityOperation(
                IdentityOperationType::UpdateIdentityContractNonce { contract_id, .. },
            ) => Some(Identifier::new(*contract_id)),
            _ => None,
        })?;
        let refused_operations = operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation,
                    DriveOperation::IdentityOperation(
                        IdentityOperationType::UpdateIdentityContractNonce { .. }
                    ) | DriveOperation::ContractModerationOperation(
                        ContractModerationOperationType::RemoveSuspension { .. }
                    )
                )
            })
            .cloned()
            .collect();
        Some(Self {
            event: ExecutionEvent::Paid {
                identity: identity.clone(),
                removed_balance: None,
                added_to_balance_outputs: None,
                operations: refused_operations,
                execution_operations: execution_operations.clone(),
                additional_fixed_fee_cost: None,
                user_fee_increase: *user_fee_increase,
                signing_key_limits: *signing_key_limits,
                gas_sponsor: None,
                action_fees: vec![],
            },
            attempted_operations: operations.clone(),
            contract_id,
        })
    }
}

impl ExecutionEvent<'_> {
    /// The event with `fee` added to the processing fee it charges up front.
    fn with_additional_fixed_fee_cost(mut self, fee: Credits) -> Self {
        if let ExecutionEvent::Paid {
            additional_fixed_fee_cost,
            ..
        } = &mut self
        {
            *additional_fixed_fee_cost = Some(
                additional_fixed_fee_cost
                    .unwrap_or_default()
                    .saturating_add(fee),
            );
        }
        self
    }
}
