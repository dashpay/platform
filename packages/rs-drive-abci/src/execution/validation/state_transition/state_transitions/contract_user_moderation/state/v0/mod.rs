use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::ValidationOperation;
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::state_transitions::batch::fetch_document_with_id;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::state::contract_moderation::{
    ContractModerationNotEnabledError, ContractModerationTargetNotAllowedError,
    ContractModerationTargetNotFoundError, ContractSuspensionNotInFutureError,
    ContractUserAlreadyBannedError, ContractUserBannedError, ContractUserNotBannedError,
    ContractUserNotSuspendedError, DocumentTypeNotDeletableByModeratorsError,
    IdentityNotContractModeratorError,
};
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractModerationConfig, ContractModerationList, ContractModerationStatus,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::document::DocumentV0Getters;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransitionOwned;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::v0::ContractDocumentDeletionContext;
use drive::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;
use std::sync::Arc;

pub(in crate::execution::validation::state_transition::state_transitions::contract_user_moderation) trait ContractUserModerationStateTransitionStateValidationV0
{
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error>;
}

impl ContractUserModerationStateTransitionStateValidationV0 for ContractUserModerationTransition {
    /// A document deletion is checked by `transform_document_deletion_v0`. For the rest:
    /// reads the contract and the target's status and checks the moderation: the contract
    /// keeps the list the action edits, the signer is its owner or one of its moderators, the
    /// target is neither and exists, and the action fits the target's status. Every refusal,
    /// a contract that does not exist included, is paid for by bumping the signer's contract
    /// nonce.
    ///
    /// The action carries what Drive needs of the target's status as read here, so Drive edits
    /// the lists without reading them again, and the mempool, which transforms without a state validation stage,
    /// refuses with the same consensus codes as a block.
    fn transform_into_action_v0<C: CoreRPCLike>(
        &self,
        platform: &PlatformRef<C>,
        block_info: &BlockInfo,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
        let contract_id = self.data_contract_id();
        let moderator_id = self.owner_id();
        let action = self.action();

        let (contract_fetch_fee, maybe_contract_fetch_info) =
            platform.drive.get_contract_with_fetch_info_and_fee(
                contract_id.to_buffer(),
                Some(&block_info.epoch),
                false,
                tx,
                platform_version,
            )?;
        // The read is billed from the fee this call returns, whether the contract was pulled
        // from disk, was in the cache or does not exist. The fee a cached fetch info carries is
        // only there when that entry was built with an epoch, which differs from node to node,
        // so billing it would make the fee, and the app hash, depend on the cache.
        let contract_fetch_fee =
            contract_fetch_fee.ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "fee must exist for the contract fetch of a contract user moderation transition",
            )))?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(
            contract_fetch_fee,
        ));

        let bump_action = || {
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                    self,
                ),
            )
        };
        let refuse = |error: ConsensusError| {
            Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                vec![error],
            ))
        };

        // Paid like every other refusal: the signer is authenticated and the lookup happened,
        // as for a contract update of a contract that does not exist.
        let Some(contract_fetch_info) = maybe_contract_fetch_info else {
            return refuse(DataContractNotPresentError::new(contract_id).into());
        };

        let contract = &contract_fetch_info.contract;

        let target_id = match action {
            ContractUserModerationAction::DeleteDocument {
                document_type_name,
                document_id,
                ..
            } => {
                return transform_document_deletion_v0(
                    self,
                    platform,
                    block_info,
                    &contract_fetch_info,
                    document_type_name,
                    *document_id,
                    execution_context,
                    tx,
                    platform_version,
                );
            }
            ContractUserModerationAction::Ban { identity_id, .. }
            | ContractUserModerationAction::Unban { identity_id }
            | ContractUserModerationAction::Suspend { identity_id, .. }
            | ContractUserModerationAction::Unsuspend { identity_id } => *identity_id,
        };
        let Some(list) = list_of(action) else {
            return Err(Error::Execution(ExecutionError::CorruptedCodeExecution(
                "a moderation of an identity edits a list",
            )));
        };

        let Some(moderation) = contract.config().moderation() else {
            return refuse(ContractModerationNotEnabledError::new(contract_id, list).into());
        };
        if !moderation.keeps(list) {
            return refuse(ContractModerationNotEnabledError::new(contract_id, list).into());
        }

        let owner_id = contract.owner_id();
        if !moderation.may_moderate(&owner_id, &moderator_id) {
            return refuse(
                IdentityNotContractModeratorError::new(contract_id, moderator_id).into(),
            );
        }
        // Whoever may moderate (the owner and the moderators) cannot be put on a list. They can
        // be taken off one: a contract update may name as moderator an identity that already
        // carries an entry, and without the removal that entry could only be lifted by demoting
        // the moderator first.
        let adds_an_entry = matches!(
            action,
            ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Suspend { .. }
        );
        if adds_an_entry && moderation.may_moderate(&owner_id, &target_id) {
            return refuse(
                ContractModerationTargetNotAllowedError::new(contract_id, target_id).into(),
            );
        }

        if !validate_identity_exists(
            platform.drive,
            &target_id,
            execution_context,
            tx,
            platform_version,
        )? {
            return refuse(
                ContractModerationTargetNotFoundError::new(contract_id, target_id).into(),
            );
        }

        let lists = lists_to_read(moderation, action, list);
        let (fee, status) = platform.drive.fetch_contract_moderation_status_with_fee(
            contract_id,
            target_id,
            &lists,
            &block_info.epoch,
            tx,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

        if let Some(error) = refusal_for_status(action, target_id, &status, contract_id, block_info)
        {
            return refuse(error);
        }

        Ok(ConsensusValidationResult::new_with_data(
            ContractUserModerationTransitionAction::from_borrowed_transition_with_status(
                self, &status,
            )
            .into(),
        ))
    }
}

/// A document deletion: the document type exists and says moderators may delete its
/// documents, the signer is the contract's owner or one of its moderators, the document
/// exists, and it is not the owner's or a moderator's. Every refusal is paid for by
/// bumping the signer's contract nonce.
///
/// The action carries the contract, the document's owner and whether a removal record of
/// the same document id is already stored, so Drive deletes the document and writes its
/// record without reading again. No token is charged: the deletion cost of a document type
/// is what a document's own owner pays.
#[allow(clippy::too_many_arguments)]
fn transform_document_deletion_v0<C: CoreRPCLike>(
    transition: &ContractUserModerationTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type_name: &str,
    document_id: Identifier,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
    let contract = &contract_fetch_info.contract;
    let contract_id = contract.id();
    let moderator_id = transition.owner_id();
    let refuse = |error: ConsensusError| {
        Ok(ConsensusValidationResult::new_with_data_and_errors(
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                    transition,
                ),
            ),
            vec![error],
        ))
    };

    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        return refuse(
            InvalidDocumentTypeError::new(document_type_name.to_string(), contract_id).into(),
        );
    };
    // The keyword is only admitted on a contract that declares moderation, so a document
    // type that carries it always has moderators; one that does not reads the same as a
    // contract that has none.
    let moderation = contract
        .config()
        .moderation()
        .filter(|_| document_type.documents_can_be_deleted_by_moderators());
    let Some(moderation) = moderation else {
        return refuse(
            DocumentTypeNotDeletableByModeratorsError::new(
                contract_id,
                document_type_name.to_string(),
            )
            .into(),
        );
    };

    let owner_id = contract.owner_id();
    if !moderation.may_moderate(&owner_id, &moderator_id) {
        return refuse(IdentityNotContractModeratorError::new(contract_id, moderator_id).into());
    }

    let Some(document) = fetch_document_with_id(
        platform.drive,
        contract,
        document_type,
        document_id,
        &block_info.epoch,
        execution_context,
        tx,
        platform_version,
    )?
    else {
        return refuse(ConsensusError::StateError(
            StateError::DocumentNotFoundError(DocumentNotFoundError::new(document_id)),
        ));
    };

    // What protects the owner and the moderators from a ban protects their documents:
    // the owner demotes a moderator by a contract update before deleting what it wrote.
    let document_owner_id = document.owner_id();
    if moderation.may_moderate(&owner_id, &document_owner_id) {
        return refuse(
            ContractModerationTargetNotAllowedError::new(contract_id, document_owner_id).into(),
        );
    }

    // A document id can be created again by its author; removing it again replaces the
    // record, which Drive must know to write one operation on that key, not two.
    let (fee, existing_removal) = platform.drive.fetch_contract_document_removal_with_fee(
        contract_id,
        document_type_name,
        document_id,
        &block_info.epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));

    Ok(ConsensusValidationResult::new_with_data(
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_deletion(
            transition,
            ContractDocumentDeletionContext {
                data_contract_fetch_info: Arc::clone(contract_fetch_info),
                document_owner_id,
                removed_at: block_info.time_ms,
                replaces_existing_removal: existing_removal.is_some(),
            },
        )
        .into(),
    ))
}

/// The list the action edits, `None` for a document deletion, which edits none.
fn list_of(action: &ContractUserModerationAction) -> Option<ContractModerationList> {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Unban { .. } => {
            Some(ContractModerationList::Banlist)
        }
        ContractUserModerationAction::Suspend { .. }
        | ContractUserModerationAction::Unsuspend { .. } => {
            Some(ContractModerationList::Suspensions)
        }
        ContractUserModerationAction::DeleteDocument { .. } => None,
    }
}

/// The lists the action needs to know about: its own, and for a ban or a suspend the other
/// one as well, because a ban removes a suspension and a suspend is refused for a banned
/// identity. Only lists the contract keeps are read.
fn lists_to_read(
    moderation: &ContractModerationConfig,
    action: &ContractUserModerationAction,
    list: ContractModerationList,
) -> Vec<ContractModerationList> {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Suspend { .. } => {
            moderation.lists().collect()
        }
        ContractUserModerationAction::Unban { .. }
        | ContractUserModerationAction::Unsuspend { .. }
        | ContractUserModerationAction::DeleteDocument { .. } => vec![list],
    }
}

/// The first rule the target's status breaks for this action, if any.
fn refusal_for_status(
    action: &ContractUserModerationAction,
    target_id: Identifier,
    status: &ContractModerationStatus,
    contract_id: Identifier,
    block_info: &BlockInfo,
) -> Option<ConsensusError> {
    match action {
        ContractUserModerationAction::Ban { .. } => status
            .banned()
            .then(|| ContractUserAlreadyBannedError::new(contract_id, target_id).into()),
        ContractUserModerationAction::Unban { .. } => (!status.banned())
            .then(|| ContractUserNotBannedError::new(contract_id, target_id).into()),
        ContractUserModerationAction::Suspend { until, .. } => {
            // A suspend blocked by a ban is not a duplicate ban: it gets the "is banned" code.
            if status.banned() {
                return Some(ContractUserBannedError::new(contract_id, target_id).into());
            }
            (*until <= block_info.time_ms).then(|| {
                ContractSuspensionNotInFutureError::new(
                    contract_id,
                    target_id,
                    *until,
                    block_info.time_ms,
                )
                .into()
            })
        }
        ContractUserModerationAction::Unsuspend { .. } => status
            .suspension
            .is_none()
            .then(|| ContractUserNotSuspendedError::new(contract_id, target_id).into()),
        // A document deletion reads no list.
        ContractUserModerationAction::DeleteDocument { .. } => None,
    }
}
