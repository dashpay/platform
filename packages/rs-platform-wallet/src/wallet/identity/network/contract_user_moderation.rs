//! Moderate a data contract's users and documents on `IdentityWallet` (protocol version 14).
//!
//! A contract that declares moderation keeps a banlist, a suspension list and/or a warning
//! list, and its document types may let moderators delete, restore and change the fields of
//! their documents; an elected contract's seated team also deletes settled documents
//! together. Each of these is a `ContractUserModeration` transition, which rs-sdk builds, signs
//! and proves (`dash_sdk::platform::transition::contract_user_moderation`), and Drive decides
//! whether it is allowed. This module sends each one for a signing identity this wallet holds,
//! and answers which of the wallet's identities may act on a contract and what they may offer
//! ([`contract_moderation_offer`]), from the owners of those rules: dpp's
//! [`ContractModerationConfig`], [`ElectedModerators`] and document type accessors, and
//! rs-sdk's [`ModerationTeam`] for an elected contract's seated team.
//!
//! What depends on state Drive reads when the transition executes (whether the target is
//! already banned, carries the most warnings, is protected, whether the document exists, is
//! still inside its window or settled, whether a team action still needs approvals) is left
//! to Platform: its refusal reaches the caller with its consensus error, and the proved result
//! of an accepted transition says where things now stand.

use std::collections::{BTreeMap, BTreeSet};

use dash_sdk::platform::moderation_charters::ModerationTeam;
use dash_sdk::platform::transition::contract_user_moderation::{
    ModerateContractUser, ModeratedUserStatus,
};
use dash_sdk::platform::{DocumentQuery, Fetch};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{
    ContractDocumentRemoval, ContractModerationConfig, ContractModerationList,
    ContractModerationReason, ElectedModerators, ModerationAbility, SettledDeletionRule,
};
use dpp::data_contract::config::v2::DataContractConfigGettersV2;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::group::group_action_status::GroupActionStatus;
use dpp::identity::signer::Signer;
use dpp::identity::{IdentityPublicKey, TimestampMillis};
use dpp::platform_value::Value;
use dpp::prelude::{DataContract, Identifier};
use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationAction;
use std::sync::Arc;

use crate::error::PlatformWalletError;

use super::contract_fee_claim::moderators_team;
use super::document::SignerRef;
use super::*;

/// One of the six moderations of an identity on a contract's lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContractUserModerationKind {
    /// Put the identity on the banlist.
    Ban,
    /// Take the identity off the banlist.
    Unban,
    /// Put the identity on the suspension list until a block time.
    Suspend,
    /// Take the identity off the suspension list.
    Unsuspend,
    /// Add a warning to the identity's entry on the warning list.
    Warn,
    /// Take the identity off the warning list, every warning with it.
    ClearWarnings,
}

impl ContractUserModerationKind {
    /// Every kind, in the order the lists are kept: the banlist's, the suspension list's, the
    /// warning list's.
    pub const ALL: [ContractUserModerationKind; 6] = [
        ContractUserModerationKind::Ban,
        ContractUserModerationKind::Unban,
        ContractUserModerationKind::Suspend,
        ContractUserModerationKind::Unsuspend,
        ContractUserModerationKind::Warn,
        ContractUserModerationKind::ClearWarnings,
    ];

    /// The list the kind edits.
    pub fn list(self) -> ContractModerationList {
        match self {
            ContractUserModerationKind::Ban | ContractUserModerationKind::Unban => {
                ContractModerationList::Banlist
            }
            ContractUserModerationKind::Suspend | ContractUserModerationKind::Unsuspend => {
                ContractModerationList::Suspensions
            }
            ContractUserModerationKind::Warn | ContractUserModerationKind::ClearWarnings => {
                ContractModerationList::Warnings
            }
        }
    }

    /// The ability a seated team needs to edit the kind's list, putting an identity on it or
    /// taking one off: dpp's [`ContractModerationList::ability`], which Drive checks.
    pub fn ability(self) -> ModerationAbility {
        self.list().ability()
    }

    /// The transition's action of this kind against `identity_id`. A ban, a suspension and a
    /// warning carry `reason`; the reversals carry none, and ignore it. A suspension lasts
    /// until `suspend_until`, a block time in milliseconds, which every other kind ignores.
    ///
    /// # Errors
    /// [`PlatformWalletError::InvalidParameter`] for a suspension without `suspend_until`.
    pub fn action(
        self,
        identity_id: Identifier,
        suspend_until: Option<TimestampMillis>,
        reason: ContractModerationReason,
    ) -> Result<ContractUserModerationAction, PlatformWalletError> {
        Ok(match self {
            ContractUserModerationKind::Ban => ContractUserModerationAction::Ban {
                identity_id,
                reason,
            },
            ContractUserModerationKind::Unban => {
                ContractUserModerationAction::Unban { identity_id }
            }
            ContractUserModerationKind::Suspend => ContractUserModerationAction::Suspend {
                identity_id,
                until: suspend_until.ok_or_else(|| {
                    PlatformWalletError::InvalidParameter(
                        "a suspension needs the block time it lasts until".to_string(),
                    )
                })?,
                reason,
            },
            ContractUserModerationKind::Unsuspend => {
                ContractUserModerationAction::Unsuspend { identity_id }
            }
            ContractUserModerationKind::Warn => ContractUserModerationAction::Warn {
                identity_id,
                reason,
            },
            ContractUserModerationKind::ClearWarnings => {
                ContractUserModerationAction::ClearWarnings { identity_id }
            }
        })
    }
}

/// How an identity moderates a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractModeratorRole {
    /// The contract owner, who moderates a contract whose declaration names the owner, an
    /// appointed set, or an interim that does ([`ContractModerationConfig::may_moderate`]).
    Owner,
    /// A moderator the contract appoints beside its owner.
    AppointedModerator,
    /// A moderator the interim of an elected contract appoints, until a team is seated.
    InterimModerator,
    /// The leader of an elected contract's seated team.
    TeamLeader,
    /// An active member of an elected contract's seated team.
    TeamMember,
}

/// What a moderator may do to the documents of one document type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTypeModerationOffer {
    /// Delete a document of the type alone, within its window when it has one
    /// (`moderatorAbilities.delete`, and for a seated team `deleteDocuments` on the type).
    pub delete: bool,
    /// The window in seconds after a document's last modification within which a moderator
    /// deletes it alone (`moderatorAbilities.deleteWithin`); `None` when there is no limit, or
    /// when [`Self::delete`] is not offered.
    pub delete_within_seconds: Option<u32>,
    /// Restore a document of the type a moderator deleted, within a week of its deletion:
    /// the type's deletions keep removal records (`moderatorAbilities.deleteKeepsRecord`) and
    /// the moderator may delete its documents.
    pub restore: bool,
    /// The fields of the type only moderators write that this moderator may change
    /// (`moderatorAbilities.changeFields`, and for a seated team `changeDocumentFields` on the
    /// type). Empty when it may change none.
    pub changeable_fields: BTreeSet<String>,
    /// For a member of a seated team, the rule a proposed deletion of a settled document of
    /// the type must meet (`moderatorAbilities.deleteSettled`): how many members approve, the
    /// leader among them or not. `None` when this moderator may propose none.
    pub settled_deletion: Option<SettledDeletionRule>,
}

impl DocumentTypeModerationOffer {
    /// Whether anything is offered on the type.
    pub fn offers_anything(&self) -> bool {
        self.delete
            || self.restore
            || !self.changeable_fields.is_empty()
            || self.settled_deletion.is_some()
    }
}

/// What one identity of a wallet may do on a contract as its moderator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityModerationOffer {
    /// The identity.
    pub identity_id: Identifier,
    /// How it moderates the contract.
    pub role: ContractModeratorRole,
    /// The moderations of an identity it may send: those of the lists the contract keeps,
    /// and for a seated team only those whose ability the declaration gives it.
    pub user_actions: BTreeSet<ContractUserModerationKind>,
    /// What it may do to the documents of each document type, by name: only the types on
    /// which it may do something.
    pub document_types: BTreeMap<String, DocumentTypeModerationOffer>,
    /// Whether it may approve the team actions other members of the seated team proposed.
    pub approve_team_actions: bool,
}

/// Which identities of a wallet may moderate a contract and what each may do, as the contract
/// and, for an elected contract, the moderation charters contract read when this was asked.
///
/// It names who may sign what, from the contract's declaration and the seated team; whether a
/// given target, document or team action admits the moderation is Drive's to check when the
/// transition is sent, and its refusal comes back with its consensus error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractModerationOffer {
    /// The contract asked about.
    pub contract_id: Identifier,
    /// Who moderates the contract: nobody, its declared moderators, the interim of an
    /// elected contract, or the seated team.
    pub moderators: ModeratorsPotTeam,
    /// The wallet's identities that moderate the contract, in identity id order; empty when
    /// none does.
    pub identities: Vec<IdentityModerationOffer>,
}

/// What a moderator's document deletion left, as its proof shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDocumentDeletion {
    /// The deleted document's id.
    pub document_id: Identifier,
    /// The removal record the deletion left under the contract, with the values it keeps read
    /// under the document type; `None` on a type whose deletions keep no record
    /// (`moderatorAbilities.deleteKeepsRecord: false`), whose proof shows the document gone.
    pub removal: Option<(ContractDocumentRemoval, BTreeMap<String, Value>)>,
    /// The document as it was just before the deletion, serialized under its type
    /// (`Document::serialize`): what a restore takes back, byte for byte.
    pub document_bytes: Vec<u8>,
}

/// The removal record a restore marked, as its proof shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ContractDocumentRestore {
    /// The restored document's id.
    pub document_id: Identifier,
    /// The removal record, now marked restored.
    pub removal: ContractDocumentRemoval,
    /// The values the record keeps, read under the document type.
    pub kept_values: BTreeMap<String, Value>,
}

impl IdentityWallet {
    /// Which of this wallet's signing identities may moderate `contract_id`, and what each
    /// may do.
    ///
    /// Reads the contract and, when it declares elected moderation, the team of its seated
    /// charter (`Sdk::fetch_moderation_team`), then answers with
    /// [`contract_moderation_offer`]. A host offers a moderation only to the identities named
    /// here; Drive still checks it when it is sent. Like that function, it names roles, not
    /// keys.
    ///
    /// # Errors
    /// * [`PlatformWalletError::ContractNotFound`] when Platform holds no such contract;
    /// * [`PlatformWalletError::Sdk`] when a read fails.
    pub async fn contract_moderation_offer(
        &self,
        contract_id: &Identifier,
    ) -> Result<ContractModerationOffer, PlatformWalletError> {
        let (contract, seated_team) = self.fetch_contract_and_seated_team(contract_id).await?;
        let wallet_identities = self.wallet_signing_identity_ids().await?;
        Ok(contract_moderation_offer(
            &contract,
            seated_team.as_ref(),
            wallet_identities,
        ))
    }

    /// Send one moderation of `target_identity_id` on `contract_id`, signed by
    /// `moderator_identity_id`, an identity this wallet holds, and return the target's status
    /// on the list it edited as the proof shows it (a ban proves every barring list the
    /// contract keeps).
    ///
    /// rs-sdk signs with the moderator's first CRITICAL authentication key without contract
    /// bounds that `signer` can sign with, and waits for the proved status. A ban, a suspension
    /// and a warning store `reason`; a suspension lasts until `suspend_until`, a block time in
    /// milliseconds. The signer is called with the wallet manager lock released.
    ///
    /// # Errors
    /// * [`PlatformWalletError::InvalidParameter`] for a suspension without `suspend_until`;
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures. Platform's refusals keep
    ///   their consensus error: a list the contract does not keep, a signer that does not
    ///   moderate it or a seated team without the ability, a protected or missing target, a
    ///   status the moderation does not fit (already banned, not suspended, the most warnings
    ///   reached, a suspension end not in the future), a seated team's reason its proposal
    ///   does not list.
    #[allow(clippy::too_many_arguments)]
    pub async fn moderate_contract_user_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        target_identity_id: &Identifier,
        kind: ContractUserModerationKind,
        suspend_until: Option<TimestampMillis>,
        reason: ContractModerationReason,
        signer: &S,
    ) -> Result<ModeratedUserStatus, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let action = kind.action(*target_identity_id, suspend_until, reason)?;
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        Ok(moderator
            .moderate_contract_user(
                &self.sdk,
                *contract_id,
                action,
                None,
                SignerRef(signer),
                None,
            )
            .await?)
    }

    /// Delete document `document_id` of `document_type_name` on `contract_id` as a moderator,
    /// whoever owns it, for `reason`, signed by `moderator_identity_id`, an identity this
    /// wallet holds.
    ///
    /// Reads the contract, and the document as it stands, which it returns serialized so the
    /// caller can keep it for a restore (the removal record holds only its hash). A document
    /// that read proves absent is refused there, before anything is signed: sent, the deletion
    /// would only be a paid refusal. Then rs-sdk signs, broadcasts and waits for the proved
    /// removal record, whose kept values are read under the document type, or, on a type that
    /// keeps no record, the proved absence.
    ///
    /// The read and the broadcast are not atomic. A document deleted between them is refused
    /// by Platform with its consensus error, as before; one created between them under the id
    /// read as absent is not deleted, and the moderator asks again. Both are accepted: the
    /// window is one round trip, and neither loses anything.
    ///
    /// # Errors
    /// * [`PlatformWalletError::ContractNotFound`] when Platform holds no such contract;
    /// * [`PlatformWalletError::InvalidParameter`] when the contract has no such type;
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::DocumentNotFound`] when the read proves the document absent;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures, Platform's refusals with
    ///   their consensus error: a type moderators may not delete from, a signer that does not
    ///   moderate the contract, a protected owner, a document gone since the read, one past
    ///   the type's window (`DocumentModerationWindowElapsedError`).
    #[allow(clippy::too_many_arguments)]
    pub async fn delete_contract_document_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        reason: ContractModerationReason,
        signer: &S,
    ) -> Result<ContractDocumentDeletion, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        let contract = Arc::new(self.fetch_moderated_contract(contract_id).await?);
        let document_type = moderated_document_type(&contract, document_type_name)?;
        let query = DocumentQuery::new(Arc::clone(&contract), document_type_name)
            .map_err(|error| {
                PlatformWalletError::InvalidParameter(format!(
                    "cannot read document {document_id} of {document_type_name}: {error}"
                ))
            })?
            .with_document_id(document_id);
        let document = Document::fetch(&self.sdk, query).await?.ok_or_else(|| {
            PlatformWalletError::DocumentNotFound {
                contract_id: *contract_id,
                document_type_name: document_type_name.to_string(),
                document_id: *document_id,
            }
        })?;
        let document_bytes = document
            .serialize(document_type, &contract, self.sdk.version())
            .map_err(dash_sdk::Error::from)?;

        let removal = moderator
            .delete_contract_document(
                &self.sdk,
                *contract_id,
                document_type_name.to_string(),
                *document_id,
                reason,
                None,
                SignerRef(signer),
                None,
            )
            .await?;
        let removal = match removal {
            Some(removal) => {
                let kept_values = removal
                    .kept_values_with_default_budget(document_type)
                    .map_err(dash_sdk::Error::from)?;
                Some((removal, kept_values))
            }
            None => None,
        };
        Ok(ContractDocumentDeletion {
            document_id: *document_id,
            removal,
            document_bytes,
        })
    }

    /// Restore a document of `document_type_name` on `contract_id` that a moderator deleted,
    /// signed by `moderator_identity_id`, an identity this wallet holds: `document_bytes` is
    /// the document as it was, serialized under its type, as
    /// [`Self::delete_contract_document_with_signer`] returns it.
    ///
    /// Reads the contract as it is now, under whose document type the bytes are decoded and
    /// the restore is serialized and proved, then rs-sdk signs, broadcasts and waits for the
    /// removal record, now marked restored.
    ///
    /// # Errors
    /// * [`PlatformWalletError::ContractNotFound`] when Platform holds no such contract;
    /// * [`PlatformWalletError::InvalidParameter`] when the contract has no such type or the
    ///   bytes are not a document of it;
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures, Platform's refusals with
    ///   their consensus error: no removal record, one already restored, a restore past the
    ///   week, a document that is not the one deleted, a deletion the seated team approved
    ///   together (`SettledDeletionNotRestorableError`), a unique value taken meanwhile.
    pub async fn restore_contract_document_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_bytes: &[u8],
        signer: &S,
    ) -> Result<ContractDocumentRestore, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        let contract = self.fetch_moderated_contract(contract_id).await?;
        let document_type = moderated_document_type(&contract, document_type_name)?;
        let document = Document::from_bytes(document_bytes, document_type, self.sdk.version())
            .map_err(|error| {
                PlatformWalletError::InvalidParameter(format!(
                    "the bytes are not a {document_type_name} document: {error}"
                ))
            })?;
        let document_id = document.id();
        let removal = moderator
            .restore_contract_document(
                &self.sdk,
                &contract,
                document_type_name.to_string(),
                &document,
                None,
                SignerRef(signer),
                None,
            )
            .await?;
        let kept_values = removal
            .kept_values_with_default_budget(document_type)
            .map_err(dash_sdk::Error::from)?;
        Ok(ContractDocumentRestore {
            document_id,
            removal,
            kept_values,
        })
    }

    /// Set the fields `fields_json` names on document `document_id` of `document_type_name` on
    /// `contract_id`, whoever owns it, for `reason`, signed by `moderator_identity_id`, an
    /// identity this wallet holds, and return the document as it now stands.
    ///
    /// `fields_json` is a JSON object keyed by top-level property name, a `null` removing the
    /// field; its values are read as a document create reads its properties (identifiers as
    /// base58, byte arrays as base64 or hex), under the document type of the contract read
    /// here.
    ///
    /// # Errors
    /// * [`PlatformWalletError::InvalidParameter`] for `fields_json` that is not a JSON object,
    ///   or a contract without such a type;
    /// * [`PlatformWalletError::ContractNotFound`] when Platform holds no such contract;
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures, Platform's refusals with
    ///   their consensus error: a field the type does not keep for its moderators, a signer
    ///   that does not moderate the contract, a seated team without `changeDocumentFields`, a
    ///   change that leaves the document outside its schema.
    #[allow(clippy::too_many_arguments)]
    pub async fn change_contract_document_fields_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        fields_json: &str,
        reason: ContractModerationReason,
        signer: &S,
    ) -> Result<Document, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let mut fields: BTreeMap<String, Value> =
            serde_json::from_str::<serde_json::Value>(fields_json)
                .and_then(serde_json::from_value)
                .map_err(|error| {
                    PlatformWalletError::InvalidParameter(format!(
                        "the fields must be a JSON object keyed by property name: {error}"
                    ))
                })?;
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        let contract = self.fetch_moderated_contract(contract_id).await?;
        moderated_document_type(&contract, document_type_name)?
            .sanitize_document_properties(&mut fields);
        Ok(moderator
            .change_contract_document_fields(
                &self.sdk,
                *contract_id,
                document_type_name.to_string(),
                *document_id,
                fields,
                reason,
                None,
                SignerRef(signer),
                None,
            )
            .await?)
    }

    /// Propose, as a member of the seated team of `contract_id`, the deletion of settled
    /// document `document_id` of `document_type_name`, for `reason`, signed by
    /// `moderator_identity_id`, an identity this wallet holds. Returns the team action's id,
    /// which the other members approve, and its status as the proof shows it: active while
    /// the approvals fall short of the type's rule, closed once the document is gone.
    ///
    /// # Errors
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures, Platform's refusals with
    ///   their consensus error: a type without `deleteSettled`, no seated team, a signer not on
    ///   it, a reason the team's proposal does not list, a document not settled yet, one
    ///   created before a member the leader added (`ContractTeamMemberAddedAfterDocumentError`).
    #[allow(clippy::too_many_arguments)]
    pub async fn propose_settled_contract_document_deletion_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        document_type_name: &str,
        document_id: &Identifier,
        reason: ContractModerationReason,
        signer: &S,
    ) -> Result<(Identifier, GroupActionStatus), PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        Ok(moderator
            .delete_settled_contract_document(
                &self.sdk,
                *contract_id,
                document_type_name.to_string(),
                *document_id,
                reason,
                None,
                SignerRef(signer),
                None,
            )
            .await?)
    }

    /// Approve, as a member of the seated team of `contract_id`, team action `action_id`
    /// another member proposed, signed by `moderator_identity_id`, an identity this wallet
    /// holds. Returns the action's status as the proof shows it: active while the approvals
    /// still fall short of the rule, closed once the action ran.
    ///
    /// # Errors
    /// * [`PlatformWalletError::IdentityNotFound`] when this wallet holds no signing identity
    ///   `moderator_identity_id`;
    /// * [`PlatformWalletError::Sdk`] for SDK and Platform failures, Platform's refusals with
    ///   their consensus error: no such action, one this member already approved, one already
    ///   closed, a document changed since the proposal, a member added after the document.
    pub async fn approve_contract_team_action_with_signer<S>(
        &self,
        moderator_identity_id: &Identifier,
        contract_id: &Identifier,
        action_id: &Identifier,
        signer: &S,
    ) -> Result<GroupActionStatus, PlatformWalletError>
    where
        S: Signer<IdentityPublicKey> + Send + Sync,
    {
        let moderator = self.wallet_signing_identity(moderator_identity_id).await?;
        Ok(moderator
            .approve_contract_team_action(
                &self.sdk,
                *contract_id,
                *action_id,
                None,
                SignerRef(signer),
                None,
            )
            .await?)
    }

    /// `contract_id` as Platform holds it now, registered with the SDK's context provider for
    /// the proof of the moderation that follows.
    async fn fetch_moderated_contract(
        &self,
        contract_id: &Identifier,
    ) -> Result<DataContract, PlatformWalletError> {
        let contract = DataContract::fetch(&self.sdk, *contract_id).await?.ok_or(
            PlatformWalletError::ContractNotFound {
                contract_id: *contract_id,
            },
        )?;
        self.register_contract_for_proof_verification(&contract);
        Ok(contract)
    }
}

/// The document type `document_type_name` of `contract`.
fn moderated_document_type<'a>(
    contract: &'a DataContract,
    document_type_name: &str,
) -> Result<DocumentTypeRef<'a>, PlatformWalletError> {
    contract
        .document_type_for_name(document_type_name)
        .map_err(|error| {
            PlatformWalletError::InvalidParameter(format!(
                "contract {} has no document type {document_type_name}: {error}",
                contract.id()
            ))
        })
}

/// How one candidate moderates, and with what authority: the declared moderators hold every
/// ability the contract backs, a seated team only those its declaration gives it.
enum Authority<'a> {
    Declared,
    Seated(&'a ElectedModerators),
}

impl Authority<'_> {
    /// Whether the authority holds `ability` on `document_type_name`, or on some moderated
    /// type when `None` (the lists are contract-wide): [`ElectedModerators::allows`] and
    /// [`ElectedModerators::allows_on_any_type`] for a seated team.
    fn holds(&self, ability: ModerationAbility, document_type_name: Option<&str>) -> bool {
        match (self, document_type_name) {
            (Authority::Declared, _) => true,
            (Authority::Seated(elected), Some(name)) => elected.allows(name, ability),
            (Authority::Seated(elected), None) => elected.allows_on_any_type(ability),
        }
    }
}

/// Which of `candidates` may moderate `contract` and what each may do, given the team of the
/// contract's seated charter (`seated_team`, read only for a contract that declares elected
/// moderation; ignored for any other).
///
/// Who moderates is the choice Drive makes: the leader and the active members of a seated
/// team ([`ModerationTeam::contains`]) once an elected contract has one, otherwise whoever the
/// declaration lets moderate ([`ContractModerationConfig::may_moderate`]). Then:
/// * the moderations of an identity are those of the lists the contract keeps
///   ([`ContractModerationConfig::keeps`]), for a seated team only with the list's ability
///   on some moderated type ([`ElectedModerators::allows_on_any_type`]);
/// * a document type offers deletion when its moderators may delete its documents
///   (`documents_can_be_deleted_by_moderators`) and, for a seated team, it holds
///   `deleteDocuments` on the type ([`ElectedModerators::allows`]); restoring when it also
///   keeps removal records (`moderator_deletions_keep_records`); the fields it keeps for its
///   moderators (`moderator_changeable_fields`) with `changeDocumentFields` for a seated team;
///   and, to a seated team holding `deleteDocuments` on it, the proposal of a settled
///   document's deletion under its rule (`moderator_settled_deletion`, beside a
///   `deleteWithin` window);
/// * only a seated team's members approve team actions.
///
/// It is role-based, not key-based: a candidate is named by how it moderates the contract,
/// whether or not the wallet's signer holds a key it can sign with. A moderation needs a
/// CRITICAL authentication key without contract bounds; a named identity without one fails
/// when the moderation is sent, and a host shows that error where it was asked for.
pub fn contract_moderation_offer(
    contract: &DataContract,
    seated_team: Option<&ModerationTeam>,
    candidates: impl IntoIterator<Item = Identifier>,
) -> ContractModerationOffer {
    let candidates: BTreeSet<Identifier> = candidates.into_iter().collect();
    let moderators = moderators_team(contract, seated_team);
    let identities = match contract.config().moderation() {
        None => Vec::new(),
        Some(moderation) => candidates
            .into_iter()
            .filter_map(|candidate| identity_offer(contract, moderation, seated_team, candidate))
            .collect(),
    };
    ContractModerationOffer {
        contract_id: contract.id(),
        moderators,
        identities,
    }
}

/// What `candidate` may do on `contract`, which declares `moderation`; `None` when it does
/// not moderate the contract.
fn identity_offer(
    contract: &DataContract,
    moderation: &ContractModerationConfig,
    seated_team: Option<&ModerationTeam>,
    candidate: Identifier,
) -> Option<IdentityModerationOffer> {
    let owner_id = contract.owner_id();
    let elected = moderation.moderators.elected();
    let (role, authority) = match (elected, seated_team) {
        (Some(elected), Some(team)) => {
            if !team.contains(&candidate) {
                return None;
            }
            let role = if candidate == team.leader_id {
                ContractModeratorRole::TeamLeader
            } else {
                ContractModeratorRole::TeamMember
            };
            (role, Authority::Seated(elected))
        }
        _ => {
            if !moderation.may_moderate(&owner_id, &candidate) {
                return None;
            }
            let role = if candidate == owner_id {
                ContractModeratorRole::Owner
            } else if elected.is_some() {
                ContractModeratorRole::InterimModerator
            } else {
                ContractModeratorRole::AppointedModerator
            };
            (role, Authority::Declared)
        }
    };
    let seated = matches!(authority, Authority::Seated(_));

    let user_actions = ContractUserModerationKind::ALL
        .into_iter()
        .filter(|kind| moderation.keeps(kind.list()) && authority.holds(kind.ability(), None))
        .collect();

    let document_types = contract
        .document_types()
        .iter()
        .filter_map(|(name, document_type)| {
            let document_type = document_type.as_ref();
            let deletes = document_type.documents_can_be_deleted_by_moderators()
                && authority.holds(ModerationAbility::DeleteDocuments, Some(name));
            let changeable_fields =
                if authority.holds(ModerationAbility::ChangeDocumentFields, Some(name)) {
                    document_type.moderator_changeable_fields().clone()
                } else {
                    BTreeSet::new()
                };
            // The window bounds a deletion, so it is offered only beside one
            let delete_within_seconds = if deletes {
                document_type.documents_can_be_deleted_by_moderators_for()
            } else {
                None
            };
            let settled_deletion = if seated && deletes && delete_within_seconds.is_some() {
                document_type.moderator_settled_deletion()
            } else {
                None
            };
            let offer = DocumentTypeModerationOffer {
                delete: deletes,
                delete_within_seconds,
                restore: deletes && document_type.moderator_deletions_keep_records(),
                changeable_fields,
                settled_deletion,
            };
            offer.offers_anything().then(|| (name.clone(), offer))
        })
        .collect();

    Some(IdentityModerationOffer {
        identity_id: candidate,
        role,
        user_actions,
        document_types,
        approve_team_actions: seated,
    })
}

#[cfg(test)]
mod tests {
    use super::super::decision_proposal::tests::{owner, proposals_sdk, FixedSignatureSigner};
    use super::super::signing_key::tests::{lock_checking_signer, wallet_with_signing_keys_on};
    use super::*;
    use crate::wallet::persister::NoPlatformPersistence;
    use dash_sdk::dapi_grpc::platform::v0::{BroadcastStateTransitionRequest, GetDocumentsRequest};
    use dash_sdk::error::StateTransitionBroadcastError;
    use dash_sdk::platform::transition::broadcast_request::BroadcastRequestForStateTransition;
    use dash_sdk::platform::Query;
    use dash_sdk::query_types::IdentityContractNonceFetcher;
    use dash_sdk::Sdk;
    use dpp::consensus::state::contract_moderation::ContractUserAlreadyBannedError;
    use dpp::consensus::state::state_error::StateError;
    use dpp::consensus::ConsensusError;
    use dpp::data_contract::config::moderation::elected::InterimModerators;
    use dpp::data_contract::config::moderation::ContractModerators;
    use dpp::data_contract::config::DataContractConfig;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::DocumentV0;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::platform_value;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
    use dpp::state_transition::contract_user_moderation_transition::ContractUserModerationTransition;
    use dpp::version::PlatformVersion;

    const STORED_NONCE: u64 = 9;
    const OWNER: u8 = 0x0A;

    fn id(byte: u8) -> Identifier {
        Identifier::from([byte; 32])
    }

    fn ids(bytes: &[u8]) -> BTreeSet<Identifier> {
        bytes.iter().copied().map(id).collect()
    }

    /// A `post` its moderators delete within a day, whose `flag` only they write; a `note`
    /// they have no ability on.
    fn schemas() -> Value {
        platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "flag": { "type": "string", "maxLength": 20, "position": 1 },
                },
                "additionalProperties": false,
                "required": ["$createdAt", "$updatedAt"],
                "moderatorAbilities": {
                    "delete": true,
                    "deleteWithin": 86400,
                    "changeFields": ["flag"],
                },
            },
            "note": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
            },
        })
    }

    /// [`schemas`] with `post`'s settled documents deleted by the seated team, its leader
    /// alone meeting the rule.
    fn settled_schemas() -> Value {
        platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "flag": { "type": "string", "maxLength": 20, "position": 1 },
                },
                "additionalProperties": false,
                "required": ["$createdAt", "$updatedAt"],
                "moderatorAbilities": {
                    "delete": true,
                    "deleteWithin": 86400,
                    "changeFields": ["flag"],
                    "deleteSettled": { "leader": true, "approvals": 1 },
                },
            },
            "note": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                },
                "additionalProperties": false,
            },
        })
    }

    /// A `post` whose moderators change its `flag` text, its `curator` identifier and its
    /// `digest` bytes.
    fn changeable_schemas() -> Value {
        platform_value!({
            "post": {
                "type": "object",
                "properties": {
                    "text": { "type": "string", "maxLength": 50, "position": 0 },
                    "flag": { "type": "string", "maxLength": 20, "position": 1 },
                    "curator": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 2,
                    },
                    "digest": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 4,
                        "maxItems": 4,
                        "position": 3,
                    },
                },
                "additionalProperties": false,
                "required": ["$createdAt", "$updatedAt"],
                "moderatorAbilities": {
                    "delete": true,
                    "changeFields": ["flag", "curator", "digest"],
                },
            },
        })
    }

    /// A contract of `owner` declaring `moderation` over `schemas`.
    fn contract_of(
        owner: Identifier,
        moderation: Option<ContractModerationConfig>,
        schemas: Value,
    ) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let config = DataContractConfig::default_for_version(platform_version)
            .expect("a default config")
            .with_moderation(moderation);
        DataContractFactory::new(platform_version.protocol_version)
            .expect("a factory")
            .create(owner, 1, schemas, Some(config), None)
            .expect("the contract")
            .data_contract_owned()
    }

    fn lists(moderators: ContractModerators) -> ContractModerationConfig {
        ContractModerationConfig {
            banlist: true,
            suspensions: true,
            moderators,
            warnings: false,
        }
    }

    /// An elected declaration over `post` with `abilities`, `interim` until a team is seated.
    fn elected(
        interim: InterimModerators,
        abilities: &[ModerationAbility],
    ) -> ContractModerationConfig {
        lists(ContractModerators::Elected(Box::new(ElectedModerators {
            join_window: 0,
            vote_window: 0,
            challenge_cool_down: None,
            election_delay: None,
            max_added_moderators: 0,
            moderated_document_types: BTreeMap::from([(
                "post".to_string(),
                abilities.iter().copied().collect(),
            )]),
            interim,
            owner_protected: false,
        })))
    }

    fn seated(leader: u8, members: &[u8]) -> ModerationTeam {
        ModerationTeam {
            elected_charter_id: id(0xC1),
            submitted_charter_id: id(0xC2),
            leader_id: id(leader),
            members: members.iter().copied().map(id).collect(),
            elected_members: members.iter().copied().map(id).collect(),
        }
    }

    fn offered(offer: &ContractModerationOffer) -> BTreeSet<Identifier> {
        offer.identities.iter().map(|i| i.identity_id).collect()
    }

    fn of(offer: &ContractModerationOffer, byte: u8) -> &IdentityModerationOffer {
        offer
            .identities
            .iter()
            .find(|i| i.identity_id == id(byte))
            .expect("the identity is offered")
    }

    #[test]
    fn should_offer_nothing_on_a_contract_without_moderation() {
        let contract = contract_of(
            id(OWNER),
            None,
            platform_value!({
                "note": {
                    "type": "object",
                    "properties": { "text": { "type": "string", "maxLength": 5, "position": 0 } },
                    "additionalProperties": false,
                },
            }),
        );
        let offer = contract_moderation_offer(&contract, None, [id(OWNER), id(2)]);
        assert_eq!(offer.moderators, ModeratorsPotTeam::NoModeration);
        assert!(offer.identities.is_empty());
        assert_eq!(offer.contract_id, contract.id());
    }

    /// The owner and the appointed set moderate, each with the actions of the lists the
    /// contract keeps, and every ability the document types grant.
    #[test]
    fn should_offer_the_owner_and_the_appointed_moderators_the_kept_lists_and_type_abilities() {
        let contract = contract_of(
            id(OWNER),
            Some(lists(ContractModerators::AppointedModerators(ids(&[2])))),
            schemas(),
        );
        let offer = contract_moderation_offer(&contract, None, [id(OWNER), id(2), id(3)]);
        assert_eq!(offer.moderators, ModeratorsPotTeam::Declared);
        assert_eq!(offered(&offer), ids(&[OWNER, 2]));
        assert_eq!(of(&offer, OWNER).role, ContractModeratorRole::Owner);
        let appointed = of(&offer, 2);
        assert_eq!(appointed.role, ContractModeratorRole::AppointedModerator);
        // The banlist and the suspension list are kept, the warning list is not
        assert_eq!(
            appointed.user_actions,
            BTreeSet::from([
                ContractUserModerationKind::Ban,
                ContractUserModerationKind::Unban,
                ContractUserModerationKind::Suspend,
                ContractUserModerationKind::Unsuspend,
            ])
        );
        assert!(!appointed.approve_team_actions);

        let post_type = contract.document_type_for_name("post").expect("post");
        let post = appointed.document_types.get("post").expect("post offered");
        assert!(post.delete);
        assert_eq!(post.delete_within_seconds, Some(86400));
        assert_eq!(post.restore, post_type.moderator_deletions_keep_records());
        assert_eq!(post.changeable_fields, BTreeSet::from(["flag".to_string()]));
        assert_eq!(post.settled_deletion, None);
        // No ability on `note`, so it is left out
        assert!(!appointed.document_types.contains_key("note"));
    }

    #[test]
    fn should_offer_the_interim_moderators_until_a_team_is_seated() {
        let contract = contract_of(
            id(OWNER),
            Some(elected(
                InterimModerators::AppointedModerators(ids(&[2])),
                &[ModerationAbility::DeleteDocuments],
            )),
            settled_schemas(),
        );
        let offer = contract_moderation_offer(&contract, None, [id(OWNER), id(2), id(5)]);
        assert_eq!(offer.moderators, ModeratorsPotTeam::ElectedInterim);
        assert_eq!(offered(&offer), ids(&[OWNER, 2]));
        let interim = of(&offer, 2);
        assert_eq!(interim.role, ContractModeratorRole::InterimModerator);
        // The interim moderates as the declared kinds do: every list kept, every type ability,
        // and no settled deletion, which only a seated team proposes
        assert_eq!(interim.user_actions.len(), 4);
        let post = interim.document_types.get("post").expect("post");
        assert!(post.delete);
        assert_eq!(post.changeable_fields, BTreeSet::from(["flag".to_string()]));
        assert_eq!(post.settled_deletion, None);
        assert!(!interim.approve_team_actions);

        let nobody = contract_of(
            id(OWNER),
            Some(elected(
                InterimModerators::NoModeration,
                &[ModerationAbility::DeleteDocuments],
            )),
            settled_schemas(),
        );
        assert!(contract_moderation_offer(&nobody, None, [id(OWNER)])
            .identities
            .is_empty());
    }

    /// Once a team is seated, its leader and active members moderate with the abilities the
    /// declaration gives the team, the interim (the owner among it) no longer.
    #[test]
    fn should_offer_the_seated_team_only_the_abilities_the_declaration_gives_it() {
        let contract = contract_of(
            id(OWNER),
            Some(elected(
                InterimModerators::ContractOwner,
                &[ModerationAbility::DeleteDocuments, ModerationAbility::Ban],
            )),
            settled_schemas(),
        );
        let team = seated(5, &[6]);
        let offer =
            contract_moderation_offer(&contract, Some(&team), [id(OWNER), id(5), id(6), id(7)]);
        assert_eq!(offer.moderators, ModeratorsPotTeam::ElectedSeated);
        assert_eq!(offered(&offer), ids(&[5, 6]));
        assert_eq!(of(&offer, 5).role, ContractModeratorRole::TeamLeader);
        let member = of(&offer, 6);
        assert_eq!(member.role, ContractModeratorRole::TeamMember);
        // Ban but not Suspend: the team holds the banlist's ability only
        assert_eq!(
            member.user_actions,
            BTreeSet::from([
                ContractUserModerationKind::Ban,
                ContractUserModerationKind::Unban
            ])
        );
        let post = member.document_types.get("post").expect("post");
        assert!(post.delete);
        // No changeDocumentFields on the type
        assert!(post.changeable_fields.is_empty());
        assert_eq!(
            post.settled_deletion,
            contract
                .document_type_for_name("post")
                .expect("post")
                .moderator_settled_deletion()
        );
        assert!(post.settled_deletion.is_some());
        assert!(member.approve_team_actions);
    }

    /// A seated team that may change a type's fields but not delete its documents is offered
    /// no deletion window: the window bounds a deletion it can not make.
    #[test]
    fn should_offer_no_deletion_window_to_a_team_that_may_not_delete() {
        let contract = contract_of(
            id(OWNER),
            Some(elected(
                InterimModerators::ContractOwner,
                &[ModerationAbility::ChangeDocumentFields],
            )),
            settled_schemas(),
        );
        // `post` has a window, and its moderators may delete within it
        assert_eq!(
            contract
                .document_type_for_name("post")
                .expect("post")
                .documents_can_be_deleted_by_moderators_for(),
            Some(86400)
        );
        let team = seated(5, &[6]);
        let offer = contract_moderation_offer(&contract, Some(&team), [id(6)]);
        let post = of(&offer, 6).document_types.get("post").expect("post");
        assert!(!post.delete);
        assert_eq!(post.delete_within_seconds, None);
        assert!(!post.restore);
        assert_eq!(post.settled_deletion, None);
        assert_eq!(post.changeable_fields, BTreeSet::from(["flag".to_string()]));
    }

    /// A seated team passed for a contract that is not elected is not read.
    #[test]
    fn should_ignore_a_seated_team_for_a_contract_that_is_not_elected() {
        let contract = contract_of(
            id(OWNER),
            Some(lists(ContractModerators::ContractOwner)),
            schemas(),
        );
        let team = seated(5, &[6]);
        let offer = contract_moderation_offer(&contract, Some(&team), [id(OWNER), id(5)]);
        assert_eq!(offer.moderators, ModeratorsPotTeam::Declared);
        assert_eq!(offered(&offer), ids(&[OWNER]));
    }

    #[test]
    fn should_build_each_user_moderation_and_refuse_a_suspension_without_an_end() {
        let reason = ContractModerationReason::from_text("spam");
        assert_eq!(
            ContractUserModerationKind::Suspend
                .action(id(2), Some(1_000), reason.clone())
                .expect("a suspension"),
            ContractUserModerationAction::Suspend {
                identity_id: id(2),
                until: 1_000,
                reason: reason.clone(),
            }
        );
        assert!(matches!(
            ContractUserModerationKind::Suspend.action(id(2), None, reason.clone()),
            Err(PlatformWalletError::InvalidParameter(_))
        ));
        assert_eq!(
            ContractUserModerationKind::ClearWarnings
                .action(id(2), Some(1_000), reason.clone())
                .expect("a clearing"),
            ContractUserModerationAction::ClearWarnings { identity_id: id(2) }
        );
        for kind in ContractUserModerationKind::ALL {
            let ability = kind.ability();
            match kind.list() {
                ContractModerationList::Banlist => assert_eq!(ability, ModerationAbility::Ban),
                ContractModerationList::Suspensions => {
                    assert_eq!(ability, ModerationAbility::Suspend)
                }
                ContractModerationList::Warnings => assert_eq!(ability, ModerationAbility::Warn),
            }
        }
    }

    /// The wallet's signing identity (the `owner()` the fixture holds) is offered moderation
    /// of a contract it owns, read once through the SDK; a contract that is not elected reads
    /// no charter (the mock answers nothing else).
    #[tokio::test]
    async fn should_offer_the_wallet_identity_that_owns_the_contract() {
        let contract = contract_of(
            owner(),
            Some(lists(ContractModerators::ContractOwner)),
            schemas(),
        );
        let contract_id = contract.id();
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, Some(contract))
            .await
            .expect("expects the contract read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let offer = wallet
            .identity()
            .contract_moderation_offer(&contract_id)
            .await
            .expect("reads");
        assert_eq!(offer.contract_id, contract_id);
        assert_eq!(offered(&offer), BTreeSet::from([owner()]));
        assert_eq!(offer.identities[0].role, ContractModeratorRole::Owner);
    }

    #[tokio::test]
    async fn should_report_a_contract_platform_does_not_hold() {
        let contract_id = id(0xEE);
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, None::<DataContract>)
            .await
            .expect("expects the contract read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .contract_moderation_offer(&contract_id)
            .await
            .expect_err("no such contract");
        assert!(
            matches!(error, PlatformWalletError::ContractNotFound { contract_id: missing }
                if missing == contract_id),
            "{error:?}"
        );
    }

    /// A moderator the wallet does not hold is refused before anything is read or signed, for
    /// every operation.
    #[tokio::test]
    async fn should_refuse_a_moderator_the_wallet_does_not_hold() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let identity = wallet.identity();
        let stranger = id(5);
        let contract_id = id(0xEE);
        let reason = || ContractModerationReason::from_text("spam");
        let errors = [
            identity
                .moderate_contract_user_with_signer(
                    &stranger,
                    &contract_id,
                    &id(2),
                    ContractUserModerationKind::Ban,
                    None,
                    reason(),
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
            identity
                .delete_contract_document_with_signer(
                    &stranger,
                    &contract_id,
                    "post",
                    &id(3),
                    reason(),
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
            identity
                .restore_contract_document_with_signer(
                    &stranger,
                    &contract_id,
                    "post",
                    &[1, 2, 3],
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
            identity
                .change_contract_document_fields_with_signer(
                    &stranger,
                    &contract_id,
                    "post",
                    &id(3),
                    "{\"flag\": null}",
                    reason(),
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
            identity
                .propose_settled_contract_document_deletion_with_signer(
                    &stranger,
                    &contract_id,
                    "post",
                    &id(3),
                    reason(),
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
            identity
                .approve_contract_team_action_with_signer(
                    &stranger,
                    &contract_id,
                    &id(4),
                    &FixedSignatureSigner,
                )
                .await
                .map(|_| ()),
        ];
        for error in errors {
            assert!(
                matches!(error, Err(PlatformWalletError::IdentityNotFound(missing)) if missing == stranger),
                "{error:?}"
            );
        }
    }

    /// Fields that are not a JSON object are refused before anything else.
    #[tokio::test]
    async fn should_refuse_fields_that_are_not_a_json_object() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        for fields in ["[1, 2]", "not json", "\"flag\""] {
            let error = wallet
                .identity()
                .change_contract_document_fields_with_signer(
                    &owner(),
                    &id(0xEE),
                    "post",
                    &id(3),
                    fields,
                    ContractModerationReason::default(),
                    &FixedSignatureSigner,
                )
                .await
                .expect_err("not an object");
            assert!(
                matches!(error, PlatformWalletError::InvalidParameter(_)),
                "{fields}: {error:?}"
            );
        }
    }

    /// rs-sdk asks the signer for a key with the wallet manager lock released; a signer holding
    /// no CRITICAL key of the moderator stops the moderation before any request.
    #[tokio::test]
    async fn should_ask_the_signer_for_a_key_with_the_wallet_lock_released() {
        let wallet =
            wallet_with_signing_keys_on(proposals_sdk(), Arc::new(NoPlatformPersistence)).await;
        let signer = lock_checking_signer(wallet.identity());
        let error = wallet
            .identity()
            .approve_contract_team_action_with_signer(&owner(), &id(0xEE), &id(4), &signer)
            .await
            .expect_err("no key available");
        assert!(
            matches!(error, PlatformWalletError::Sdk(dash_sdk::Error::Generic(ref message))
                if message.contains("CRITICAL authentication key")),
            "{error:?}"
        );
    }

    /// The broadcast of `action` on `contract_id` as rs-sdk builds it for the wallet's
    /// signing identity (`owner()`): signed with its first CRITICAL key at the nonce after
    /// [`STORED_NONCE`].
    async fn moderation_broadcast(
        identity: &IdentityWallet,
        contract_id: Identifier,
        action: ContractUserModerationAction,
    ) -> BroadcastStateTransitionRequest {
        let moderator = identity
            .wallet_signing_identity(&owner())
            .await
            .expect("the moderator");
        let first_key_id = *moderator
            .public_keys()
            .keys()
            .next()
            .expect("the moderator's first key");
        ContractUserModerationTransition::try_from_identity_with_signer(
            &moderator,
            &first_key_id,
            contract_id,
            action,
            STORED_NONCE + 1,
            0,
            &FixedSignatureSigner,
            PlatformVersion::latest(),
            None,
        )
        .await
        .expect("the moderation")
        .broadcast_request_for_state_transition()
        .expect("serializes")
    }

    /// [`proposals_sdk`] answering a read of `contract` when given, and of the moderator's
    /// contract nonce on `contract_id` with [`STORED_NONCE`]. It answers no broadcast.
    async fn moderation_sdk(contract_id: Identifier, contract: Option<DataContract>) -> Sdk {
        let mut sdk = proposals_sdk();
        if let Some(contract) = contract {
            sdk.mock()
                .expect_fetch(contract_id, Some(contract))
                .await
                .expect("expects the contract read");
        }
        sdk.mock()
            .expect_fetch(
                (owner(), contract_id),
                Some(IdentityContractNonceFetcher(STORED_NONCE)),
            )
            .await
            .expect("expects the nonce read");
        sdk
    }

    /// The moderator-owned contract over [`schemas`].
    fn owned_contract() -> DataContract {
        contract_of(
            owner(),
            Some(lists(ContractModerators::ContractOwner)),
            schemas(),
        )
    }

    /// A `post` as it stands on Platform, owned by someone else.
    fn stored_post() -> Document {
        DocumentV0 {
            id: id(3),
            owner_id: id(2),
            properties: BTreeMap::from([("text".to_string(), Value::Text("hello".to_string()))]),
            revision: Some(1),
            created_at: Some(1_700_000_000_000),
            updated_at: Some(1_700_000_000_000),
            ..Default::default()
        }
        .into()
    }

    /// The query a deletion reads the `post` [`stored_post`] names with.
    fn post_query(contract: &DataContract) -> DocumentQuery {
        DocumentQuery::new(Arc::new(contract.clone()), "post")
            .expect("the post type")
            .with_document_id(&id(3))
    }

    /// A ban reads the moderator's next contract nonce (the contract is resolved first for
    /// the proof) and broadcasts the ban signed with the moderator's first CRITICAL key. The
    /// mock answers no broadcast, so the ban fails after sending it.
    #[tokio::test]
    async fn should_broadcast_the_ban_signed_by_the_moderator() {
        let contract = owned_contract();
        let contract_id = contract.id();
        let mut sdk = moderation_sdk(contract_id, Some(contract)).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;
        let reason = ContractModerationReason::from_text("spam");

        let error = wallet
            .identity()
            .moderate_contract_user_with_signer(
                &owner(),
                &contract_id,
                &id(2),
                ContractUserModerationKind::Ban,
                None,
                reason.clone(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::Ban {
                identity_id: id(2),
                reason,
            },
        )
        .await;
        assert!(
            sdk.mock().request_count(&broadcast).await > 0,
            "the ban must be broadcast at nonce {}",
            STORED_NONCE + 1
        );
    }

    /// A deletion reads the contract and the document as it stands (kept for a restore), then
    /// reads the moderator's nonce and broadcasts the deletion. The mock answers no broadcast.
    #[tokio::test]
    async fn should_read_the_document_before_deleting_it() {
        let contract = owned_contract();
        let contract_id = contract.id();
        let document_query = post_query(&contract);
        let mut sdk = moderation_sdk(contract_id, Some(contract)).await;
        sdk.mock()
            .expect_fetch(document_query.clone(), Some(stored_post()))
            .await
            .expect("expects the document read");
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;
        let reason = ContractModerationReason::from_text("spam");

        let error = wallet
            .identity()
            .delete_contract_document_with_signer(
                &owner(),
                &contract_id,
                "post",
                &id(3),
                reason.clone(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");
        let request: GetDocumentsRequest =
            Query::<GetDocumentsRequest>::query(&document_query, &sdk.query_settings())
                .expect("encodes");
        assert_eq!(sdk.mock().request_count(&request).await, 1);

        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::DeleteDocument {
                document_type_name: "post".to_string(),
                document_id: id(3),
                reason,
            },
        )
        .await;
        assert!(sdk.mock().request_count(&broadcast).await > 0);
    }

    /// A document the read proves absent is refused before the nonce is read or anything is
    /// signed: sent, the deletion would only be a paid refusal. The mock answers no nonce
    /// read, so reaching it would fail with an SDK error instead.
    #[tokio::test]
    async fn should_refuse_to_delete_a_document_the_read_proves_absent() {
        let contract = owned_contract();
        let contract_id = contract.id();
        let document_query = post_query(&contract);
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, Some(contract))
            .await
            .expect("expects the contract read");
        sdk.mock()
            .expect_fetch(document_query, None::<Document>)
            .await
            .expect("expects the document read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .delete_contract_document_with_signer(
                &owner(),
                &contract_id,
                "post",
                &id(3),
                ContractModerationReason::from_text("spam"),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("no such document");
        assert!(
            matches!(
                error,
                PlatformWalletError::DocumentNotFound {
                    contract_id: missing_contract,
                    ref document_type_name,
                    document_id,
                } if missing_contract == contract_id
                    && document_type_name == "post"
                    && document_id == id(3)
            ),
            "{error:?}"
        );
    }

    /// A field change reads its fields as a document create reads its properties, under the
    /// document type of the contract it reads: a base58 identifier and a hex byte array are
    /// coerced, a `null` kept as the removal of the field, and each value signed in its
    /// canonical form. The mock answers no broadcast.
    #[tokio::test]
    async fn should_broadcast_the_field_change_with_its_fields_read_under_the_type() {
        let contract = contract_of(
            owner(),
            Some(lists(ContractModerators::ContractOwner)),
            changeable_schemas(),
        );
        let contract_id = contract.id();
        let mut sdk = moderation_sdk(contract_id, Some(contract)).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;
        let reason = ContractModerationReason::from_text("mislabelled");
        let fields_json = format!(
            "{{\"curator\": \"{}\", \"digest\": \"deadbeef\", \"flag\": null}}",
            id(7).to_string(Encoding::Base58)
        );

        let error = wallet
            .identity()
            .change_contract_document_fields_with_signer(
                &owner(),
                &contract_id,
                "post",
                &id(3),
                &fields_json,
                reason.clone(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let fields = BTreeMap::from([
            ("curator".to_string(), Value::Identifier(id(7).to_buffer())),
            (
                "digest".to_string(),
                Value::Bytes(vec![0xde, 0xad, 0xbe, 0xef]),
            ),
            ("flag".to_string(), Value::Null),
        ])
        .into_iter()
        .map(|(name, value)| {
            (
                name,
                ContractUserModerationAction::canonical_field_value(value),
            )
        })
        .collect();
        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::ChangeDocumentFields {
                document_type_name: "post".to_string(),
                document_id: id(3),
                fields,
                reason,
            },
        )
        .await;
        assert!(
            sdk.mock().request_count(&broadcast).await > 0,
            "the change must be broadcast with the coerced fields"
        );
    }

    /// A restore decodes the kept bytes under the document type of the contract it reads and
    /// broadcasts them, byte for byte, as the restored document. The mock answers no
    /// broadcast.
    #[tokio::test]
    async fn should_broadcast_the_restore_of_the_kept_document() {
        let contract = owned_contract();
        let contract_id = contract.id();
        let document_bytes = stored_post()
            .serialize(
                contract.document_type_for_name("post").expect("post"),
                &contract,
                PlatformVersion::latest(),
            )
            .expect("serializes");
        let mut sdk = moderation_sdk(contract_id, Some(contract)).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .restore_contract_document_with_signer(
                &owner(),
                &contract_id,
                "post",
                &document_bytes,
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::RestoreDocument {
                document_type_name: "post".to_string(),
                document: document_bytes.into(),
            },
        )
        .await;
        assert!(sdk.mock().request_count(&broadcast).await > 0);
    }

    /// A settled deletion's proposal is verified by the signer's approval of the team action,
    /// so it reads no contract: only the moderator's nonce, then the broadcast. The mock
    /// answers no broadcast.
    #[tokio::test]
    async fn should_broadcast_the_proposal_of_a_settled_deletion() {
        let contract_id = id(0xEE);
        let mut sdk = moderation_sdk(contract_id, None).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;
        let reason = ContractModerationReason::from_text("spam").with_reason_document(id(8));

        let error = wallet
            .identity()
            .propose_settled_contract_document_deletion_with_signer(
                &owner(),
                &contract_id,
                "post",
                &id(3),
                reason.clone(),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::DeleteSettledDocument {
                document_type_name: "post".to_string(),
                document_id: id(3),
                reason,
            },
        )
        .await;
        assert!(sdk.mock().request_count(&broadcast).await > 0);
    }

    /// An approval reads no contract either: only the moderator's nonce, then the broadcast.
    /// The mock answers no broadcast.
    #[tokio::test]
    async fn should_broadcast_the_approval_of_a_team_action() {
        let contract_id = id(0xEE);
        let mut sdk = moderation_sdk(contract_id, None).await;
        let wallet =
            wallet_with_signing_keys_on(sdk.clone(), Arc::new(NoPlatformPersistence)).await;

        let error = wallet
            .identity()
            .approve_contract_team_action_with_signer(
                &owner(),
                &contract_id,
                &id(4),
                &FixedSignatureSigner,
            )
            .await
            .expect_err("the mock answers no broadcast");
        assert!(matches!(error, PlatformWalletError::Sdk(_)), "{error:?}");

        let broadcast = moderation_broadcast(
            wallet.identity(),
            contract_id,
            ContractUserModerationAction::ApproveTeamAction { action_id: id(4) },
        )
        .await;
        assert!(sdk.mock().request_count(&broadcast).await > 0);
    }

    /// A type the contract does not have is refused before anything is signed.
    #[tokio::test]
    async fn should_refuse_a_document_type_the_contract_does_not_have() {
        let contract = contract_of(
            owner(),
            Some(lists(ContractModerators::ContractOwner)),
            schemas(),
        );
        let contract_id = contract.id();
        let mut sdk = proposals_sdk();
        sdk.mock()
            .expect_fetch(contract_id, Some(contract))
            .await
            .expect("expects the contract read");
        let wallet = wallet_with_signing_keys_on(sdk, Arc::new(NoPlatformPersistence)).await;
        let error = wallet
            .identity()
            .restore_contract_document_with_signer(
                &owner(),
                &contract_id,
                "comment",
                &[0; 64],
                &FixedSignatureSigner,
            )
            .await
            .expect_err("no such type");
        assert!(
            matches!(error, PlatformWalletError::InvalidParameter(_)),
            "{error:?}"
        );
    }

    /// Platform's refusal of a ban of an identity already banned keeps its consensus error, so
    /// the FFI hands the host its code.
    #[test]
    fn should_keep_platforms_refusal_of_a_ban_of_a_banned_identity() {
        let cause = ConsensusError::StateError(StateError::ContractUserAlreadyBannedError(
            ContractUserAlreadyBannedError::new(id(0xEE), id(2)),
        ));
        let refusal =
            dash_sdk::Error::StateTransitionBroadcastError(StateTransitionBroadcastError {
                code: 41103,
                message: cause.to_string(),
                cause: Some(cause),
            });
        let error = PlatformWalletError::from(refusal);
        assert!(
            matches!(
                error.consensus_error(),
                Some(ConsensusError::StateError(
                    StateError::ContractUserAlreadyBannedError(_)
                ))
            ),
            "{error:?}"
        );
    }
}
