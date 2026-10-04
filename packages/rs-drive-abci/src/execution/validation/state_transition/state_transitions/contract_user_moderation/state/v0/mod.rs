use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::moderators::{
    next_moderation_action_count, ModeratorSeat, Moderators,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::{
    SeatedModerationCharter, TeamSeat,
};
use crate::execution::validation::state_transition::common::validate_document_not_expired::validate_document_not_expired;
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::state_transitions::batch::{
    fetch_document_with_id, read_property_constraint_aggregates_for_moderator_change,
};
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::contract_moderation::InvalidContractModerationDocumentFieldsError;
use dpp::consensus::basic::decode::DecodingError;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::contract_moderation::{
    ContractDocumentAlreadyRestoredError, ContractDocumentRemovalNotFoundError,
    ContractModerationAbilityNotGrantedError, ContractModerationNotEnabledError,
    ContractModerationTargetNotAllowedError, ContractModerationTargetNotFoundError,
    ContractModerationTeamNotSeatedError, ContractSuspensionNotInFutureError,
    ContractTeamActionAlreadyCompletedError, ContractTeamActionAlreadySignedError,
    ContractTeamActionDocumentChangedError, ContractTeamActionDoesNotExistError,
    ContractTeamMemberAddedAfterDocumentError, ContractUserAlreadyBannedError,
    ContractUserBannedError, ContractUserNotBannedError, ContractUserNotSuspendedError,
    ContractUserNotWarnedError, ContractUserWarningLimitReachedError,
    DocumentFieldNotChangeableByModeratorsError, DocumentModerationWindowElapsedError,
    DocumentNotSettledError, DocumentRestoreHashMismatchError, DocumentRestoreWindowElapsedError,
    DocumentTypeNotDeletableByModeratorsError, DocumentTypeNotDeletableOnceSettledError,
    IdentityNotContractModeratorError, SettledDeletionNotRestorableError,
};
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    encode_kept_fields, ContractDocumentRemoval, ContractDocumentRestoration,
    ContractModerationConfig, ContractModerationList, ContractModerationReason,
    ContractModerationStatus, ContractTeamAction, ContractTeamActionEvent, ModerationAbility,
    SettledDeletionRule,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::TimestampMillis;
use dpp::platform_value::Value;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransitionOwned;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use drive::drive::contract::moderation::types::ContractTeamActionWrite;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentChangeContext, ContractDocumentDeletionContext,
    ContractDocumentRemovalRecordContext, ContractDocumentRestorationContext,
    ContractTeamActionContext,
};
use drive::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// `moderatorAbilities.deleteWithin` is declared in seconds, as the other durations of a document
/// type are; block time, `$updatedAt` and `$createdAt` are in milliseconds.
const MILLIS_PER_SECOND: u64 = 1_000;

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
    /// A document deletion is checked by `transform_document_deletion_v0`, a document
    /// restore by `transform_document_restore_v0` and a document field change by
    /// `transform_document_fields_change_v0`. For the rest:
    /// reads the contract and the target's status and checks the moderation: the contract
    /// keeps the list the action edits, the signer moderates the contract (see [`Moderators`]:
    /// on an elected contract with a seated charter, the charter's team, which must also hold
    /// the ability the list needs), the target is not protected and exists, and the action
    /// fits the target's status (a warn fits while the target carries fewer than
    /// `SystemLimits::max_contract_warnings_per_identity` warnings). Every refusal, a contract
    /// that does not exist included, is paid for by bumping the signer's contract nonce.
    ///
    /// The action carries what Drive needs of the target's status as read here (and for a
    /// warn the block time the warning is stamped with), so Drive edits the lists without
    /// reading them again, and the mempool, which transforms without a state validation
    /// stage, refuses with the same consensus codes as a block.
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
            ContractUserModerationAction::RestoreDocument {
                document_type_name,
                document,
            } => {
                return transform_document_restore_v0(
                    self,
                    platform,
                    block_info,
                    &contract_fetch_info,
                    document_type_name,
                    document.as_slice(),
                    execution_context,
                    tx,
                    platform_version,
                );
            }
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name,
                document_id,
                reason,
            } => {
                return transform_settled_deletion_proposal_v0(
                    self,
                    platform,
                    block_info,
                    &contract_fetch_info,
                    document_type_name,
                    *document_id,
                    reason,
                    execution_context,
                    tx,
                    platform_version,
                );
            }
            ContractUserModerationAction::ApproveTeamAction { action_id } => {
                return transform_team_action_approval_v0(
                    self,
                    platform,
                    block_info,
                    &contract_fetch_info,
                    *action_id,
                    execution_context,
                    tx,
                    platform_version,
                );
            }
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name,
                document_id,
                fields,
                ..
            } => {
                return transform_document_fields_change_v0(
                    self,
                    platform,
                    block_info,
                    &contract_fetch_info,
                    document_type_name,
                    *document_id,
                    fields,
                    execution_context,
                    tx,
                    platform_version,
                );
            }
            ContractUserModerationAction::Ban { identity_id, .. }
            | ContractUserModerationAction::Unban { identity_id }
            | ContractUserModerationAction::Suspend { identity_id, .. }
            | ContractUserModerationAction::Unsuspend { identity_id }
            | ContractUserModerationAction::Warn { identity_id, .. }
            | ContractUserModerationAction::ClearWarnings { identity_id } => *identity_id,
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
        let epoch = &block_info.epoch;
        let moderators = Moderators::read(
            moderation,
            contract_id,
            platform.drive,
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        if !moderators.may_moderate(
            owner_id,
            moderator_id,
            platform.drive,
            epoch,
            execution_context,
            tx,
            platform_version,
        )? {
            return refuse(
                IdentityNotContractModeratorError::new(contract_id, moderator_id).into(),
            );
        }
        // The lists are contract-wide: a seated team uses one when the declaration gives it
        // the ability on some moderated document type.
        let ability = ability_of(list);
        if moderators.lacks(ability, None) {
            return refuse(
                ContractModerationAbilityNotGrantedError::new(contract_id, ability, None).into(),
            );
        }
        if let Some(error) = moderators.unlisted_reason(
            action,
            contract_id,
            platform.drive,
            epoch,
            execution_context,
            tx,
            platform_version,
        )? {
            return refuse(error);
        }
        // Whoever the contract protects (the owner and the moderators, and the owner of an
        // elected contract whose declaration says so) cannot be put on a list. They can
        // be taken off one: a contract update may name as moderator an identity that already
        // carries an entry, and without the removal that entry could only be lifted by demoting
        // the moderator first.
        let adds_an_entry = matches!(
            action,
            ContractUserModerationAction::Ban { .. }
                | ContractUserModerationAction::Suspend { .. }
                | ContractUserModerationAction::Warn { .. }
        );
        if adds_an_entry
            && moderators.protects(
                owner_id,
                target_id,
                platform.drive,
                epoch,
                execution_context,
                tx,
                platform_version,
            )?
        {
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

        if let Some(error) = refusal_for_status(
            action,
            target_id,
            &status,
            contract_id,
            block_info,
            platform_version,
        ) {
            return refuse(error);
        }

        let moderation_action =
            ContractUserModerationTransitionAction::from_borrowed_transition_with_status(
                self,
                &status,
                block_info.time_ms,
            );
        let moderation_action = moderators.count_for_signer(
            moderation_action,
            platform.drive,
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        Ok(ConsensusValidationResult::new_with_data(
            moderation_action.into(),
        ))
    }
}

/// A document deletion: the document type exists and says moderators may delete its
/// documents, the signer moderates the contract (see [`Moderators`]: a seated team must also
/// hold `deleteDocuments` on the type), the document exists, its owner is not protected, and it
/// was last modified within the window the document type gives its moderators, if it gives one.
/// Every refusal is paid for by bumping the signer's contract nonce.
///
/// The action carries the contract and the document's owner, so Drive deletes the document
/// and writes its record without reading again; a type whose moderators' deletions keep no
/// record (`moderatorAbilities.deleteKeepsRecord: false`) gets none, nothing is hashed and no
/// record read. The owner forfeits its storage refund unless the type gives it back
/// (`moderatorAbilities.deleteRefundsOwner`). Nothing the document type prices is charged,
/// neither its deletion token cost nor its `actionFees` deletion fee: both are what a
/// document's own owner pays for deleting it, and a moderator removes content on the
/// contract's behalf.
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

    let epoch = &block_info.epoch;
    let moderators = Moderators::read(
        moderation,
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    if let Err(error) = deletion_authority(
        &moderators,
        contract,
        moderator_id,
        document_type_name,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(error);
    }
    if let Some(error) = moderators.unlisted_reason(
        transition.action(),
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(error);
    }
    let document = match fetch_deletable_document(
        &moderators,
        contract,
        document_type,
        document_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        Ok(document) => document,
        Err(error) => return refuse(error),
    };

    // A document type may give its moderators a window: so many seconds after a document's
    // last modification, past which the document is settled and no moderator deletes it (its
    // own owner's deletion is `canBeDeleted`'s business, at any age). The last modification is
    // `$updatedAt`, which a replace moves, opening the window again since what it wrote is new
    // content; a type whose documents never change may carry `$createdAt` alone, and that is
    // then the clock. The type requires one of the two, so every document carries it; one that
    // carried neither would read as modified at time zero, which is settled: the refusal that
    // protects the author.
    if let Some(window_seconds) = document_type.documents_can_be_deleted_by_moderators_for() {
        let (last_modified_at, settled_at) =
            last_modified_and_settled_at(&document, window_seconds);
        if block_info.time_ms > settled_at {
            return refuse(
                DocumentModerationWindowElapsedError::new(
                    contract_id,
                    document_id,
                    last_modified_at,
                    window_seconds,
                    block_info.time_ms,
                )
                .into(),
            );
        }
    }

    let deletion = document_deletion_context(
        platform,
        contract_fetch_info,
        document_type,
        document_type_name,
        &document,
        block_info,
        execution_context,
        tx,
        platform_version,
    )?;
    let moderation_action =
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_deletion(
            transition, deletion,
        );
    let moderation_action = moderators.count_for_signer(
        moderation_action,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    Ok(ConsensusValidationResult::new_with_data(
        moderation_action.into(),
    ))
}

/// The proposal of a settled document's deletion: the document type exists and says who of a
/// seated team must approve the deletion of its settled documents
/// (`moderatorAbilities.deleteSettled`), a team is seated on the contract, the signer is on it
/// and the declaration gives the team `deleteDocuments` on the type, the reason is one the team's
/// proposal lists, the document exists, its owner is not protected, it is settled: last
/// modified longer ago than the type's `deleteWithin` window, within which a moderator deletes it
/// alone, and a signer the leader added was added before the document was created, unless the
/// rule admits later additions (`approversPredateDocument: false`). Every refusal is paid for by
/// bumping the signer's contract nonce.
///
/// The proposal is kept as a team action, by the id it commits to
/// ([`ContractUserModerationTransition::team_action_id`]), naming the document as it is now
/// (its last modification and revision) and the proposer's reason, with the proposer's
/// approval. When the proposer meets the rule alone (the leader, under a rule of one approval
/// the leader may give) the document is deleted at once, as a moderator's `DeleteDocument`
/// deletes it, and the action is kept closed; the proposer's moderation action count goes up by
/// one. Otherwise the other members approve it by its id (`ApproveTeamAction`), and nobody is
/// counted yet.
#[allow(clippy::too_many_arguments)]
fn transform_settled_deletion_proposal_v0<C: CoreRPCLike>(
    transition: &ContractUserModerationTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type_name: &str,
    document_id: Identifier,
    reason: &ContractModerationReason,
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
    // The keyword is only admitted beside `delete` and `deleteWithin`, on a contract whose
    // moderators are elected, so a type that carries it always has all three.
    let (Some(moderation), Some(rule), Some(window_seconds)) = (
        contract.config().moderation(),
        document_type.moderator_settled_deletion(),
        document_type.documents_can_be_deleted_by_moderators_for(),
    ) else {
        return refuse(
            DocumentTypeNotDeletableOnceSettledError::new(
                contract_id,
                document_type_name.to_string(),
            )
            .into(),
        );
    };

    let epoch = &block_info.epoch;
    let moderators = Moderators::read(
        moderation,
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    // Only a seated team deletes a settled document: the rule names its leader and counts its
    // members, and the moderators of an interim are neither.
    let Moderators::Seated { elected, charter } = &moderators else {
        return refuse(ContractModerationTeamNotSeatedError::new(contract_id).into());
    };
    // Who deletes which document is judged as for a moderator's deletion, the team's own
    // documents as protected from a settled deletion as from any other.
    let seat = match deletion_authority(
        &moderators,
        contract,
        moderator_id,
        document_type_name,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        Ok(seat) => seat,
        Err(error) => return refuse(error),
    };
    if let Some(error) = moderators.unlisted_reason(
        transition.action(),
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(error);
    }
    let document = match fetch_deletable_document(
        &moderators,
        contract,
        document_type,
        document_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        Ok(document) => document,
        Err(error) => return refuse(error),
    };

    // The window of a moderator's deletion, measured the same way: the boundary millisecond
    // is still inside it.
    let (last_modified_at, settled_at) = last_modified_and_settled_at(&document, window_seconds);
    if block_info.time_ms <= settled_at {
        return refuse(
            DocumentNotSettledError::new(
                contract_id,
                document_id,
                last_modified_at,
                window_seconds,
                block_info.time_ms,
            )
            .into(),
        );
    }
    if let Some(error) = late_addition_refusal(contract_id, &rule, moderator_id, &seat, &document) {
        return refuse(error);
    }

    let action_id = transition.team_action_id().ok_or(Error::Execution(
        ExecutionError::CorruptedCodeExecution(
            "the proposal of a settled document's deletion has a team action id",
        ),
    ))?;
    let action = ContractTeamAction {
        proposer_id: moderator_id,
        proposed_at: block_info.time_ms,
        event: ContractTeamActionEvent::DeleteSettledDocument {
            document_type_name: document_type_name.to_string(),
            document_id,
            document_last_modified_at: last_modified_at,
            document_revision: document.revision(),
            reason: reason.clone(),
        },
    };
    let team_capacity = usize::from(charter.charter.seats(elected.max_added_moderators));
    let closes = rule.is_met_by(&[moderator_id], charter.leader_id, team_capacity);
    let (deletion, approver_action_counts) = if closes {
        let (deletion, counts) = run_settled_deletion(
            platform,
            contract_fetch_info,
            document_type,
            document_type_name,
            &document,
            &[moderator_id],
            block_info,
            execution_context,
            tx,
            platform_version,
        )?;
        (Some(deletion), counts)
    } else {
        (None, vec![])
    };
    let write = ContractTeamActionWrite::Propose { action, closes };

    Ok(ConsensusValidationResult::new_with_data(
        ContractUserModerationTransitionAction::from_borrowed_transition_with_team_action(
            transition,
            ContractTeamActionContext {
                action_id,
                write,
                deletion,
                approver_action_counts,
            },
        )
        .into(),
    ))
}

/// The approval of a team action another member proposed: the contract keeps team actions
/// (a document type of it sets `moderatorAbilities.deleteSettled`), the action exists and has not
/// run, a team is seated on it, the signer is on the team with the authority the action
/// needs (for the deletion of a settled document, `deleteDocuments` on its type), the document
/// the action names still exists, its owner not protected, as it was when proposed: an approval
/// of a document changed since would approve the deletion of content the team never saw, a
/// signer the leader added was added before the document was created, unless the rule admits
/// later additions, and the signer has not approved it already. Every refusal is paid for by
/// bumping the signer's contract nonce.
///
/// The approval is added to the action's. When the approvals given could meet the rule, the team
/// is read: the approvals of members who left since no longer count and are dropped, refunded to
/// them (one back after that approves again), as are those of members the leader took off and
/// added again after the document was created, under a rule admitting only members from before
/// it, and when those of members still on the team
/// (the leader among them, when the rule says so) meet the rule, counting this one, the action
/// runs: the document is deleted as a moderator's `DeleteDocument` deletes it, for the
/// proposal's reason, the action closes with the approvals that counted, and every counted
/// approver's moderation action count goes up by one: a deletion signed by several counts for
/// each of them. An approval that falls short counts for nobody, so that approving what never
/// passes earns no share of the moderators pot.
#[allow(clippy::too_many_arguments)]
fn transform_team_action_approval_v0<C: CoreRPCLike>(
    transition: &ContractUserModerationTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    action_id: Identifier,
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

    // A contract keeps team actions only when a document type sets `deleteSettled`, which needs
    // an elected declaration: any other has none to approve, and no tree to read them from.
    let moderation = contract
        .config()
        .moderation()
        .filter(|_| contract.keeps_team_actions());
    let Some(moderation) = moderation else {
        return refuse(ContractTeamActionDoesNotExistError::new(contract_id, action_id).into());
    };

    // The action first: its refusals need nothing of the team. An action exists only once a
    // seated member proposed it, and in protocol version 14 a seat is never replaced, so a team
    // is seated whenever one is found.
    let epoch = &block_info.epoch;
    let (action_fee, found) = platform.drive.fetch_contract_team_action_with_fee(
        contract_id,
        action_id,
        epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(action_fee));
    let action = match found {
        None => {
            return refuse(ContractTeamActionDoesNotExistError::new(contract_id, action_id).into())
        }
        Some((GroupActionStatus::ActionClosed, _)) => {
            return refuse(
                ContractTeamActionAlreadyCompletedError::new(contract_id, action_id).into(),
            )
        }
        Some((GroupActionStatus::ActionActive, action)) => action,
    };

    let moderators = Moderators::read(
        moderation,
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    let Moderators::Seated { elected, charter } = &moderators else {
        return refuse(ContractModerationTeamNotSeatedError::new(contract_id).into());
    };
    let ContractTeamActionEvent::DeleteSettledDocument {
        document_type_name,
        document_id,
        ..
    } = &action.event;
    let document_id = *document_id;

    // The type was there when the member proposed, with its rule, and neither is ever taken
    // away: a contract update keeps every document type and freezes the keyword.
    let document_type = contract.document_type_optional_for_name(document_type_name);
    let (Some(document_type), Some(rule), Some(window_seconds)) = (
        document_type,
        document_type.and_then(|document_type| document_type.moderator_settled_deletion()),
        document_type
            .and_then(|document_type| document_type.documents_can_be_deleted_by_moderators_for()),
    ) else {
        return Err(Error::Execution(ExecutionError::CorruptedDriveResponse(
            format!(
                "team action {} of contract {} names document type {}, which no longer deletes \
                 settled documents",
                action_id, contract_id, document_type_name
            ),
        )));
    };
    let seat = match deletion_authority(
        &moderators,
        contract,
        moderator_id,
        document_type_name,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        Ok(seat) => seat,
        Err(error) => return refuse(error),
    };

    let (signers_fee, signers) = platform.drive.fetch_contract_team_action_signers_with_fee(
        contract_id,
        GroupActionStatus::ActionActive,
        action_id,
        epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(signers_fee));

    let document = match fetch_deletable_document(
        &moderators,
        contract,
        document_type,
        document_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        Ok(document) => document,
        Err(error) => return refuse(error),
    };
    // Of the document as proposed: its revision too, which a moderator's change of its fields
    // moves while leaving `$updatedAt`, and so the settling time, alone. The document was
    // settled then and time only moves on, so it still is.
    let (last_modified_at, _) = last_modified_and_settled_at(&document, window_seconds);
    if !action.names_document_as(last_modified_at, document.revision()) {
        return refuse(
            ContractTeamActionDocumentChangedError::new(contract_id, action_id, document_id).into(),
        );
    }
    if let Some(error) = late_addition_refusal(contract_id, &rule, moderator_id, &seat, &document) {
        return refuse(error);
    }
    // After the late addition: a member the leader took off and added again after the document
    // may still have an approval here, which no longer counts and goes at the next team read.
    if signers.contains(&moderator_id) {
        return refuse(
            ContractTeamActionAlreadySignedError::new(contract_id, action_id, moderator_id).into(),
        );
    }

    // The most members the seated team can hold (`ElectedCharter::seats`): its
    // leader, the members its charter elected and those the declaration lets the leader add. A
    // rule asking for more asks for all of them, so that a charter electing fewer members than
    // the rule allows for can still meet it. A member the leader removed still counts: the
    // leader can not lower the bar by removing members who would not approve, and gets the
    // seat back by deleting the removal.
    let team_capacity = usize::from(charter.charter.seats(elected.max_added_moderators));
    // Approvals that can not meet the rule, even if every one still counts, need no team read,
    // and leave the approvals of members who left where they are until one that reads the team:
    // too few of them, or none of them, this one included, the leader's when the rule needs it.
    let could_meet = signers.len() + 1 >= rule.approvals_needed(team_capacity)
        && (!rule.leader
            || moderator_id == charter.leader_id
            || signers.contains(&charter.leader_id));
    let (write, deletion, approver_action_counts) = if !could_meet {
        (
            ContractTeamActionWrite::Approve {
                dropped_signers: vec![],
            },
            None,
            vec![],
        )
    } else {
        // The approvals of members still on the team count, this one among them; those of
        // members who left are dropped, so that a member who comes back after this approves
        // again, and so are those of members the leader took off and added again too late for
        // the document under a rule admitting only members from before it.
        let earlier_counted = still_counted_approvers(
            &signers,
            charter,
            elected.max_added_moderators,
            &rule,
            document.created_at().unwrap_or_default(),
            platform.drive,
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        let dropped_signers: Vec<Identifier> = signers
            .iter()
            .copied()
            .filter(|signer| !earlier_counted.contains(signer))
            .collect();
        let mut counted = earlier_counted.clone();
        counted.push(moderator_id);
        if rule.is_met_by(&counted, charter.leader_id, team_capacity) {
            let (deletion, counts) = run_settled_deletion(
                platform,
                contract_fetch_info,
                document_type,
                document_type_name,
                &document,
                &counted,
                block_info,
                execution_context,
                tx,
                platform_version,
            )?;
            (
                ContractTeamActionWrite::Close {
                    action: action.clone(),
                    earlier_signers: earlier_counted,
                    dropped_signers,
                },
                Some(deletion),
                counts,
            )
        } else {
            (
                ContractTeamActionWrite::Approve { dropped_signers },
                None,
                vec![],
            )
        }
    };

    Ok(ConsensusValidationResult::new_with_data(
        ContractUserModerationTransitionAction::from_borrowed_transition_with_team_action(
            transition,
            ContractTeamActionContext {
                action_id,
                write,
                deletion,
                approver_action_counts,
            },
        )
        .into(),
    ))
}

/// What a team action that deletes a settled document runs once the approvals meet its rule:
/// what the deletion reads, as a moderator's `DeleteDocument` does (the action's event names the
/// document and the reason Drive deletes it for), and the moderation action count of every
/// approver in `counted`, one point read each, billed. A
/// contract stored elected before the counts existed has nowhere to count, which the first read
/// finds.
#[allow(clippy::too_many_arguments)]
fn run_settled_deletion<C: CoreRPCLike>(
    platform: &PlatformRef<C>,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type: DocumentTypeRef,
    document_type_name: &str,
    document: &Document,
    counted: &[Identifier],
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<(ContractDocumentDeletionContext, Vec<(Identifier, u32)>), Error> {
    let context = document_deletion_context(
        platform,
        contract_fetch_info,
        document_type,
        document_type_name,
        document,
        block_info,
        execution_context,
        tx,
        platform_version,
    )?;
    let contract_id = contract_fetch_info.contract.id();
    let mut counts = Vec::with_capacity(counted.len());
    for approver in counted.iter().copied() {
        let Some(count) = next_moderation_action_count(
            platform.drive,
            contract_id,
            approver,
            &block_info.epoch,
            execution_context,
            tx,
            platform_version,
        )?
        else {
            break;
        };
        counts.push((approver, count));
    }
    Ok((context, counts))
}

/// When `document` last changed, and when it settles under a moderators' window of
/// `window_seconds`: its `$updatedAt`, or `$createdAt` on a type that carries no `$updatedAt`,
/// and that time plus the window. A document that carried neither reads as changed at time
/// zero, which is settled. A moderator's deletion is refused after the settling time (41116) and
/// the proposal of a settled document's deletion up to it (41206): measured once, the two can
/// neither overlap nor leave a gap.
fn last_modified_and_settled_at(document: &Document, window_seconds: u32) -> (u64, u64) {
    let last_modified_at = document
        .updated_at()
        .or(document.created_at())
        .unwrap_or_default();
    let settled_at = last_modified_at
        .saturating_add(u64::from(window_seconds).saturating_mul(MILLIS_PER_SECOND));
    (last_modified_at, settled_at)
}

/// Whether `moderators` let `moderator_id` delete documents of `document_type_name` on
/// `contract`, for a moderator's deletion and a team's settled deletion alike: the signer
/// moderates the contract (41101), and a seated team holds `deleteDocuments` on the type
/// (41201). Where the signer sits when they do, which a settled deletion's rule reads
/// ([`late_addition_refusal`]); the refusal when they do not, the caller's to pay.
#[allow(clippy::too_many_arguments)]
fn deletion_authority(
    moderators: &Moderators,
    contract: &DataContract,
    moderator_id: Identifier,
    document_type_name: &str,
    drive: &Drive,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Result<ModeratorSeat, ConsensusError>, Error> {
    let contract_id = contract.id();
    let Some(seat) = moderators.seat_of(
        contract.owner_id(),
        moderator_id,
        drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?
    else {
        return Ok(Err(IdentityNotContractModeratorError::new(
            contract_id,
            moderator_id,
        )
        .into()));
    };
    if moderators.lacks(ModerationAbility::DeleteDocuments, Some(document_type_name)) {
        return Ok(Err(ContractModerationAbilityNotGrantedError::new(
            contract_id,
            ModerationAbility::DeleteDocuments,
            Some(document_type_name.to_string()),
        )
        .into()));
    }
    Ok(Ok(seat))
}

/// The refusal of the proposal or approval of `document`'s settled deletion by `member_id`,
/// sitting in `seat`, when it is a member the leader added no earlier than the document was
/// created and `rule` admits only members added before it (`approversPredateDocument`,
/// 41212): the leader could otherwise add members to approve whatever it proposes. The leader
/// and the elected members always count. A type whose rule says so requires `$createdAt`, so
/// every document of it records one.
fn late_addition_refusal(
    contract_id: Identifier,
    rule: &SettledDeletionRule,
    member_id: Identifier,
    seat: &ModeratorSeat,
    document: &Document,
) -> Option<ConsensusError> {
    let ModeratorSeat::Team(TeamSeat::Added { added_at }) = seat else {
        return None;
    };
    let document_created_at = document.created_at().unwrap_or_default();
    (!rule.admits_addition(*added_at, document_created_at)).then(|| {
        ContractTeamMemberAddedAfterDocumentError::new(
            contract_id,
            member_id,
            *added_at,
            document.id(),
            document_created_at,
        )
        .into()
    })
}

/// The document a moderator's deletion, or a team's settled deletion, names, read and
/// billed; or the refusal: it does not exist (40101), or its owner is protected (41102). What
/// protects the owner and the moderators from a ban protects their documents: the owner demotes
/// a moderator by a contract update before deleting what it wrote, and the leader of a seated
/// team removes a member before anyone deletes what it wrote.
#[allow(clippy::too_many_arguments)]
fn fetch_deletable_document(
    moderators: &Moderators,
    contract: &DataContract,
    document_type: DocumentTypeRef,
    document_id: Identifier,
    drive: &Drive,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Result<Document, ConsensusError>, Error> {
    let Some(document) = fetch_document_with_id(
        drive,
        contract,
        document_type,
        document_id,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?
    else {
        return Ok(Err(ConsensusError::StateError(
            StateError::DocumentNotFoundError(DocumentNotFoundError::new(document_id)),
        )));
    };
    let document_owner_id = document.owner_id();
    if moderators.protects(
        contract.owner_id(),
        document_owner_id,
        drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return Ok(Err(ContractModerationTargetNotAllowedError::new(
            contract.id(),
            document_owner_id,
        )
        .into()));
    }
    Ok(Ok(document))
}

/// Which of `approvers`, earlier approvals of a team action deleting a document created at
/// `document_created_at`, still count toward `rule`, in their order: those still on the seated
/// team, and of the members the leader added, only those `rule` admits for the document
/// ([`TeamSeat::counts_toward`]): one taken off and added again since approving sits in a new
/// addition, which may come too late. The leader always counts. The others are checked one
/// point read each ([`SeatedModerationCharter::seat_of`]) while there are no more of them than
/// the queries a read of the whole team makes, and past that by reading the team once
/// ([`SeatedModerationCharter::fetch_active_seats`]): the same answer either way, for fewer
/// billed reads.
#[allow(clippy::too_many_arguments)]
fn still_counted_approvers(
    approvers: &[Identifier],
    charter: &SeatedModerationCharter,
    max_added_moderators: u16,
    rule: &SettledDeletionRule,
    document_created_at: TimestampMillis,
    drive: &Drive,
    epoch: &Epoch,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<Vec<Identifier>, Error> {
    let others = approvers
        .iter()
        .filter(|approver| **approver != charter.leader_id)
        .count();
    // `fetch_active_seats` reads the removals when the charter elected anyone, and the
    // additions when the declaration allows any.
    let team_read_queries =
        usize::from(!charter.charter.members.is_empty()) + usize::from(max_added_moderators > 0);
    if others > team_read_queries {
        let active = charter.fetch_active_seats(
            drive,
            max_added_moderators,
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        return Ok(approvers
            .iter()
            .copied()
            .filter(|approver| {
                *approver == charter.leader_id
                    || active
                        .get(approver)
                        .is_some_and(|seat| seat.counts_toward(rule, document_created_at))
            })
            .collect());
    }
    let mut still_counted = Vec::with_capacity(approvers.len() + 1);
    for approver in approvers.iter().copied() {
        let seat = charter.seat_of(
            drive,
            approver,
            epoch,
            execution_context,
            tx,
            platform_version,
        )?;
        if seat.is_some_and(|seat| seat.counts_toward(rule, document_created_at)) {
            still_counted.push(approver);
        }
    }
    Ok(still_counted)
}

/// What Drive needs to delete `document` as a moderator deletes it: the contract, the document's
/// owner, the removal record when the type keeps one, and whether the owner is refunded. The
/// record commits to the document as serialized under its type, and is read first, billed, for
/// a restored record it replaces. Shared by a moderator's deletion and the team action that
/// deletes a settled document once its approvals meet the type's rule.
#[allow(clippy::too_many_arguments)]
fn document_deletion_context<C: CoreRPCLike>(
    platform: &PlatformRef<C>,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type: DocumentTypeRef,
    document_type_name: &str,
    document: &Document,
    block_info: &BlockInfo,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ContractDocumentDeletionContext, Error> {
    let contract = &contract_fetch_info.contract;
    let contract_id = contract.id();
    let document_id = document.id();
    // The removal record, when the type keeps one. What it commits to: the document as
    // serialized under its type, which a restore must bring back byte for byte. It is
    // serialized here, from the document as read, rather than hashed as stored: a document
    // stored under an earlier version of its type or of the serialization would never
    // re-serialize to its stored bytes, and a client keeping the document (not the bytes)
    // could never match them. A type that keeps no record has no records tree to read, and
    // nothing to restore from.
    let record = if document_type.moderator_deletions_keep_records() {
        let serialized = document.serialize(document_type, contract, platform_version)?;
        execution_context.add_operation(ValidationOperation::DoubleSha256(
            serialized.len() as u16 / SHA256_BLOCK_SIZE,
        ));
        let document_hash = hash_double(serialized);

        // A document id is produced at most once (it commits to the nonce of its create
        // transition), so the only record this id can already have is of a deletion a
        // moderator restored: the document is live again and this deletion writes a fresh
        // record in its place. An unrestored record beside a live document is a state no
        // transition produces.
        let (removal_fee, existing_removal) =
            platform.drive.fetch_contract_document_removal_with_fee(
                contract_id,
                document_type_name,
                document_id,
                &block_info.epoch,
                tx,
                platform_version,
            )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(removal_fee));
        let replaced_record = match existing_removal {
            None => None,
            Some(removal) if removal.is_restored() => Some(removal),
            Some(_) => {
                return Err(Error::Execution(ExecutionError::DriveIncoherence(
                    "a document with an unrestored moderation removal record exists",
                )))
            }
        };
        Some(ContractDocumentRemovalRecordContext {
            removed_at: block_info.time_ms,
            document_hash,
            replaced_record,
            // What of the document stays public once it is gone, copied from it as stored and
            // encoded as the document encodes its properties
            kept_fields: encode_kept_fields(document, document_type)?,
        })
    } else {
        None
    };

    Ok(ContractDocumentDeletionContext {
        data_contract_fetch_info: Arc::clone(contract_fetch_info),
        document_owner_id: document.owner_id(),
        record,
        refunds_owner: document_type.moderator_deletions_refund_owner(),
    })
}

/// A document field change: the document type exists and every field the change names is
/// one it keeps for its moderators (`moderatorAbilities.changeFields`), the signer moderates
/// the contract (see [`Moderators`]: a seated team must also hold `changeDocumentFields` on the
/// type) for a reason a seated team's proposal lists, the document exists and has not expired,
/// and the document as changed is still one of its type: its schema, the shapes of its
/// `encryptedFor` properties, its `distinctFrom` properties, its `propertyConstraints` with the
/// totals they read, and its unique indexes. Every refusal is paid for by bumping the signer's
/// contract nonce.
///
/// The fields are the moderators', not the document's owner's, so whoever owns the document,
/// the contract owner and the moderators included, is no protection. Nothing the document type
/// prices is charged: a moderator's change is no action of the owner's. The type's references
/// are not checked again: none of the fields a moderator writes holds a reference or is read by
/// one, which registration guarantees, and the references it does hold were checked when its
/// owner wrote them.
///
/// The action carries the contract and the changed document: the stored one with the fields set
/// (a `null` removing one), `$revision` one higher, and everything else as it was, `$updatedAt`
/// among it, so that Drive stores it without reading again. A moderator's change is not the
/// owner's modification: the window a type gives its moderators to delete a document is not
/// opened again by it.
#[allow(clippy::too_many_arguments)]
fn transform_document_fields_change_v0<C: CoreRPCLike>(
    transition: &ContractUserModerationTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type_name: &str,
    document_id: Identifier,
    fields: &BTreeMap<String, Value>,
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
    let contract = &contract_fetch_info.contract;
    let contract_id = contract.id();
    let moderator_id = transition.owner_id();
    let refuse_all = |errors: Vec<ConsensusError>| {
        Ok(ConsensusValidationResult::new_with_data_and_errors(
            StateTransitionAction::BumpIdentityDataContractNonceAction(
                BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                    transition,
                ),
            ),
            errors,
        ))
    };
    let refuse = |error: ConsensusError| refuse_all(vec![error]);

    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        return refuse(
            InvalidDocumentTypeError::new(document_type_name.to_string(), contract_id).into(),
        );
    };
    // The keyword is only admitted on a contract that declares moderation, so a type keeping
    // fields for its moderators always has moderators; a field it does not keep, on a contract
    // with no moderation among them, is refused the same.
    let not_changeable = |field: &str| {
        DocumentFieldNotChangeableByModeratorsError::new(
            contract_id,
            document_type_name.to_string(),
            field.to_string(),
        )
        .into()
    };
    let moderator_fields = document_type.moderator_changeable_fields();
    if let Some(field) = fields
        .keys()
        .find(|field| !moderator_fields.contains(*field))
    {
        return refuse(not_changeable(field));
    }
    let Some(moderation) = contract.config().moderation() else {
        let field = fields.keys().next().map(String::as_str).unwrap_or_default();
        return refuse(not_changeable(field));
    };

    let owner_id = contract.owner_id();
    let epoch = &block_info.epoch;
    let moderators = Moderators::read(
        moderation,
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    if !moderators.may_moderate(
        owner_id,
        moderator_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(IdentityNotContractModeratorError::new(contract_id, moderator_id).into());
    }
    if moderators.lacks(
        ModerationAbility::ChangeDocumentFields,
        Some(document_type_name),
    ) {
        return refuse(
            ContractModerationAbilityNotGrantedError::new(
                contract_id,
                ModerationAbility::ChangeDocumentFields,
                Some(document_type_name.to_string()),
            )
            .into(),
        );
    }
    if let Some(error) = moderators.unlisted_reason(
        transition.action(),
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(error);
    }

    let Some(stored) = fetch_document_with_id(
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

    // A document whose type declares a `ttl` and has expired is the platform's to delete: no
    // write of it is admitted any more.
    if let Some(error) = validate_document_not_expired(
        contract_id,
        document_type,
        document_id,
        stored.created_at(),
        block_info,
    )?
    .errors
    .into_iter()
    .next()
    {
        return refuse(error);
    }

    // The type keeps a revision on every document, since it keeps fields for its moderators; a
    // stored one without it is a state no transition produces.
    let Some(revision) = stored.revision() else {
        return Err(Error::Execution(ExecutionError::DriveIncoherence(
            "a document whose type keeps fields for its moderators has no revision",
        )));
    };
    let Some(next_revision) = revision.checked_add(1) else {
        return refuse(
            OverflowError::new(format!(
                "document {} can not be changed again: its revision is {}",
                document_id, revision
            ))
            .into(),
        );
    };

    // The fields whose value the change moves: the unique indexes reading them are checked,
    // and a field set to the value it holds is no change to check.
    let mut changed = stored.clone();
    let mut changed_fields = BTreeSet::new();
    for (field, value) in fields {
        let properties = changed.properties_mut();
        match value {
            Value::Null => {
                if properties.remove(field).is_some() {
                    changed_fields.insert(field.clone());
                }
            }
            // Compared as a replace compares: an integer is the same whichever width the
            // transition and the stored document carry it in.
            value => {
                let unchanged = properties
                    .get(field)
                    .is_some_and(|stored| stored.equal_underlying_data(value));
                if !unchanged {
                    properties.insert(field.clone(), value.clone());
                    changed_fields.insert(field.clone());
                }
            }
        }
    }
    // A change that changes nothing is refused rather than written: it would bump the revision,
    // refusing a replace its owner built meanwhile, for no change at all.
    if changed_fields.is_empty() {
        return refuse(
            InvalidContractModerationDocumentFieldsError::new(
                "every field already holds the value the change names, so nothing would change"
                    .to_string(),
            )
            .into(),
        );
    }
    changed.set_revision(Some(next_revision));
    // The document records the moderator who last wrote its moderator fields, and when.
    changed.set_moderated_at(Some(block_info.time_ms));
    changed.set_moderated_by(Some(moderator_id));

    // The changed document is judged as a replace judges one: the owner is the document's, the
    // times and heights are the ones it keeps, and the `countOf` and `sumOf` totals are read as
    // they will be once it is stored in place of the one read.
    let aggregates = read_property_constraint_aggregates_for_moderator_change(
        platform.drive,
        contract,
        document_type_name,
        &changed,
        &stored,
        block_info,
        execution_context,
        tx,
        platform_version,
    )?;
    let system = DocumentSystemValues {
        owner_id: Some(changed.owner_id()),
        created_at: changed.created_at(),
        updated_at: changed.updated_at(),
        transferred_at: changed.transferred_at(),
        created_at_block_height: changed.created_at_block_height(),
        updated_at_block_height: changed.updated_at_block_height(),
        transferred_at_block_height: changed.transferred_at_block_height(),
        created_at_core_block_height: changed.created_at_core_block_height(),
        updated_at_core_block_height: changed.updated_at_core_block_height(),
        transferred_at_core_block_height: changed.transferred_at_core_block_height(),
        aggregates: Some(aggregates),
    };
    // Each check runs only on a document the previous ones passed, as a replace's do: the later
    // two read values the schema check makes well formed.
    let properties = changed.properties();
    let result = contract.validate_document_properties(
        document_type_name,
        properties.into(),
        &system,
        platform_version,
    )?;
    if !result.is_valid() {
        return refuse_all(result.errors);
    }
    let result = document_type.validate_distinct_from_properties(
        properties,
        changed.owner_id(),
        platform_version,
    )?;
    if !result.is_valid() {
        return refuse_all(result.errors);
    }
    let result = document_type.validate_encrypted_property_shapes(properties, platform_version)?;
    if !result.is_valid() {
        return refuse_all(result.errors);
    }

    // Another document may already hold the new value of one of the type's unique indexes.
    if document_type.indexes().values().any(|index| index.unique) {
        let uniqueness = platform.drive.validate_moderated_document_uniqueness(
            contract,
            document_type,
            &changed,
            Some(&changed_fields),
            tx,
            platform_version,
        )?;
        if !uniqueness.is_valid() {
            return refuse_all(uniqueness.errors);
        }
    }

    let moderation_action =
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_change(
            transition,
            ContractDocumentChangeContext {
                data_contract_fetch_info: Arc::clone(contract_fetch_info),
                document: changed,
            },
        );
    let moderation_action = moderators.count_for_signer(
        moderation_action,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    Ok(ConsensusValidationResult::new_with_data(
        moderation_action.into(),
    ))
}

/// A document restore: the document type exists and says moderators may delete its
/// documents, the signer moderates the contract (see [`Moderators`]: a seated team must also
/// hold `deleteDocuments` on the type, which is what a restore undoes), the bytes decode
/// under the type, the type keeps removal records and the document has one that is not yet
/// restored, block time is within the restore window after the removal, the bytes hash to what
/// the record holds, the removal is no deletion the seated team approved together (those
/// stand), and no other document holds a value of one of the type's unique indexes. Every refusal is paid
/// for by bumping the signer's contract nonce.
///
/// The action carries the contract, the decoded document and the record marked restored, so
/// Drive puts the document back and marks the record without reading again. Nothing the
/// document type prices is charged, neither its creation token cost nor its `actionFees`
/// creation fee, and no fee agreement is asked: a moderator undoes a moderation, it does not
/// create content. The document comes back as it was, `$updatedAt` included, so a type's
/// deletion window (`moderatorAbilities.deleteWithin`) may have run out on it by then.
#[allow(clippy::too_many_arguments)]
fn transform_document_restore_v0<C: CoreRPCLike>(
    transition: &ContractUserModerationTransition,
    platform: &PlatformRef<C>,
    block_info: &BlockInfo,
    contract_fetch_info: &Arc<DataContractFetchInfo>,
    document_type_name: &str,
    document_bytes: &[u8],
    execution_context: &mut StateTransitionExecutionContext,
    tx: TransactionArg,
    platform_version: &PlatformVersion,
) -> Result<ConsensusValidationResult<StateTransitionAction>, Error> {
    let contract = &contract_fetch_info.contract;
    let contract_id = contract.id();
    let moderator_id = transition.owner_id();
    let bump_action = || {
        StateTransitionAction::BumpIdentityDataContractNonceAction(
            BumpIdentityDataContractNonceAction::from_borrowed_contract_user_moderation_transition(
                transition,
            ),
        )
    };
    let refuse = |error: ConsensusError| {
        Ok(ConsensusValidationResult::new_with_data_and_errors(
            bump_action(),
            vec![error],
        ))
    };

    let Some(document_type) = contract.document_type_optional_for_name(document_type_name) else {
        return refuse(
            InvalidDocumentTypeError::new(document_type_name.to_string(), contract_id).into(),
        );
    };
    // Only a type moderators delete from keeps records, so only such a type has anything to
    // restore.
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
    let epoch = &block_info.epoch;
    let moderators = Moderators::read(
        moderation,
        contract_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )?;
    if !moderators.may_moderate(
        owner_id,
        moderator_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(IdentityNotContractModeratorError::new(contract_id, moderator_id).into());
    }
    if moderators.lacks(ModerationAbility::DeleteDocuments, Some(document_type_name)) {
        return refuse(
            ContractModerationAbilityNotGrantedError::new(
                contract_id,
                ModerationAbility::DeleteDocuments,
                Some(document_type_name.to_string()),
            )
            .into(),
        );
    }

    // The bytes are the moderator's: whatever they fail to decode as is a refusal, never an
    // execution error, which would fail the block.
    let decoded =
        match Document::from_bytes_in_consensus(document_bytes, document_type, platform_version) {
            Ok(decoded) => decoded,
            Err(error) => ConsensusValidationResult::new_with_error(ConsensusError::BasicError(
                BasicError::ContractError(DataContractError::DecodingDocumentError(
                    DecodingError::new(format!(
                        "the document to restore does not decode under document type {}: {}",
                        document_type_name, error
                    )),
                )),
            )),
        };
    if !decoded.is_valid() {
        return Ok(ConsensusValidationResult::new_with_data_and_errors(
            bump_action(),
            decoded.errors,
        ));
    }
    let document = decoded.into_data()?;
    let document_id = document.id();

    // A deletion leaves the record a restore brings the document back from only on a type that
    // keeps them: on one that keeps none, there is no record, and no records tree to read.
    if !document_type.moderator_deletions_keep_records() {
        return refuse(
            ContractDocumentRemovalNotFoundError::new(
                contract_id,
                document_type_name.to_string(),
                document_id,
            )
            .into(),
        );
    }

    let (removal_fee, removal) = platform.drive.fetch_contract_document_removal_with_fee(
        contract_id,
        document_type_name,
        document_id,
        &block_info.epoch,
        tx,
        platform_version,
    )?;
    execution_context.add_operation(ValidationOperation::PrecalculatedOperation(removal_fee));
    let Some(removal) = removal else {
        return refuse(
            ContractDocumentRemovalNotFoundError::new(
                contract_id,
                document_type_name.to_string(),
                document_id,
            )
            .into(),
        );
    };
    // A restored record means the document is live: there is nothing to bring back.
    if let Some(restoration) = &removal.restoration {
        return refuse(
            ContractDocumentAlreadyRestoredError::new(
                contract_id,
                document_id,
                restoration.moderator_id,
                restoration.restored_at,
            )
            .into(),
        );
    }

    // The window bounds how far back a moderation can be undone: past it the removal stands.
    // At exactly the removal time plus the window the restore still passes.
    let window_ms = platform_version
        .system_limits
        .contract_document_restore_window_ms;
    if block_info.time_ms > removal.removed_at.saturating_add(window_ms) {
        return refuse(
            DocumentRestoreWindowElapsedError::new(
                contract_id,
                document_id,
                removal.removed_at,
                window_ms,
                block_info.time_ms,
            )
            .into(),
        );
    }

    // The record pins the content: only the document as it was comes back, not an edit of it.
    // The hash is billed, as the one the record was written with was.
    execution_context.add_operation(ValidationOperation::DoubleSha256(
        document_bytes.len() as u16 / SHA256_BLOCK_SIZE,
    ));
    let document_hash = hash_double(document_bytes);
    if document_hash != removal.document_hash {
        return refuse(
            DocumentRestoreHashMismatchError::new(
                contract_id,
                document_id,
                removal.document_hash,
                document_hash,
            )
            .into(),
        );
    }

    // A deletion the seated team made together stands: no single moderator undoes what the
    // leader and the members agreed on. It is the only removal made after the document
    // settled, a moderator alone deleting up to its settling time and the team only after it,
    // so the removal time tells the two apart with no read: the hash above pins the document
    // to the one deleted, its last modification included, and the window is fixed with the
    // type.
    if let (Some(_), Some(window_seconds)) = (
        document_type.moderator_settled_deletion(),
        document_type.documents_can_be_deleted_by_moderators_for(),
    ) {
        let (_, settled_at) = last_modified_and_settled_at(&document, window_seconds);
        if removal.removed_at > settled_at {
            return refuse(
                SettledDeletionNotRestorableError::new(
                    contract_id,
                    document_id,
                    removal.removed_at,
                )
                .into(),
            );
        }
    }

    // A document whose type declares a `ttl` and has expired stays deleted: the cleanup after
    // this block's state transitions would delete it again, and the record would say restored
    // for a document that no longer exists. Judged from its `$createdAt`, which the hash above
    // pins to the document as it was.
    if let Some(error) = validate_document_not_expired(
        contract_id,
        document_type,
        document_id,
        document.created_at(),
        block_info,
    )?
    .errors
    .into_iter()
    .next()
    {
        return refuse(error);
    }

    // What the hash does not pin: another document may have taken a value of one of the
    // type's unique indexes while the document was gone, and would clash with it.
    if document_type.indexes().values().any(|index| index.unique) {
        let uniqueness = platform.drive.validate_moderated_document_uniqueness(
            contract,
            document_type,
            &document,
            None,
            tx,
            platform_version,
        )?;
        if !uniqueness.is_valid() {
            return Ok(ConsensusValidationResult::new_with_data_and_errors(
                bump_action(),
                uniqueness.errors,
            ));
        }
    }

    let removal = ContractDocumentRemoval {
        restoration: Some(ContractDocumentRestoration {
            moderator_id,
            restored_at: block_info.time_ms,
        }),
        ..removal
    };
    Ok(ConsensusValidationResult::new_with_data(
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_restoration(
            transition,
            ContractDocumentRestorationContext {
                data_contract_fetch_info: Arc::clone(contract_fetch_info),
                document,
                removal,
            },
        )
        .into(),
    ))
}

/// The ability a seated team needs to edit `list`, putting an identity on it or taking one off.
fn ability_of(list: ContractModerationList) -> ModerationAbility {
    match list {
        ContractModerationList::Banlist => ModerationAbility::Ban,
        ContractModerationList::Suspensions => ModerationAbility::Suspend,
        ContractModerationList::Warnings => ModerationAbility::Warn,
    }
}

/// The list the action edits, `None` for an action on a document, which edits none.
fn list_of(action: &ContractUserModerationAction) -> Option<ContractModerationList> {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Unban { .. } => {
            Some(ContractModerationList::Banlist)
        }
        ContractUserModerationAction::Suspend { .. }
        | ContractUserModerationAction::Unsuspend { .. } => {
            Some(ContractModerationList::Suspensions)
        }
        ContractUserModerationAction::Warn { .. }
        | ContractUserModerationAction::ClearWarnings { .. } => {
            Some(ContractModerationList::Warnings)
        }
        ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. }
        | ContractUserModerationAction::ChangeDocumentFields { .. }
        | ContractUserModerationAction::DeleteSettledDocument { .. }
        | ContractUserModerationAction::ApproveTeamAction { .. } => None,
    }
}

/// The lists the action needs to know about: its own, and for a ban or a suspend the other
/// barring one as well, because a ban removes a suspension and a suspend is refused for a
/// banned identity. Warnings bar nothing and are left alone by a ban, so the warning list is
/// read by a warn and a clearing alone. Only lists the contract keeps are read.
fn lists_to_read(
    moderation: &ContractModerationConfig,
    action: &ContractUserModerationAction,
    list: ContractModerationList,
) -> Vec<ContractModerationList> {
    match action {
        ContractUserModerationAction::Ban { .. } | ContractUserModerationAction::Suspend { .. } => {
            moderation.barring_lists().collect()
        }
        ContractUserModerationAction::Unban { .. }
        | ContractUserModerationAction::Unsuspend { .. }
        | ContractUserModerationAction::Warn { .. }
        | ContractUserModerationAction::ClearWarnings { .. }
        | ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. }
        | ContractUserModerationAction::ChangeDocumentFields { .. }
        | ContractUserModerationAction::DeleteSettledDocument { .. }
        | ContractUserModerationAction::ApproveTeamAction { .. } => vec![list],
    }
}

/// The first rule the target's status breaks for this action, if any.
fn refusal_for_status(
    action: &ContractUserModerationAction,
    target_id: Identifier,
    status: &ContractModerationStatus,
    contract_id: Identifier,
    block_info: &BlockInfo,
    platform_version: &PlatformVersion,
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
        // A warning is refused only once the entry is full: a banned or suspended identity
        // may be warned, since the warning outlives the ban and says why it came to that.
        ContractUserModerationAction::Warn { .. } => {
            let max_warnings = platform_version
                .system_limits
                .max_contract_warnings_per_identity;
            (status.warnings.len() >= usize::from(max_warnings)).then(|| {
                ContractUserWarningLimitReachedError::new(contract_id, target_id, max_warnings)
                    .into()
            })
        }
        ContractUserModerationAction::ClearWarnings { .. } => (!status.warned())
            .then(|| ContractUserNotWarnedError::new(contract_id, target_id).into()),
        // A document deletion, restore, field change or team action reads no list.
        ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. }
        | ContractUserModerationAction::ChangeDocumentFields { .. }
        | ContractUserModerationAction::DeleteSettledDocument { .. }
        | ContractUserModerationAction::ApproveTeamAction { .. } => None,
    }
}
