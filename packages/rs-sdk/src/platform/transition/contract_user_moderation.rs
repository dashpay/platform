//! Ban, unban, suspend and unsuspend identities on a moderated data contract (protocol
//! version 14).
//!
//! A contract whose config declares moderation keeps a banlist and/or a suspension list. The
//! contract owner, or a moderator the config names, edits them with a
//! [`ContractUserModerationTransition`] signed by a CRITICAL authentication key. A banned or
//! suspended identity cannot act on the contract at the document level.
//!
//! ```ignore
//! let status = moderator_identity
//!     .ban_contract_user(&sdk, contract_id, user_id, None, signer, None)
//!     .await?;
//! ```

use dpp::data_contract::config::moderation::ContractModerationListStatus;
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

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};

use super::waitable::Waitable;

/// The target identity's entry on the list a moderation edited, as the proof of the moderation
/// shows it. The proof holds that one entry: it says nothing about the contract's other list,
/// so an identity shown as no longer suspended may still be banned. Fetch
/// `ContractModerationListStatuses` over every list the contract keeps for the whole picture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModeratedUserStatus {
    /// The moderated contract
    pub contract_id: Identifier,
    /// The moderated identity
    pub identity_id: Identifier,
    /// Its status on the edited list after the moderation
    pub status: ContractModerationListStatus,
}

impl TryFrom<StateTransitionProofResult> for ModeratedUserStatus {
    type Error = Error;

    fn try_from(value: StateTransitionProofResult) -> Result<Self, Self::Error> {
        match value {
            StateTransitionProofResult::VerifiedContractModerationListStatus(
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

    /// Puts `identity_id` on the banlist of `contract_id`.
    async fn ban_contract_user<S: Signer<IdentityPublicKey> + Send>(
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
            ContractUserModerationAction::Ban { identity_id },
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
    /// replacing a suspension it already carries.
    #[allow(clippy::too_many_arguments)]
    async fn suspend_contract_user<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        contract_id: Identifier,
        identity_id: Identifier,
        until: TimestampMillis,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<ModeratedUserStatus, Error> {
        self.moderate_contract_user(
            sdk,
            contract_id,
            ContractUserModerationAction::Suspend { identity_id, until },
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
        let signing_key_id = match signing_key_to_use {
            Some(key) => key.id(),
            None => signing_key_for_moderation(self, &signer)?,
        };
        let identity_contract_nonce = sdk
            .get_identity_contract_nonce(self.id(), contract_id, true, settings)
            .await?;
        let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
        let state_transition = ContractUserModerationTransition::try_from_identity_with_signer(
            self,
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
}

/// The first enabled CRITICAL authentication key without contract bounds (a bound key may
/// only sign batches) that the signer can sign with.
fn signing_key_for_moderation<S: Signer<IdentityPublicKey>>(
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
