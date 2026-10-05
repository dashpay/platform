//! Ban, unban, suspend, unsuspend, warn and clear the warnings of identities on a moderated
//! data contract, and delete documents of the document types that let moderators do so
//! (protocol version 14), alone or, once settled, as a seated team.
//!
//! A contract whose config declares moderation keeps a banlist, a suspension list and/or a
//! warning list. The contract owner, or a moderator the config names, edits them with a
//! [`ContractUserModerationTransition`] signed by a CRITICAL authentication key. A banned or
//! suspended identity cannot act on the contract at the document level; a warned one can, the
//! warnings being a record it and everyone else can read.
//!
//! The same transition deletes one document of a document type that sets
//! `moderatorAbilities.delete`, whoever owns it, and leaves a record of the deletion under the
//! contract; it restores such a document, as it was, within a week of its deletion; and it
//! sets or removes the fields a document type keeps for its moderators
//! (`moderatorAbilities.changeFields`) on one of its documents, whoever owns it.
//!
//! Past a type's `moderatorAbilities.deleteWithin` window a document is settled: no moderator
//! deletes it alone. A type that says who of an elected contract's seated team must approve
//! (`moderatorAbilities.deleteSettled`) lets the team's members delete it together, as a token
//! group acts: one member proposes the deletion, which is its own approval and a team action
//! kept under the contract by its id, and the others approve that action by its id until as
//! many as the rule asks for have, the leader among them when the rule says so. The approval
//! that meets the rule closes the action and deletes the document.
//!
//! ```ignore
//! let reason = ContractModerationReason::from_text("spam");
//! let status = moderator_identity
//!     .ban_contract_user(&sdk, contract_id, user_id, reason.clone(), None, &signer, None)
//!     .await?;
//! let removal = moderator_identity
//!     .delete_contract_document(
//!         &sdk, contract_id, "post".to_string(), document_id, reason.clone(), None, &signer,
//!         None,
//!     )
//!     .await?;
//! let (action_id, status) = team_member_identity
//!     .delete_settled_contract_document(
//!         &sdk, contract_id, "post".to_string(), settled_document_id, reason, None, &signer,
//!         None,
//!     )
//!     .await?;
//! // Every other member that approves names the action the proposal opened.
//! let status = other_team_member_identity
//!     .approve_contract_team_action(&sdk, contract_id, action_id, None, &other_signer, None)
//!     .await?;
//! if status == GroupActionStatus::ActionClosed {
//!     // The action ran, by this approval or a later one: the document is gone.
//! }
//! ```

use crate::platform::Fetch;
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractModerationListStatuses, ContractModerationReason,
};
use dpp::data_contract::DataContract;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::Document;
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel, TimestampMillis};
use dpp::platform_value::{Identifier, Value};
use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::proof_result::StateTransitionProofResult;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};

use super::waitable::Waitable;

/// The target identity's status on the lists a moderation touched, as the proof of the
/// moderation shows it. A ban proves every barring list the contract keeps (it removes a
/// suspension too); an unban, a suspend, an unsuspend, a warn and a clearing prove the one
/// list they edit and say nothing about the others, so an identity shown as no longer
/// suspended may still be banned. Fetch `ContractModerationListStatuses` over every list the
/// contract keeps for the whole picture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeratedUserStatus {
    /// The moderated contract
    pub contract_id: Identifier,
    /// The moderated identity
    pub identity_id: Identifier,
    /// Its status on the lists proved, after the moderation
    pub status: ContractModerationListStatuses,
}

impl TryFrom<StateTransitionProofResult> for ModeratedUserStatus {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractModerationListStatuses(
                contract_id,
                identity_id,
                status,
            ) => Ok(Self {
                contract_id,
                identity_id,
                status,
            }),
            other => Err(Error::Generic(format!(
                "expected a contract moderation status proof result, got {other}"
            ))),
        }
    }
}

/// The document a field change left, as its proof shows it: the document as it now stands.
struct VerifiedChangedDocument(Document);

impl TryFrom<StateTransitionProofResult> for VerifiedChangedDocument {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedDocuments(mut documents) => {
                match (documents.pop_first(), documents.is_empty()) {
                    (Some((_, Some(document))), true) => Ok(Self(document)),
                    _ => Err(Error::Generic(
                        "expected the proof of a document field change to show the one \
                         document it changed"
                            .to_string(),
                    )),
                }
            }
            other => Err(Error::Generic(format!(
                "expected a document proof result, got {other}"
            ))),
        }
    }
}

/// The record a document deletion left under the contract, as its proof shows it. The proof
/// binds the record to the transition that wrote it, so only the record itself is kept here:
/// the contract, the document type and the document id are the caller's own arguments.
struct VerifiedDocumentRemoval(ContractDocumentRemoval);

impl TryFrom<StateTransitionProofResult> for VerifiedDocumentRemoval {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractDocumentRemoval(_, _, _, removal) => {
                Ok(Self(removal))
            }
            other => Err(Error::Generic(format!(
                "expected a contract document removal proof result, got {other}"
            ))),
        }
    }
}

/// A team action as the proof of one member's proposal or approval shows it: its id and
/// whether it is still active or closed, its approvals having met the rule and the action run.
/// The proof binds both to the transition: the verifier rebuilds the action's id from it (the
/// proposal's computed id, or the id the approval carries) and finds the signer's approval
/// under that action.
struct VerifiedTeamActionSignature(Identifier, GroupActionStatus);

impl TryFrom<StateTransitionProofResult> for VerifiedTeamActionSignature {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractTeamActionSignature(
                _,
                action_id,
                status,
            ) => Ok(Self(action_id, status)),
            other => Err(Error::Generic(format!(
                "expected a contract team action signature proof result, got {other}"
            ))),
        }
    }
}

/// What a document deletion left, as its proof shows it: the removal record, or nothing on a
/// type whose moderators' deletions keep no record, whose proof shows the document gone.
struct VerifiedDocumentDeletion(Option<ContractDocumentRemoval>);

impl TryFrom<StateTransitionProofResult> for VerifiedDocumentDeletion {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractDocumentRemoval(_, _, _, removal) => {
                Ok(Self(Some(removal)))
            }
            StateTransitionProofResult::VerifiedDocuments(mut documents) => {
                match (documents.pop_first(), documents.is_empty()) {
                    (Some((_, None)), true) => Ok(Self(None)),
                    _ => Err(Error::Generic(
                        "expected the proof of a document deletion to show the document gone"
                            .to_string(),
                    )),
                }
            }
            other => Err(Error::Generic(format!(
                "expected a contract document removal proof result, got {other}"
            ))),
        }
    }
}

#[async_trait::async_trait]
pub trait ModerateContractUser: Waitable {
    /// Sends one moderation `action` for `contract_id`, signed by this identity, and resolves
    /// once the target's status is proved. If `signing_key_to_use` is not set, the first
    /// CRITICAL authentication key without contract bounds that the signer can sign with is
    /// used.
    ///
    /// The proof shows the target's entry on the list, not this exact transition (the nonce is
    /// not stored), so it is waited for as affected state.
    async fn moderate_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        action: ContractUserModerationAction,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error>;

    /// Puts `identity_id` on the banlist of `contract_id` for `reason`, which is stored with
    /// the entry: a text of at most `SystemLimits::max_contract_moderation_reason_length`
    /// bytes, and a code nothing checks, reserved for ban codes contracts may declare later.
    #[allow(clippy::too_many_arguments)]
    async fn ban_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Ban {
                identity_id,
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Takes `identity_id` off the banlist of `contract_id`.
    async fn unban_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Unban { identity_id },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Suspends `identity_id` on `contract_id` until the block time `until`, in milliseconds,
    /// for `reason` (as for a ban), replacing a suspension it already carries.
    #[allow(clippy::too_many_arguments)]
    async fn suspend_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Suspend {
                identity_id,
                until,
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Takes `identity_id` off the suspension list of `contract_id`, lapsed or not.
    async fn unsuspend_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Unsuspend { identity_id },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Adds a warning for `reason` (as for a ban) to the entry of `identity_id` on the
    /// warning list of `contract_id`, stamped with the block time. Warnings bar nothing and
    /// accumulate, at most `SystemLimits::max_contract_warnings_per_identity` at a time:
    /// past that, the warn is refused until they are cleared.
    #[allow(clippy::too_many_arguments)]
    async fn warn_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Warn {
                identity_id,
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Takes `identity_id` off the warning list of `contract_id`: every warning it carries
    /// goes.
    async fn clear_contract_user_warnings<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::ClearWarnings { identity_id },
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    /// Deletes document `document_id` of `document_type_name` on `contract_id`, whoever owns
    /// it, for `reason` (as for a ban). The document type must set
    /// `moderatorAbilities.delete`. Resolves with the record the deletion left under the
    /// contract: whose the document was, who removed it, why and when; or with `None` on a
    /// type whose moderators' deletions keep no record (`moderatorAbilities.deleteKeepsRecord:
    /// false`), whose proof shows the document gone.
    ///
    /// The document's owner gets no storage refund unless the type gives it back
    /// (`moderatorAbilities.deleteRefundsOwner`), and nothing ever deletes the record. The
    /// record holds a hash of the document as it was: keep the document (or its bytes) if the
    /// deletion may have to be undone, since `restore_contract_document` needs it. Which proof
    /// to expect is read from the contract, which is registered with the SDK's context provider
    /// when it holds none.
    #[allow(clippy::too_many_arguments)]
    async fn delete_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<Option<ContractDocumentRemoval>, Error>;

    /// Proposes, as a member of the seated moderation team of `contract_id`, the deletion of
    /// settled document `document_id` of `document_type_name`, for `reason` (as for a ban): one
    /// last modified longer ago than the type's `moderatorAbilities.deleteWithin` window, within
    /// which a moderator deletes it alone. The type must say who of the team must approve
    /// (`moderatorAbilities.deleteSettled`), and `reason` must name a reason document the
    /// team's proposal lists. A member the leader added proposes only the deletion of documents
    /// created after its addition, unless the type's rule sets `approversPredateDocument: false`
    /// (`ContractTeamMemberAddedAfterDocumentError`). The proposal is this member's approval,
    /// kept under the contract as a team action with the document's last modification and
    /// revision and `reason`.
    ///
    /// Resolves with the action's id and its status. The id is what the other members approve
    /// with [`approve_contract_team_action`](Self::approve_contract_team_action), and what
    /// `ContractTeamActions` and `ContractTeamActionSigners` read it back by: it is computed
    /// from the contract, this identity, its contract nonce, the document and `reason`, so every
    /// proposal has its own. The status is where the proof finds this proposal's approval:
    /// `ActionActive` while the approvals fall short of the rule, and `ActionClosed` once the
    /// action ran, whether this proposal alone met the rule (a rule of one approval, given by
    /// the leader when the rule names the leader) or a later approval did before the proof was
    /// taken: the document is then gone, deleted as `delete_contract_document` deletes it.
    ///
    /// A proposal never lapses, but an approval is refused once the document changed since the
    /// proposal (a replace, a transfer, a moderator's change of its fields). The proof is
    /// verified without the contract, and fails once this identity left the team and a later
    /// approval dropped its approval.
    #[allow(clippy::too_many_arguments)]
    async fn delete_settled_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<(Identifier, GroupActionStatus), Error>;

    /// Approves, as a member of the seated moderation team of `contract_id`, team action
    /// `action_id` another member proposed: today the deletion of a settled document, whose
    /// proposal ([`delete_settled_contract_document`](Self::delete_settled_contract_document))
    /// returned the id. What the action does and why are the proposal's.
    ///
    /// Resolves with the action's status where the proof finds this approval: `ActionActive`
    /// while the approvals still fall short of the rule, and `ActionClosed` once the action ran,
    /// the leader among the approvals when the rule says so, whether this approval met the rule
    /// or a later one did before the proof was taken: the document is gone. The approvals of
    /// members who left the team since no longer count and are dropped, as are those of members
    /// the leader took off and added again after the document was created when the rule admits
    /// only members from before it, and a dropped approval no longer proves: the proof of this
    /// one fails once this identity left the team and a later approval dropped it. An approval
    /// of an action that does not exist (`ContractTeamActionDoesNotExistError`), one this
    /// member already approved (`ContractTeamActionAlreadySignedError`), one already closed
    /// (`ContractTeamActionAlreadyCompletedError`), one whose document changed since the
    /// proposal (`ContractTeamActionDocumentChangedError`), or one by a member the leader added
    /// no earlier than the document was created, under a rule admitting only members from
    /// before it (`ContractTeamMemberAddedAfterDocumentError`), is refused. The proof is
    /// verified without the contract.
    async fn approve_contract_team_action<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        action_id: Identifier,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<GroupActionStatus, Error>;

    /// Brings back `document`, of `document_type_name` on `contract`, that a moderator deleted:
    /// the document as it was when it was deleted, which must hash to what its removal record
    /// holds, within `SystemLimits::contract_document_restore_window_ms` (a week) of the
    /// deletion. Any current moderator or the contract owner may restore, whoever deleted,
    /// except a deletion a seated team approved together past the type's window
    /// (`moderatorAbilities.deleteSettled`): that one stands, and its restore is refused
    /// (`SettledDeletionNotRestorableError`, 41209). The document goes back through an ordinary
    /// insert, so a unique index value another document took meanwhile refuses it. Resolves
    /// with the record, now marked restored.
    ///
    /// The signer pays for the document's storage; the refund of a later deletion stays its
    /// owner's. `contract` serializes the document and is what the proof of the restore is
    /// verified against, so it is registered with the SDK's context provider.
    #[allow(clippy::too_many_arguments)]
    async fn restore_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract: &DataContract,
        document_type_name: String,
        document: &Document,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ContractDocumentRemoval, Error>;

    /// Sets the fields `fields` names on document `document_id` of `document_type_name` on
    /// `contract_id`, whoever owns it, for `reason` (as for a ban): each a field the document
    /// type keeps for its moderators (`moderatorAbilities.changeFields`), a `null` value
    /// removing it. The document as changed must still be one of its type (its schema, its
    /// `propertyConstraints`, its unique indexes). Resolves with the document as it now
    /// stands: every other property as its owner wrote it, `$updatedAt` among them, and
    /// `$revision` one higher, so a replace its owner built on the earlier revision is refused.
    ///
    /// The signer pays for the bytes the change adds; a refund of the document's storage stays
    /// its owner's. The document is read back under the contract, which is registered with the
    /// SDK's context provider when it holds none.
    #[allow(clippy::too_many_arguments)]
    async fn change_contract_document_fields<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        fields: BTreeMap<String, Value>,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error>;
}

#[async_trait::async_trait]
impl ModerateContractUser for Identity {
    async fn moderate_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        action: ContractUserModerationAction,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        // A document deletion or restore is proved by its removal record, not by a status,
        // and a team action's proposal or approval by the signer's approval of the action.
        // Refused before the nonce is taken: sent from here it would execute, be paid for, and
        // then fail to read its own result.
        if matches!(
            action,
            ContractUserModerationAction::DeleteDocument { .. }
                | ContractUserModerationAction::RestoreDocument { .. }
                | ContractUserModerationAction::ChangeDocumentFields { .. }
                | ContractUserModerationAction::DeleteSettledDocument { .. }
                | ContractUserModerationAction::ApproveTeamAction { .. }
        ) {
            return Err(Error::Generic(
                "a document deletion, restore, field change or team action names no identity \
                 to report a status of: send it with `delete_contract_document`, \
                 `restore_contract_document`, `change_contract_document_fields`, \
                 `delete_settled_contract_document` or `approve_contract_team_action`, which \
                 return what it left"
                    .to_string(),
            ));
        }
        broadcast_moderation(
            self,
            sdk,
            contract_id,
            action,
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    async fn delete_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<Option<ContractDocumentRemoval>, Error> {
        let VerifiedDocumentDeletion(removal) = broadcast_moderation(
            self,
            sdk,
            contract_id,
            ContractUserModerationAction::DeleteDocument {
                document_type_name,
                document_id,
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await?;
        Ok(removal)
    }

    async fn delete_settled_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<(Identifier, GroupActionStatus), Error> {
        let VerifiedTeamActionSignature(action_id, status) = broadcast_moderation(
            self,
            sdk,
            contract_id,
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name,
                document_id,
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await?;
        Ok((action_id, status))
    }

    async fn approve_contract_team_action<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        action_id: Identifier,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<GroupActionStatus, Error> {
        let VerifiedTeamActionSignature(approved_action_id, status) = broadcast_moderation(
            self,
            sdk,
            contract_id,
            ContractUserModerationAction::ApproveTeamAction { action_id },
            signing_key_to_use,
            signer,
            settings,
        )
        .await?;
        // The verifier reads the action id out of the transition, so this holds whenever the
        // proof verified; checked so a verifier that answered for another action is not
        // taken for this one.
        if approved_action_id != action_id {
            return Err(Error::Generic(format!(
                "the proof of the approval of team action {action_id} shows team action \
                 {approved_action_id}"
            )));
        }
        Ok(status)
    }

    async fn restore_contract_document<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract: &DataContract,
        document_type_name: String,
        document: &Document,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ContractDocumentRemoval, Error> {
        let document_type = contract
            .document_type_for_name(&document_type_name)
            .map_err(dpp::ProtocolError::from)?;
        // Serialized under the contract as given, which must be its current version: Drive
        // decodes the bytes under the type as the contract holds it now and hashes them
        // against the document as it was serialized when deleted, so a type whose layout
        // changed inside the restore window leaves the record unrestorable whatever the
        // caller serializes with.
        let document_bytes = document.serialize(document_type, contract, sdk.version())?;
        // The verifier reads the document's id out of the bytes under the contract's document
        // type, through the context provider: what the caller serialized with is what it must
        // resolve.
        if let Some(provider) = sdk.context_provider() {
            provider.register_data_contract(Arc::new(contract.clone()));
        }
        let VerifiedDocumentRemoval(removal) = broadcast_moderation(
            self,
            sdk,
            contract.id(),
            ContractUserModerationAction::RestoreDocument {
                document_type_name,
                document: document_bytes.into(),
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await?;
        Ok(removal)
    }

    async fn change_contract_document_fields<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        document_type_name: String,
        document_id: Identifier,
        fields: BTreeMap<String, Value>,
        reason: ContractModerationReason,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<Document, Error> {
        let VerifiedChangedDocument(document) = broadcast_moderation(
            self,
            sdk,
            contract_id,
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name,
                document_id,
                // Signed in the form an object or JSON copy of the transition reads back in
                fields: fields
                    .into_iter()
                    .map(|(name, value)| {
                        (
                            name,
                            ContractUserModerationAction::canonical_field_value(value),
                        )
                    })
                    .collect(),
                reason,
            },
            signing_key_to_use,
            signer,
            settings,
        )
        .await?;
        Ok(document)
    }
}

/// Makes sure the SDK's context provider can resolve `contract_id`, which the verifier of an
/// execution proof reads the contract through. A provider that can not resolve the contract
/// would refuse a result the network already accepted, so this runs before the nonce is taken
/// and anything is signed or paid for.
///
/// With `refresh` the contract is fetched and registered even when the provider already holds
/// a copy, for a proof that depends on a part of the contract an update may have changed.
/// Without it, a copy the provider holds will do.
pub(super) async fn ensure_provider_resolves_contract(
    sdk: &Sdk,
    contract_id: Identifier,
    refresh: bool,
) -> Result<(), Error> {
    let Some(provider) = sdk.context_provider() else {
        return Ok(());
    };
    let resolved = provider
        .get_data_contract(&contract_id, sdk.version())
        .ok()
        .flatten()
        .is_some();
    if refresh || !resolved {
        let contract = DataContract::fetch(sdk, contract_id)
            .await?
            .ok_or_else(|| Error::Generic(format!("data contract {contract_id} does not exist")))?;
        provider.register_data_contract(Arc::new(contract));
    }
    Ok(())
}

/// Signs `action` for `contract_id` with an identity's CRITICAL authentication key, broadcasts
/// it and waits for the state it affected: what every moderation of the trait does, whatever
/// its proof shows.
async fn broadcast_moderation<S: Signer<IdentityPublicKey> + Send, R>(
    identity: &Identity,
    sdk: &Sdk,
    contract_id: Identifier,
    action: ContractUserModerationAction,
    signing_key_to_use: Option<&IdentityPublicKey>,
    signer: S,
    settings: Option<PutSettings>,
) -> Result<R, Error>
where
    R: TryFrom<StateTransitionProofResult> + Send,
{
    let signing_key_id = match signing_key_to_use {
        Some(key) => key.id(),
        None => signing_key_for_moderation(identity, &signer)?,
    };

    // The proof of a ban covers every barring list the contract keeps, which the verifier reads
    // from the contract through the context provider. A provider that can not resolve the contract
    // would refuse a result the network already accepted, so before the nonce is taken and
    // anything is signed or paid for, the provider is asked, and only when it does not have
    // the contract (the lists never change, so whatever copy it holds will do) is the contract
    // fetched and registered with it. A document deletion's verifier reads from the contract
    // whether the type keeps removal records, and so which proof to expect; a restore's
    // verifier decodes the document under the contract's document type, and a field change's
    // verifier reads the changed document back under it, so all three need the contract too.
    // A team action's proposal or approval is verified by the signer's approval of the
    // action, a query rebuilt from the transition alone, and needs none.
    if matches!(
        action,
        ContractUserModerationAction::Ban { .. }
            | ContractUserModerationAction::DeleteDocument { .. }
            | ContractUserModerationAction::RestoreDocument { .. }
            | ContractUserModerationAction::ChangeDocumentFields { .. }
    ) {
        ensure_provider_resolves_contract(sdk, contract_id, false).await?;
    }

    let identity_contract_nonce = sdk
        .get_identity_contract_nonce(identity.id(), contract_id, true, settings)
        .await?;
    let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
    let state_transition = ContractUserModerationTransition::try_from_identity_with_signer(
        identity,
        &signing_key_id,
        contract_id,
        action,
        identity_contract_nonce,
        user_fee_increase.unwrap_or_default(),
        &signer,
        sdk.version(),
        None,
    )
    .await?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    state_transition
        .broadcast_and_wait_for_affected_state(sdk, settings)
        .await
}

/// The first enabled CRITICAL authentication key without contract bounds (a bound key may
/// only sign batches) that the signer can sign with.
pub(super) fn signing_key_for_moderation<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    signer: &S,
) -> Result<KeyID, Error> {
    identity
        .public_keys()
        .values()
        .find(|key| {
            key.purpose() == Purpose::AUTHENTICATION
                && key.security_level() == SecurityLevel::CRITICAL
                && key.disabled_at().is_none()
                && key.contract_bounds().is_none()
                && signer.can_sign_with(key)
        })
        .map(|key| key.id())
        .ok_or_else(|| {
            Error::Generic(
                "the signer holds no CRITICAL authentication key without contract bounds of this identity"
                    .to_string(),
            )
        })
}
