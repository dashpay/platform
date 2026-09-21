//! Ban, unban, suspend, unsuspend, warn and clear the warnings of identities on a moderated
//! data contract, and delete documents of the document types that let moderators do so
//! (protocol version 14).
//!
//! A contract whose config declares moderation keeps a banlist, a suspension list and/or a
//! warning list. The contract owner, or a moderator the config names, edits them with a
//! [`ContractUserModerationTransition`] signed by a CRITICAL authentication key. A banned or
//! suspended identity cannot act on the contract at the document level; a warned one can, the
//! warnings being a record it and everyone else can read.
//!
//! The same transition deletes one document of a document type that sets
//! `canBeDeletedByModerators`, whoever owns it, and leaves a record of the deletion under the
//! contract; and it restores such a document, as it was, within a week of its deletion.
//!
//! ```ignore
//! let reason = ContractModerationReason::from_text("spam");
//! let status = moderator_identity
//!     .ban_contract_user(&sdk, contract_id, user_id, reason.clone(), None, &signer, None)
//!     .await?;
//! let removal = moderator_identity
//!     .delete_contract_document(
//!         &sdk, contract_id, "post".to_string(), document_id, reason, None, &signer, None,
//!     )
//!     .await?;
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
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel, TimestampMillis};
use dpp::platform_value::Identifier;
use dpp::state_transition::contract_user_moderation_transition::methods::ContractUserModerationTransitionMethodsV0;
use dpp::state_transition::contract_user_moderation_transition::{
    ContractUserModerationAction, ContractUserModerationTransition,
};
use dpp::state_transition::proof_result::StateTransitionProofResult;
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
    /// `canBeDeletedByModerators`. Resolves with the record the deletion left under the
    /// contract: whose the document was, who removed it, why and when.
    ///
    /// The document's owner gets no storage refund, and nothing ever deletes the record. The
    /// record holds a hash of the document as it was: keep the document (or its bytes) if the
    /// deletion may have to be undone, since `restore_contract_document` needs it.
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
    ) -> Result<ContractDocumentRemoval, Error>;

    /// Brings back `document`, of `document_type_name` on `contract`, that a moderator deleted:
    /// the document as it was when it was deleted, which must hash to what its removal record
    /// holds, within `SystemLimits::contract_document_restore_window_ms` (a week) of the
    /// deletion. Any current moderator or the contract owner may restore, whoever deleted.
    /// The document goes back through an ordinary insert, so a unique index value another
    /// document took meanwhile refuses it. Resolves with the record, now marked restored.
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
        // A document deletion or restore is proved by its removal record, not by a status.
        // Refused before the nonce is taken: sent from here it would execute, be paid for, and
        // then fail to read its own result.
        if matches!(
            action,
            ContractUserModerationAction::DeleteDocument { .. }
                | ContractUserModerationAction::RestoreDocument { .. }
        ) {
            return Err(Error::Generic(
                "a document deletion or restore names no identity to report a status of: send \
                 it with `delete_contract_document` or `restore_contract_document`, which \
                 return the removal record"
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
    ) -> Result<ContractDocumentRemoval, Error> {
        let VerifiedDocumentRemoval(removal) = broadcast_moderation(
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
    // fetched and registered with it. A document deletion is proved by its own removal record
    // and needs no contract; a restore's verifier decodes the document under the contract's
    // document type, so it needs the contract too.
    if matches!(
        action,
        ContractUserModerationAction::Ban { .. }
            | ContractUserModerationAction::RestoreDocument { .. }
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
