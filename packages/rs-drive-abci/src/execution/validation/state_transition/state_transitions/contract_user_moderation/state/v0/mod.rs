use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::types::execution_operation::{ValidationOperation, SHA256_BLOCK_SIZE};
use crate::execution::types::state_transition_execution_context::{
    StateTransitionExecutionContext, StateTransitionExecutionContextMethodsV0,
};
use crate::execution::validation::state_transition::common::seated_moderation_charter::{
    fetch_seated_moderation_charter, SeatedModerationCharter,
};
use crate::execution::validation::state_transition::common::validate_identity_exists::validate_identity_exists;
use crate::execution::validation::state_transition::state_transitions::batch::fetch_document_with_id;
use crate::platform_types::platform::PlatformRef;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::consensus::basic::decode::DecodingError;
use dpp::consensus::basic::document::{DataContractNotPresentError, InvalidDocumentTypeError};
use dpp::consensus::basic::BasicError;
use dpp::consensus::state::contract_moderation::{
    ContractDocumentAlreadyRestoredError, ContractDocumentRemovalNotFoundError,
    ContractModerationAbilityNotGrantedError, ContractModerationNotEnabledError,
    ContractModerationTargetNotAllowedError, ContractModerationTargetNotFoundError,
    ContractSuspensionNotInFutureError, ContractUserAlreadyBannedError, ContractUserBannedError,
    ContractUserNotBannedError, ContractUserNotSuspendedError, ContractUserNotWarnedError,
    ContractUserWarningLimitReachedError, DocumentModerationWindowElapsedError,
    DocumentRestoreHashMismatchError, DocumentRestoreWindowElapsedError,
    DocumentTypeNotDeletableByModeratorsError, IdentityNotContractModeratorError,
    ModerationReasonNotListedError,
};
use dpp::consensus::state::document::document_not_found_error::DocumentNotFoundError;
use dpp::consensus::state::state_error::StateError;
use dpp::consensus::ConsensusError;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractDocumentRestoration, ContractModerationConfig,
    ContractModerationList, ContractModerationStatus, ElectedModerators, ModerationAbility,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::errors::DataContractError;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::prelude::{ConsensusValidationResult, Identifier};
use dpp::state_transition::contract_user_moderation_transition::accessors::ContractUserModerationTransitionAccessorsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::StateTransitionOwned;
use dpp::util::hash::hash_double;
use dpp::version::PlatformVersion;
use drive::drive::contract::DataContractFetchInfo;
use drive::drive::Drive;
use drive::grovedb::TransactionArg;
use drive::state_transition_action::contract::contract_user_moderation::v0::{
    ContractDocumentDeletionContext, ContractDocumentRestorationContext,
};
use drive::state_transition_action::contract::contract_user_moderation::ContractUserModerationTransitionAction;
use drive::state_transition_action::system::bump_identity_data_contract_nonce_action::BumpIdentityDataContractNonceAction;
use drive::state_transition_action::StateTransitionAction;
use std::sync::Arc;

/// `canBeDeletedByModeratorsFor` is declared in seconds, as the other durations of a document
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
    /// A document deletion is checked by `transform_document_deletion_v0` and a document
    /// restore by `transform_document_restore_v0`. For the rest:
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
/// and writes its record without reading again. Nothing the document type prices is charged, neither
/// its deletion token cost nor its `actionFees` deletion fee: both are what a document's own
/// owner pays for deleting it, and a moderator removes content on the contract's behalf.
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

    // What the record commits to: the document as serialized under its type, which a restore
    // must bring back byte for byte. It is serialized here, from the document as read, rather
    // than hashed as stored: a document stored under an earlier version of its type or of the
    // serialization would never re-serialize to its stored bytes, and a client keeping the
    // document (not the bytes) could never match them.
    let serialized = document.serialize(document_type, contract, platform_version)?;
    execution_context.add_operation(ValidationOperation::DoubleSha256(
        serialized.len() as u16 / SHA256_BLOCK_SIZE,
    ));
    let document_hash = hash_double(serialized);

    // A document id is produced at most once (it commits to the nonce of its create
    // transition), so the only record this id can already have is of a deletion a moderator
    // restored: the document is live again and this deletion writes a fresh record in its
    // place. An unrestored record beside a live document is a state no transition produces.
    let (removal_fee, existing_removal) = platform.drive.fetch_contract_document_removal_with_fee(
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

    let moderation_action =
        ContractUserModerationTransitionAction::from_borrowed_transition_with_document_deletion(
            transition,
            ContractDocumentDeletionContext {
                data_contract_fetch_info: Arc::clone(contract_fetch_info),
                document_owner_id,
                removed_at: block_info.time_ms,
                document_hash,
                replaces_restored_record,
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
/// under the type, the document has a removal record that is not yet restored, block time is
/// within the restore window after the removal, the bytes hash to what the record holds, and
/// no other document holds a value of one of the type's unique indexes. Every refusal is paid
/// for by bumping the signer's contract nonce.
///
/// The action carries the contract, the decoded document and the record marked restored, so
/// Drive puts the document back and marks the record without reading again. Nothing the
/// document type prices is charged, neither its creation token cost nor its `actionFees`
/// creation fee, and no fee agreement is asked: a moderator undoes a moderation, it does not
/// create content. The document comes back as it was, `$updatedAt` included, so a type's
/// deletion window (`canBeDeletedByModeratorsFor`) may have run out on it by then.
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

    // What the hash does not pin: another document may have taken a value of one of the
    // type's unique indexes while the document was gone, and would clash with it.
    if document_type.indexes().values().any(|index| index.unique) {
        let uniqueness = platform.drive.validate_restored_document_uniqueness(
            contract,
            document_type,
            &document,
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

/// Who moderates a contract, as state has it now.
///
/// Until an elected contract has a seated charter, the moderators its declaration names do: the
/// merged kinds, or the elected declaration's interim ([`ContractModerationConfig::may_moderate`]
/// and [`ContractModerationConfig::protects`]). Once a contest for its seat was awarded, the
/// team of the seated charter does, and only it (decentralized moderation teams): the
/// leader and the active members moderate, with the abilities the declaration gives the team
/// and no others, and they are protected, with the owner when the declaration says so. Interim
/// moderators are then neither.
enum Moderators<'a> {
    /// The moderators the declaration names
    Declared(&'a ContractModerationConfig),
    /// The team of the charter seated on an elected contract
    Seated {
        elected: &'a ElectedModerators,
        charter: SeatedModerationCharter,
    },
}

impl<'a> Moderators<'a> {
    /// Who moderates a contract declaring `moderation`: for an elected declaration, whether a
    /// charter is seated is read (billed); nothing is read for the merged kinds.
    #[allow(clippy::too_many_arguments)]
    fn read(
        moderation: &'a ContractModerationConfig,
        contract_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Self, Error> {
        let Some(elected) = moderation.moderators.elected() else {
            return Ok(Moderators::Declared(moderation));
        };
        Ok(
            match fetch_seated_moderation_charter(
                drive,
                contract_id,
                epoch,
                execution_context,
                tx,
                platform_version,
            )? {
                None => Moderators::Declared(moderation),
                Some(charter) => Moderators::Seated { elected, charter },
            },
        )
    }

    /// Whether `identity_id` may moderate the contract owned by `owner_id`. For a seated team,
    /// whether it is the leader or an active member, at most two point reads, billed.
    #[allow(clippy::too_many_arguments)]
    fn may_moderate(
        &self,
        owner_id: Identifier,
        identity_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match self {
            Moderators::Declared(moderation) => {
                Ok(moderation.may_moderate(&owner_id, &identity_id))
            }
            Moderators::Seated { charter, .. } => charter.seats(
                drive,
                identity_id,
                epoch,
                execution_context,
                tx,
                platform_version,
            ),
        }
    }

    /// Whether `identity_id` is protected from moderation on the contract owned by `owner_id`:
    /// it can not be put on a list, and its documents can not be deleted. For a seated team,
    /// the owner when the declaration protects it, and the leader and the active members.
    #[allow(clippy::too_many_arguments)]
    fn protects(
        &self,
        owner_id: Identifier,
        identity_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        match self {
            Moderators::Declared(moderation) => Ok(moderation.protects(&owner_id, &identity_id)),
            Moderators::Seated { elected, charter } => {
                if identity_id == owner_id && elected.owner_protected {
                    return Ok(true);
                }
                charter.seats(
                    drive,
                    identity_id,
                    epoch,
                    execution_context,
                    tx,
                    platform_version,
                )
            }
        }
    }

    /// The refusal of a seated team's ban, suspension, warning or document deletion whose
    /// reason names no reason document its proposal lists (decentralized moderation teams): a
    /// team acts only on the grounds it proposed, and a proposal that lists none can take no
    /// such action. The proposal is read, billed, only when the reason names a document. A
    /// reversal carries no reason and is not checked, and neither are the moderators a
    /// declaration names, the interim among them, whose reason document is stored as written.
    #[allow(clippy::too_many_arguments)]
    fn unlisted_reason(
        &self,
        action: &ContractUserModerationAction,
        contract_id: Identifier,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ConsensusError>, Error> {
        let Moderators::Seated { charter, .. } = self else {
            return Ok(None);
        };
        let reason = match action {
            ContractUserModerationAction::Ban { reason, .. }
            | ContractUserModerationAction::Suspend { reason, .. }
            | ContractUserModerationAction::Warn { reason, .. }
            | ContractUserModerationAction::DeleteDocument { reason, .. } => reason,
            ContractUserModerationAction::Unban { .. }
            | ContractUserModerationAction::Unsuspend { .. }
            | ContractUserModerationAction::ClearWarnings { .. }
            | ContractUserModerationAction::RestoreDocument { .. } => return Ok(None),
        };
        let listed = match reason.reason_document_id {
            None => false,
            Some(reason_document_id) => charter
                .fetch_proposal(drive, epoch, execution_context, tx, platform_version)?
                .reasons
                .contains(&reason_document_id),
        };
        Ok((!listed).then(|| {
            ModerationReasonNotListedError::new(
                contract_id,
                charter.charter.submitted_charter_id,
                reason.reason_document_id,
            )
            .into()
        }))
    }

    /// `action`, counted for its signer when a member of a seated team signs a ban, a
    /// suspension, a warning or a document deletion: the signer's moderation action count since
    /// the moderators pot was last settled is read (one point read, billed) and the action
    /// carries it one higher, for Drive to write. What a settle splits the pot's action share
    /// by. A reversal (an unban, an unsuspension, a clearing, a restore) counts for nothing,
    /// and neither does an action of the moderators a declaration names, who share the pot
    /// equally.
    #[allow(clippy::too_many_arguments)]
    fn count_for_signer(
        &self,
        action: ContractUserModerationTransitionAction,
        drive: &Drive,
        epoch: &Epoch,
        execution_context: &mut StateTransitionExecutionContext,
        tx: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContractUserModerationTransitionAction, Error> {
        let Moderators::Seated { .. } = self else {
            return Ok(action);
        };
        let counts = matches!(
            action.action(),
            ContractUserModerationAction::Ban { .. }
                | ContractUserModerationAction::Suspend { .. }
                | ContractUserModerationAction::Warn { .. }
                | ContractUserModerationAction::DeleteDocument { .. }
        );
        if !counts {
            return Ok(action);
        }
        let (fee, count) = drive.fetch_contract_moderation_action_count_with_fee(
            action.data_contract_id(),
            action.moderator_id(),
            epoch,
            tx,
            platform_version,
        )?;
        execution_context.add_operation(ValidationOperation::PrecalculatedOperation(fee));
        // A contract stored elected before the counts existed has nowhere to count: its team's
        // actions go uncounted, and a settle splits the action share equally.
        Ok(match count {
            Some(count) => action.with_moderation_action_count(count.saturating_add(1)),
            None => action,
        })
    }

    /// Whether a seated team lacks `ability`: on `document_type_name` for a deletion or a
    /// restore, on every moderated type for a list, which is contract-wide. The moderators a
    /// declaration names hold every ability the contract backs.
    fn lacks(&self, ability: ModerationAbility, document_type_name: Option<&str>) -> bool {
        match self {
            Moderators::Declared(_) => false,
            Moderators::Seated { elected, .. } => match document_type_name {
                Some(document_type_name) => !elected.allows(document_type_name, ability),
                None => !elected.allows_on_any_type(ability),
            },
        }
    }
}

/// The ability a seated team needs to edit `list`, putting an identity on it or taking one off.
fn ability_of(list: ContractModerationList) -> ModerationAbility {
    match list {
        ContractModerationList::Banlist => ModerationAbility::Ban,
        ContractModerationList::Suspensions => ModerationAbility::Suspend,
        ContractModerationList::Warnings => ModerationAbility::Warn,
    }
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
        ContractUserModerationAction::Warn { .. }
        | ContractUserModerationAction::ClearWarnings { .. } => {
            Some(ContractModerationList::Warnings)
        }
        ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. } => None,
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
        | ContractUserModerationAction::RestoreDocument { .. } => vec![list],
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
        // A document deletion reads no list.
        ContractUserModerationAction::DeleteDocument { .. }
        | ContractUserModerationAction::RestoreDocument { .. } => None,
    }
}
