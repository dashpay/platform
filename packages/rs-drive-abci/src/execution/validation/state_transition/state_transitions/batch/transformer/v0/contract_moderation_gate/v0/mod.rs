use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::fetch_seated_moderation_charter;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::contract_moderation_gate::ContractModerationRefusal;
use crate::execution::validation::state_transition::state_transitions::batch::transformer::v0::BatchTransitionInternalTransformerV0;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::state::contract_moderation::{
    ContractModeratedDocumentTypeNotYetUsableError, ContractModerationCounterpartyBarredError,
    ContractModerationCounterpartyRole, ContractUserBannedError, ContractUserSuspendedError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::ContractModerationList;
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::DataContract;
use dpp::identifier::Identifier;
use dpp::prelude::ConsensusValidationResult;
use dpp::state_transition::batch_transition::batched_transition::document_transition::{
    DocumentTransition, DocumentTransitionV0Methods,
};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use std::collections::{BTreeMap, BTreeSet};

pub(super) trait BatchTransitionContractModerationGateV0 {
    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_gate_v0<'a>(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&'a String, Vec<&'a DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<Identifier>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractModerationRefusal<'a>>, Error>;

    #[allow(clippy::too_many_arguments)]
    fn contract_moderation_counterparty_gate_v0(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        counterparty_id: Identifier,
        role: ContractModerationCounterpartyRole,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error>;
}

impl BatchTransitionContractModerationGateV0 for BatchTransition {
    /// A moderated contract refuses the document transitions of an identity on its banlist or
    /// under a live suspension, except its deletions (`Delete` and `IndexOnlyDelete`): a barred
    /// identity can write nothing new, but may still take down what it wrote. The first
    /// document transition after a suspension lapsed sweeps the stale entry. The gate runs in the transformer, so the mempool refuses a
    /// barred identity as a block does. A contract that declares no moderation costs nothing:
    /// no read is made for it.
    ///
    /// An elected contract whose declaration names no interim moderators refuses, first,
    /// every transition of a document type it moderates until a team is seated
    /// (`ContractModeratedDocumentTypeNotYetUsableError`), deletions included: nothing of
    /// those types was ever written. Whether a charter is seated is read, billed, only when a
    /// transition is on such a type; once one is, the block is over. The lists are then read
    /// only for the transitions on the other types, and not at all when nothing is left.
    fn contract_moderation_gate_v0<'a>(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        owner_id: Identifier,
        document_transitions: &BTreeMap<&'a String, Vec<&'a DocumentTransition>>,
        lapsed_suspensions: &mut BTreeSet<Identifier>,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContractModerationRefusal<'a>>, Error> {
        let Some(moderation) = contract.config().moderation() else {
            return Ok(None);
        };
        let data_contract_id = contract.id();

        // Every transition the gate refuses is refused on its own, as a per-transition
        // failure is anywhere else in the transformer, so each one's contract nonce is bumped
        // and none stays replayable.
        let mut actions = vec![];
        let mut errors = vec![];
        let failed = |transition: &DocumentTransition, error: ConsensusError| {
            Self::failed_per_transition_action(
                transition.base(),
                owner_id,
                vec![error],
                platform_version,
            )
        };

        // The interim block: the types an elected contract moderates wait for a team, until
        // a contest for its seat is awarded. Seating writes nothing under the contract, so the
        // charter contract is asked whether one was, and only when the block would apply.
        let interim_would_block = document_transitions
            .keys()
            .any(|document_type_name| moderation.interim_blocks_document_type(document_type_name));
        let interim_blocks = interim_would_block
            && fetch_seated_moderation_charter(
                drive,
                data_contract_id,
                &block_info.epoch,
                execution_context,
                transaction,
                platform_version,
            )?
            .is_none();
        let mut unblocked: BTreeMap<&'a String, Vec<&'a DocumentTransition>> = BTreeMap::new();
        for (document_type_name, transitions) in document_transitions {
            if interim_blocks && moderation.interim_blocks_document_type(document_type_name) {
                for transition in transitions {
                    let refusal = failed(
                        transition,
                        ContractModeratedDocumentTypeNotYetUsableError::new(
                            data_contract_id,
                            (*document_type_name).clone(),
                        )
                        .into(),
                    )?;
                    actions.extend(refusal.data);
                    errors.extend(refusal.errors);
                }
            } else {
                unblocked.insert(*document_type_name, transitions.clone());
            }
        }
        let blocked_any = !(actions.is_empty() && errors.is_empty());
        if unblocked.is_empty() {
            // Everything was blocked (the batch is never empty), so nothing needs the lists.
            return Ok(Some(ContractModerationRefusal {
                refused: ConsensusValidationResult::new_with_data_and_errors(actions, errors),
                passed: BTreeMap::new(),
            }));
        }

        let lists: Vec<ContractModerationList> = moderation.barring_lists().collect();
        let (fee, status) = drive.fetch_contract_moderation_status_with_fee(
            data_contract_id,
            owner_id,
            &lists,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        let barred: Option<ConsensusError> = if status.banned() {
            Some(ContractUserBannedError::new(data_contract_id, owner_id).into())
        } else {
            status
                .suspended_until()
                .filter(|until| *until > block_info.time_ms)
                .map(|until| {
                    ContractUserSuspendedError::new(data_contract_id, owner_id, until).into()
                })
        };

        let Some(error) = barred else {
            if status.has_lapsed_suspension_at(block_info.time_ms) {
                lapsed_suspensions.insert(data_contract_id);
            }
            if !blocked_any {
                return Ok(None);
            }
            return Ok(Some(ContractModerationRefusal {
                refused: ConsensusValidationResult::new_with_data_and_errors(actions, errors),
                passed: unblocked,
            }));
        };

        // Paid: the signer is authenticated and the read happened.
        let mut passed: BTreeMap<&'a String, Vec<&'a DocumentTransition>> = BTreeMap::new();
        for (document_type_name, transitions) in unblocked {
            for transition in transitions {
                if matches!(
                    transition,
                    DocumentTransition::Delete(_) | DocumentTransition::IndexOnlyDelete(_)
                ) {
                    passed
                        .entry(document_type_name)
                        .or_default()
                        .push(transition);
                    continue;
                }
                let refusal = failed(transition, error.clone())?;
                actions.extend(refusal.data);
                errors.extend(refusal.errors);
            }
        }

        // Nothing but deletions and nothing blocked: the bar does not apply, the batch carries
        // on whole.
        if actions.is_empty() && errors.is_empty() {
            return Ok(None);
        }

        let refused = if actions.is_empty() {
            ConsensusValidationResult::new_with_errors(errors)
        } else {
            ConsensusValidationResult::new_with_data_and_errors(actions, errors)
        };
        Ok(Some(ContractModerationRefusal { refused, passed }))
    }

    /// A barred identity is kept out of the contract's documents as a counterparty too: it can
    /// be given nothing by transfer and sell nothing, or a ban would still let it collect
    /// assets and proceeds on the contract. The read is billed to the batch. A lapsed
    /// suspension of a counterparty is not swept here; the identity's own next transition
    /// does that.
    fn contract_moderation_counterparty_gate_v0(
        drive: &Drive,
        block_info: &BlockInfo,
        contract: &DataContract,
        counterparty_id: Identifier,
        role: ContractModerationCounterpartyRole,
        execution_context: &mut StateTransitionExecutionContext,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error> {
        let Some(moderation) = contract.config().moderation() else {
            return Ok(None);
        };
        let data_contract_id = contract.id();

        let lists: Vec<ContractModerationList> = moderation.barring_lists().collect();
        let (fee, status) = drive.fetch_contract_moderation_status_with_fee(
            data_contract_id,
            counterparty_id,
            &lists,
            &block_info.epoch,
            transaction,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        Ok(status.is_barred_at(block_info.time_ms).then(|| {
            ContractModerationCounterpartyBarredError::new(data_contract_id, counterparty_id, role)
                .into()
        }))
    }
}
