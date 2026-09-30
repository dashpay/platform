use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::moderators::Moderators;
use crate::execution::validation::state_transition::common::validate_document_not_expired::validate_document_not_expired;
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::state_transitions::batch::{
    fetch_document_with_id, read_property_constraint_aggregates_for_moderator_change,
};
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::basic::contract_moderation::InvalidContractModerationDocumentFieldsError;
use dpp::consensus::basic::decode::DecodingError;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::overflow_error::OverflowError;
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::contract_moderation::{
    ContractDocumentAlreadyRestoredError, ContractDocumentRemovalNotFoundError,
    ContractModerationAbilityNotGrantedError, ContractModerationNotEnabledError,
    ContractModerationTargetNotAllowedError, ContractModerationTargetNotFoundError,
    ContractSuspensionNotInFutureError, ContractUserAlreadyBannedError, ContractUserBannedError,
    ContractUserNotBannedError, ContractUserNotSuspendedError, ContractUserNotWarnedError,
    ContractUserWarningLimitReachedError, DocumentFieldNotChangeableByModeratorsError,
    DocumentModerationWindowElapsedError, DocumentRestoreHashMismatchError,
    DocumentRestoreWindowElapsedError, DocumentTypeNotDeletableByModeratorsError,
    IdentityNotContractModeratorError,
};
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationConfig,
    ContractModerationList, ContractModerationStatus, ModerationAbility,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::methods::{DocumentTypeBasicMethods, DocumentTypeV0Methods};
use dpp::data_contract::document_type::property_constraints::DocumentSystemValues;
use dpp::data_contract::errors::DataContractError;
use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransitionOwned;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentChangeContext, ContractDocumentDeletionContext,
    ContractDocumentRemovalRecordContext, ContractDocumentRestorationContext,
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
    // the owner demotes a moderator by a contract update before deleting what it wrote, and the
    // leader of a seated team removes a member before anyone deletes what it wrote.
    let document_owner_id = document.owner_id();
    if moderators.protects(
        owner_id,
        document_owner_id,
        platform.drive,
        epoch,
        execution_context,
        tx,
        platform_version,
    )? {
        return refuse(
            ContractModerationTargetNotAllowedError::new(contract_id, document_owner_id).into(),
        );
    }

    // A document type may give its moderators a window: so many seconds after a document's
    // last modification, past which the document is settled and no moderator deletes it (its
    // own owner's deletion is `canBeDeleted`'s business, at any age). The last modification is
    // `$updatedAt`, which a replace moves, opening the window again since what it wrote is new
    // content; a type whose documents never change may carry `$createdAt` alone, and that is
    // then the clock. The type requires one of the two, so every document carries it; one that
    // carried neither would read as modified at time zero, which is settled: the refusal that
    // protects the author.
    if let Some(window_seconds) = document_type.documents_can_be_deleted_by_moderators_for() {
        let last_modified_at = document
            .updated_at()
            .or(document.created_at())
            .unwrap_or_default();
        let settled_at = last_modified_at
            .saturating_add(u64::from(window_seconds).saturating_mul(MILLIS_PER_SECOND));
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
        let replaces_restored_record = match existing_removal {
            None => false,
            Some(removal) if removal.is_restored() => true,
            Some(_) => {
                return Err(Error::Execution(ExecutionError::DriveIncoherence(
                    "a document with an unrestored moderation removal record exists",
                )))
            }
        };
        Some(ContractDocumentRemovalRecordContext {
            removed_at: block_info.time_ms,
            document_hash,
            replaces_restored_record,
        })
    } else {
        None
    };

    let moderation_action =
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_deletion(
            transition,
            ContractDocumentDeletionContext {
                data_contract_fetch_info: Arc::clone(contract_fetch_info),
                document_owner_id,
                record,
                refunds_owner: document_type.moderator_deletions_refund_owner(),
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
/// restored, block time is
/// within the restore window after the removal, the bytes hash to what the record holds, and
/// no other document holds a value of one of the type's unique indexes. Every refusal is paid
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
        | ContractUserModerationAction::ChangeDocumentFields { .. } => None,
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
        | ContractUserModerationAction::ChangeDocumentFields { .. } => vec![list],
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
        // A document deletion, restore or field change reads no list.
        ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. }
        | ContractUserModerationAction::ChangeDocumentFields { .. } => None,
    }
}
