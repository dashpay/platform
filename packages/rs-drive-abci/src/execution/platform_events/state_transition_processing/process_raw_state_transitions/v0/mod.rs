use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::{Platform, PlatformRef};
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::codes::ErrorWithCode;
use dpp::fee::Credits;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::token_transition::{
    TokenTransition, TokenTransitionV0Methods,
};
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::document_base_transition::v0::v0_methods::DocumentBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::document_base_transition::v1::v1_methods::DocumentBaseTransitionV1Methods;
use dpp::state_transition::batch_transition::token_mint_transition::v0::v0_methods::TokenMintTransitionV0Methods;
use dpp::state_transition::batch_transition::token_transfer_transition::v0::v0_methods::TokenTransferTransitionV0Methods;
use dpp::state_transition::token_purchase_from_shielded_pool_transition::accessors::TokenPurchaseFromShieldedPoolTransitionAccessorsV0;
use dpp::state_transition::token_shielded_transfer_with_shielded_fee_transition::accessors::TokenShieldedTransferWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::token_unshield_with_shielded_fee_transition::accessors::TokenUnshieldWithShieldedFeeTransitionAccessorsV0;
use dpp::state_transition::{StateTransition, StateTransitionOwned};
use dpp::tokens::token_payment_info::methods::v0::TokenPaymentInfoMethodsV0;
use dpp::tokens::token_payment_info::v1::v1_accessors::TokenPaymentInfoAccessorsV1;
use drive::drive::document::draw_random_document_properties::RandomDocumentRequest;
use drive::drive::tokens::settle_random_token_amounts::{
    RandomTokenRequest, RandomTokenRequestKind,
};

use crate::execution::types::block_random_requests::BlockRandomRequests;
use crate::execution::types::state_transition_container::v0::{
    DecodedStateTransition, InvalidStateTransition, InvalidWithProtocolErrorStateTransition,
    SuccessfullyDecodedStateTransition,
};
use crate::execution::validation::state_transition::processor::process_state_transition;
use crate::metrics::{state_transition_execution_histogram, HistogramTiming};
use crate::platform_types::state_transitions_processing_result::{
    NotExecutedReason, StateTransitionExecutionResult, StateTransitionsProcessingResult,
};
use dpp::util::hash::hash_single;
use dpp::version::PlatformVersion;
use drive::grovedb::Transaction;
use std::time::Instant;

use super::super::StateTransitionAwareError;

/// Test-only fault injection: force the next successfully executed state transition to be
/// reported as an `InternalError` AFTER its drive operations were applied. This models the
/// only way an `InternalError` can carry state (an `Err` surfacing after
/// `apply_drive_operations(apply = true)`, e.g. the address-input fee coverage guard failing
/// on an under-estimated `Shield`) without depending on any particular estimation bug.
#[cfg(test)]
pub(crate) mod test_fault_injection {
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use std::cell::Cell;

    thread_local! {
        pub static FAIL_NEXT_SUCCESSFUL_EXECUTION: Cell<bool> = const { Cell::new(false) };
    }

    /// If armed, consume the flag and replace a successful execution with the
    /// `InternalError` a post-apply failure would produce; identity for every other result
    /// and while unarmed. Kept here so the processing loop carries a single call instead of
    /// the override logic.
    pub(crate) fn maybe_override(
        execution_result: StateTransitionExecutionResult,
    ) -> StateTransitionExecutionResult {
        if matches!(
            execution_result,
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        ) && FAIL_NEXT_SUCCESSFUL_EXECUTION.with(|flag| flag.replace(false))
        {
            StateTransitionExecutionResult::InternalError(
                "injected post-apply failure (test_fault_injection)".to_string(),
            )
        } else {
            execution_result
        }
    }
}

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Processes the given raw state transitions based on the `block_info` and `transaction`.
    ///
    /// This function takes a reference to a vector of raw state transitions, `BlockInfo`, and a `Transaction`
    /// as input and performs the corresponding state transition operations. It deserializes the raw state
    /// transitions into a `StateTransition` and processes them.
    ///
    /// # Arguments
    ///
    /// * `raw_state_transitions` - A reference to a vector of raw state transitions.
    /// * `block_info` - Information about the current block being processed.
    /// * `transaction` - The transaction associated with the raw state transitions.
    ///
    /// # Returns
    ///
    /// * `Result<StateTransitionsProcessingResult, Error>` - If the processing is successful, it returns
    ///   a `StateTransitionsProcessingResult` with state transition execution results and aggregated information.
    ///   If the processing fails, it returns an `Error`.
    ///
    /// # Errors
    ///
    /// This function may return an `Error` variant if there is a problem with deserializing the raw
    /// state transitions, processing state transitions, or executing events.
    ///
    #[allow(clippy::too_many_arguments)]
    pub(super) fn process_raw_state_transitions_v0(
        &self,
        raw_state_transitions: &[Vec<u8>],
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
        proposing_state_transitions: bool,
        timer: Option<&HistogramTiming>,
    ) -> Result<StateTransitionsProcessingResult, Error> {
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: block_platform_state,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };

        let state_transition_container =
            self.decode_raw_state_transitions(raw_state_transitions, platform_version)?;

        // PROPOSER-SIDE ONLY (consensus-invisible, hence no protocol-version gate): while
        // building a proposal, wrap each executed state transition in a savepoint and roll
        // back if its result strips it from the block (`TxAction::Removed`). Execution can
        // write into the shared block transaction before failing (the address-input fee flow
        // is apply-then-check), and without the rollback the gossiped block omits the
        // transition while the advertised app hash includes its writes — no validator can
        // reproduce the hash, and every proposer carrying the transition burns its round
        // (mainnet evo1 stalls of 2026-08-14/15, after heights 415652 and 415661).
        //
        // The validation path (`proposing_state_transitions == false`) is deliberately
        // untouched: rolling back there would change what state a received block evaluates
        // to, which is a consensus change needing a protocol-version gate — and one that is
        // unnecessary, because `process_proposal` already REJECTS any block whose execution
        // produced an `InternalError` or `UnpaidConsensusError` result (the
        // `unexpected_execution_results` gate), so a block that carries such a transition
        // can never commit and its writes die with the rejected round's transaction. This
        // proposer-side rollback only changes which blocks this node BUILDS — the published
        // block and app hash are exactly what any un-upgraded validator computes from that
        // block, so mixed networks cannot diverge.
        //
        // The genesis height is excluded because its re-proposal path relies on a
        // single-savepoint discipline: init_chain sets one savepoint, and each genesis round
        // rewinds to it with one `rollback_to_savepoint()` (see prepare_proposal /
        // process_proposal). Savepoints of KEPT transitions stay on the stack — RocksDB
        // exposes no pop-without-rollback — and extra savepoints on the genesis transaction
        // would redirect that rewind. At every other height each proposal round runs in a
        // freshly started transaction that is either committed (leftover savepoints are inert
        // markers) or dropped when the round ends, so the residue can affect nothing.
        let rollback_dropped_transitions =
            proposing_state_transitions && block_info.height != self.config.abci.genesis_height;

        let mut processing_result = StateTransitionsProcessingResult::default();

        // Credits the block's applied operations mint into Platform (asset locks), summed
        // across state transitions and recorded once per block as a credit inflow the net
        // daily withdrawal limit adds to its daily maximum.
        let mut block_credit_mints: Credits = 0;

        // Whether the block collects what its transitions ask its random beacon for: only from
        // protocol version 15, whose `apply_random_beacon` draws the values once Tenderdash
        // hands the beacon over. Every protocol version selects this generation, and the
        // collection is inert before 15 twice over: nothing is collected while the step is
        // `None`, which every earlier table sets, and what is collected lives only on the
        // in-memory processing result and block execution context, which are neither
        // serialized nor hashed. The block that switches to 15 receives no beacon, as Tenderdash
        // still runs it under consensus version 1: the batch transformer refuses there whatever
        // would ask the beacon for a value (`random_beacon_gate`), so all it collects are the
        // creates of types that draw nothing, which no draw would change.
        let collect_random_requests = platform_version
            .drive_abci
            .methods
            .block_end
            .apply_random_beacon
            .is_some();

        for decoded_state_transition in state_transition_container.into_iter() {
            // If we propose state transitions, we need to check if we have a time limit for processing
            // set and if we have exceeded it.
            let execution_result = if proposing_state_transitions
                && timer.is_some_and(|timer| {
                    timer.elapsed().as_millis()
                        > self
                            .config
                            .abci
                            .proposer_tx_processing_time_limit
                            .unwrap_or(u16::MAX) as u128
                }) {
                StateTransitionExecutionResult::NotExecuted(NotExecutedReason::ProposerRanOutOfTime)
            } else {
                match decoded_state_transition {
                    DecodedStateTransition::SuccessfullyDecoded(
                        SuccessfullyDecodedStateTransition {
                            decoded: state_transition,
                            raw: raw_state_transition,
                            elapsed_time: decoding_elapsed_time,
                        },
                    ) => {
                        let start_time = Instant::now();

                        let state_transition_name = state_transition.name();

                        if tracing::enabled!(tracing::Level::TRACE) {
                            let st_hash = hex::encode(hash_single(raw_state_transition));

                            tracing::trace!(
                                ?state_transition,
                                st_hash,
                                "Processing {} state transition",
                                state_transition_name
                            );
                        }

                        // Mark the state we can return to if this transition's result strips
                        // it from the block (see `rollback_dropped_transitions` above). The
                        // mint accumulator mirrors applied state, so it rewinds with it.
                        if rollback_dropped_transitions {
                            transaction.set_savepoint();
                        }
                        let credit_mints_at_savepoint = block_credit_mints;

                        // Remembered before the transition is consumed: the pools an applied
                        // batch touched get their anchor recorded at block end.
                        //
                        // Every protocol version selects this generation, so this has to be inert
                        // at the ones that predate token pools, and it is twice over: the kinds
                        // `token_shielded_pools_touched` reports are exactly the ones
                        // `validate_is_allowed` refuses unpaid there before execution, so the set
                        // stays empty, and the set's only consumer,
                        // `record_token_shielded_pool_anchors`, is `None` in those versions'
                        // tables and returns without touching state. The set lives on the
                        // in-memory processing result, which is neither serialized nor hashed.
                        let token_shielded_pools_touched =
                            token_shielded_pools_touched(&state_transition);

                        // Remembered before the transition is consumed too: what an applied
                        // transition asks the block's random beacon for (see
                        // `collect_random_requests` above). Nothing caps how many a block
                        // collects yet: the draw's cost is bounded, like the block's execution,
                        // by the block's size and the proposer's processing time limit.
                        let random_requests = if collect_random_requests {
                            random_requests_of(&state_transition)
                        } else {
                            Ok(BlockRandomRequests::default())
                        };

                        // Validate state transition and produce an execution event
                        let execution_result = process_state_transition(
                            &platform_ref,
                            block_info,
                            state_transition,
                            Some(transaction),
                        )
                        .map(|validation_result| {
                            // Dispatch to the versioned helper (v0 = pre-v13, v1 = records
                            // paid-invalid balance effects). Only this helper changed at v13; the outer
                            // loop is version-agnostic, so it must NOT be versioned.
                            self.process_validation_result(
                                raw_state_transition,
                                &state_transition_name,
                                validation_result,
                                block_info,
                                transaction,
                                &mut block_credit_mints,
                                platform_version,
                            )
                            .unwrap_or_else(error_to_internal_error_execution_result)
                        })
                        .map_err(|error| StateTransitionAwareError {
                            error,
                            raw_state_transition,
                            state_transition_name: Some(state_transition_name.to_string()),
                        })
                        .unwrap_or_else(error_to_internal_error_execution_result);

                        #[cfg(test)]
                        let execution_result =
                            test_fault_injection::maybe_override(execution_result);

                        if rollback_dropped_transitions {
                            match &execution_result {
                                StateTransitionExecutionResult::InternalError(_)
                                | StateTransitionExecutionResult::UnpaidConsensusError(_) => {
                                    // This transition will be stripped from the proposal
                                    // (`TxAction::Removed`), so none of its writes may remain
                                    // in the state the app hash is computed over. A rollback
                                    // failure means the proposal can no longer match the
                                    // block — fail it rather than continue on leaked state.
                                    transaction
                                        .rollback_to_savepoint()
                                        .map_err(drive::grovedb::error::Error::StorageError)?;
                                    // The rollback discarded this transition's writes; drop
                                    // its mints with them, or the block would record a
                                    // credit inflow for a transition the proposal omits and
                                    // validators re-executing it would compute other state.
                                    block_credit_mints = credit_mints_at_savepoint;
                                    // Any contract the transition rewrote was re-seeded into
                                    // the block cache as it was applied, and the rollback
                                    // just reverted it in state. Drop those copies so the
                                    // next transactional read of them goes to the rolled-back
                                    // transaction rather than to a phantom definition.
                                    self.drive
                                        .cache
                                        .data_contracts
                                        .drop_block_modified_entries();
                                }
                                StateTransitionExecutionResult::SuccessfulExecution { .. }
                                | StateTransitionExecutionResult::PaidConsensusError { .. } => {
                                    // The transition stays in the block
                                    // (`TxAction::Unmodified`), so its writes stay. Its
                                    // savepoint is intentionally left on the stack (see
                                    // `rollback_dropped_transitions` above: no
                                    // pop-without-rollback exists, and at non-genesis heights
                                    // the residue is inert).
                                }
                                StateTransitionExecutionResult::NotExecuted(_) => {
                                    // Delayed to a later block (`TxAction::Delayed`) without
                                    // having been executed: nothing was written since the
                                    // savepoint, so rolling back and leaving it are
                                    // equivalent. Leave it, like the kept outcomes above.
                                    //
                                    // Deliberately exhaustive: a new execution result variant
                                    // must make an explicit savepoint decision here — the
                                    // rollback classification must match the `TxAction`
                                    // classification in `prepare_proposal`.
                                }
                            }
                        }

                        // Only a successful execution writes to a token pool; a paid failure
                        // bumps the nonce and nothing else, and its pool may not even exist
                        // (a token without the flag, or a paid rejection before the pool
                        // check), so it must not reach the block end anchor recorder.
                        if matches!(
                            execution_result,
                            StateTransitionExecutionResult::SuccessfulExecution { .. }
                        ) {
                            processing_result
                                .add_token_shielded_pools_touched(token_shielded_pools_touched);
                            // Likewise only a successful execution writes a placeholder or
                            // reserves a random amount: a paid failure creates no document and
                            // moves no token.
                            processing_result.add_random_requests(random_requests.map_err(
                                |message| {
                                    Error::Execution(ExecutionError::CorruptedCodeExecution(
                                        message,
                                    ))
                                },
                            )?);
                        }

                        // Store metrics
                        let elapsed_time = start_time.elapsed() + decoding_elapsed_time;

                        let code = match &execution_result {
                            StateTransitionExecutionResult::SuccessfulExecution { .. } => 0,
                            StateTransitionExecutionResult::PaidConsensusError {
                                error, ..
                            } => error.code(),
                            StateTransitionExecutionResult::UnpaidConsensusError(error) => {
                                error.code()
                            }
                            StateTransitionExecutionResult::InternalError(_) => 1,
                            StateTransitionExecutionResult::NotExecuted(_) => 1, //todo
                        };

                        state_transition_execution_histogram(
                            elapsed_time,
                            &state_transition_name,
                            code,
                        );

                        execution_result
                    }
                    DecodedStateTransition::InvalidEncoding(InvalidStateTransition {
                        raw,
                        error,
                        elapsed_time: decoding_elapsed_time,
                    }) => {
                        if tracing::enabled!(tracing::Level::DEBUG) {
                            let st_hash = hex::encode(hash_single(raw));

                            tracing::debug!(
                                ?error,
                                st_hash,
                                "Invalid unknown state transition ({}): {}",
                                st_hash,
                                error
                            );
                        }

                        // Store metrics
                        state_transition_execution_histogram(
                            decoding_elapsed_time,
                            "Unknown",
                            error.code(),
                        );

                        StateTransitionExecutionResult::UnpaidConsensusError(error)
                    }
                    DecodedStateTransition::FailedToDecode(
                        InvalidWithProtocolErrorStateTransition {
                            raw,
                            error: protocol_error,
                            elapsed_time: decoding_elapsed_time,
                        },
                    ) => {
                        // Store metrics
                        state_transition_execution_histogram(decoding_elapsed_time, "Unknown", 1);

                        error_to_internal_error_execution_result(StateTransitionAwareError {
                            error: protocol_error.into(),
                            raw_state_transition: raw,
                            state_transition_name: None,
                        })
                    }
                }
            };

            processing_result.add(execution_result)?;
        }

        processing_result.set_credit_mints(block_credit_mints);

        Ok(processing_result)
    }
}

fn error_to_internal_error_execution_result(
    error_with_st: StateTransitionAwareError,
) -> StateTransitionExecutionResult {
    if tracing::enabled!(tracing::Level::ERROR) {
        let st_hash = hex::encode(hash_single(error_with_st.raw_state_transition));

        tracing::error!(
            error = ?error_with_st.error,
            raw_state_transition = ?error_with_st.raw_state_transition,
            st_hash,
            "Failed to process {} state transition ({}) : {}",
            error_with_st.state_transition_name.unwrap_or_else(|| "unknown".to_string()),
            st_hash,
            error_with_st.error,
        );
    }

    StateTransitionExecutionResult::InternalError(error_with_st.error.to_string())
}

/// The token shielded pools a state transition writes to: the token ids of every token pool
/// transition in a batch, of every document whose token cost is paid from a pool, and of the
/// identity-less token pool transitions.
///
/// Every kind matched here is one that `StateTransitionIsAllowedValidationV0::validate_is_allowed`
/// refuses, unpaid and before execution, below the protocol version that introduces token pools,
/// so this returns an empty list at every earlier version.
fn token_shielded_pools_touched(state_transition: &StateTransition) -> Vec<[u8; 32]> {
    match state_transition {
        StateTransition::Batch(batch) => batch
            .transitions_iter()
            .filter_map(|transition| match transition {
                BatchedTransitionRef::Token(token_transition) => token_transition
                    .shielded_pool_actions()
                    .map(|_| token_transition.token_id().to_buffer()),
                BatchedTransitionRef::Document(document_transition) => {
                    let base = document_transition.base();
                    base.token_payment_info_ref()
                        .as_ref()
                        .filter(|info| info.shielded_payment().is_some())
                        .map(|info| info.token_id(base.data_contract_id()).to_buffer())
                }
            })
            .collect(),
        StateTransition::TokenShieldedTransferWithShieldedFee(st) => {
            vec![st.token_id().to_buffer()]
        }
        StateTransition::TokenUnshieldWithShieldedFee(st) => vec![st.token_id().to_buffer()],
        StateTransition::TokenPurchaseFromShieldedPool(st) => vec![st.token_id().to_buffer()],
        _ => vec![],
    }
}

/// What a state transition asks the block's random beacon for (protocol version 15), in the
/// order of its transitions: every document a batch creates, whose `sys.random` properties the
/// draw fills (it leaves alone a document whose type declares none), and every token transfer
/// or mint of a batch with a random minimum amount, whose reservation the settlement pays out.
///
/// The requests mirror what execution wrote: the reservation of a random amount is keyed by
/// the token, the batch's owner and the transition's identity contract nonce, and holds the
/// transition's amount, the most it moves. A random mint always names its recipient, as
/// structure validation refuses one that does not; the error only reaches the caller, as a
/// corrupted execution, should such a mint ever execute successfully.
fn random_requests_of(
    state_transition: &StateTransition,
) -> Result<BlockRandomRequests, &'static str> {
    let mut requests = BlockRandomRequests::default();
    let StateTransition::Batch(batch) = state_transition else {
        return Ok(requests);
    };
    let owner_id = batch.owner_id();
    for transition in batch.transitions_iter() {
        match transition {
            BatchedTransitionRef::Document(document_transition) => {
                if matches!(document_transition, DocumentTransition::Create(_)) {
                    let base = document_transition.base();
                    requests.documents.push(RandomDocumentRequest {
                        contract_id: base.data_contract_id(),
                        document_type_name: base.document_type_name().clone(),
                        document_id: base.id(),
                    });
                }
            }
            BatchedTransitionRef::Token(token_transition) => {
                let random_amount = match token_transition {
                    TokenTransition::Transfer(transfer) => {
                        transfer.random_minimum_amount().map(|minimum| {
                            (
                                minimum,
                                transfer.amount(),
                                RandomTokenRequestKind::Transfer {
                                    recipient_id: transfer.recipient_id(),
                                },
                            )
                        })
                    }
                    TokenTransition::Mint(mint) => match mint.random_minimum_amount() {
                        Some(minimum) => {
                            let recipient_id = mint
                                .issued_to_identity_id()
                                .ok_or("a random token mint names its recipient")?;
                            Some((
                                minimum,
                                mint.amount(),
                                RandomTokenRequestKind::Mint { recipient_id },
                            ))
                        }
                        None => None,
                    },
                    _ => None,
                };
                if let Some((minimum, maximum, kind)) = random_amount {
                    requests.tokens.push(RandomTokenRequest {
                        token_id: token_transition.token_id(),
                        owner_id,
                        identity_contract_nonce: token_transition.identity_contract_nonce(),
                        minimum,
                        maximum,
                        kind,
                    });
                }
            }
        }
    }
    Ok(requests)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identifier::Identifier;
    use dpp::platform_value::BinaryData;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use dpp::state_transition::batch_transition::document_create_transition::{
        DocumentCreateTransition, DocumentCreateTransitionV0,
    };
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_mint_transition::TokenMintTransitionV0;
    use dpp::state_transition::batch_transition::token_transfer_transition::TokenTransferTransitionV0;
    use dpp::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV1, TokenMintTransition, TokenTransferTransition,
    };
    use std::collections::BTreeMap;

    const OWNER: [u8; 32] = [1; 32];
    const TOKEN: [u8; 32] = [2; 32];
    const RECIPIENT: [u8; 32] = [3; 32];
    const CONTRACT: [u8; 32] = [4; 32];

    fn batch(transitions: Vec<BatchedTransition>) -> StateTransition {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from(OWNER),
            transitions,
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::default(),
        }))
    }

    fn transfer(random_minimum_amount: Option<u64>) -> BatchedTransition {
        let mut transfer: TokenTransferTransition = TokenTransferTransitionV0 {
            base: TokenBaseTransitionV0 {
                identity_contract_nonce: 5,
                token_contract_position: 0,
                data_contract_id: Identifier::from(CONTRACT),
                token_id: Identifier::from(TOKEN),
                using_group_info: None,
            }
            .into(),
            amount: 900,
            recipient_id: Identifier::from(RECIPIENT),
            public_note: None,
            shared_encrypted_note: None,
            private_encrypted_note: None,
        }
        .into();
        transfer.set_random_minimum_amount(random_minimum_amount);
        BatchedTransition::Token(TokenTransition::Transfer(transfer))
    }

    fn mint(
        issued_to_identity_id: Option<Identifier>,
        random_minimum_amount: Option<u64>,
    ) -> BatchedTransition {
        let mut mint: TokenMintTransition = TokenMintTransitionV0 {
            base: TokenBaseTransitionV0 {
                identity_contract_nonce: 6,
                token_contract_position: 0,
                data_contract_id: Identifier::from(CONTRACT),
                token_id: Identifier::from(TOKEN),
                using_group_info: None,
            }
            .into(),
            issued_to_identity_id,
            amount: 700,
            public_note: None,
        }
        .into();
        mint.set_random_minimum_amount(random_minimum_amount);
        BatchedTransition::Token(TokenTransition::Mint(mint))
    }

    fn create(document_id: [u8; 32]) -> BatchedTransition {
        let create: DocumentCreateTransition = DocumentCreateTransitionV0 {
            base: DocumentBaseTransitionV0 {
                id: Identifier::from(document_id),
                identity_contract_nonce: 7,
                document_type_name: "card".to_string(),
                data_contract_id: Identifier::from(CONTRACT),
            }
            .into(),
            entropy: [0; 32],
            data: BTreeMap::new(),
            prefunded_voting_balance: None,
        }
        .into();
        BatchedTransition::Document(DocumentTransition::Create(create))
    }

    #[test]
    fn should_collect_every_created_document_and_random_token_amount_in_order() {
        let requests = random_requests_of(&batch(vec![
            create([8; 32]),
            transfer(Some(100)),
            transfer(None),
            mint(Some(Identifier::from(RECIPIENT)), Some(50)),
            mint(Some(Identifier::from(RECIPIENT)), None),
            create([9; 32]),
        ]))
        .expect("expected the requests");

        let card = |document_id: [u8; 32]| RandomDocumentRequest {
            contract_id: Identifier::from(CONTRACT),
            document_type_name: "card".to_string(),
            document_id: Identifier::from(document_id),
        };
        assert_eq!(requests.documents, vec![card([8; 32]), card([9; 32])]);
        assert_eq!(
            requests.tokens,
            vec![
                RandomTokenRequest {
                    token_id: Identifier::from(TOKEN),
                    owner_id: Identifier::from(OWNER),
                    identity_contract_nonce: 5,
                    minimum: 100,
                    maximum: 900,
                    kind: RandomTokenRequestKind::Transfer {
                        recipient_id: Identifier::from(RECIPIENT),
                    },
                },
                RandomTokenRequest {
                    token_id: Identifier::from(TOKEN),
                    owner_id: Identifier::from(OWNER),
                    identity_contract_nonce: 6,
                    minimum: 50,
                    maximum: 700,
                    kind: RandomTokenRequestKind::Mint {
                        recipient_id: Identifier::from(RECIPIENT),
                    },
                },
            ]
        );
    }

    #[test]
    fn should_refuse_a_random_mint_that_names_no_recipient() {
        assert!(random_requests_of(&batch(vec![mint(None, Some(50))])).is_err());
        assert!(random_requests_of(&batch(vec![mint(None, None)]))
            .expect("a mint of a fixed amount asks for nothing")
            .is_empty());
    }
}
