//! Raise the limits of one of an identity's authentication keys (protocol version 14).
//!
//! A key registered with a total budget or an expiry cannot be edited by an identity update.
//! [`UpdateIdentityKeyLimits`] raises the budget (the key's `total_budget` and what is left of it
//! grow by the same amount) or moves the expiry later. An update only ever loosens limits.
//!
//! ```ignore
//! let key = identity
//!     .top_up_key_budget(&sdk, key_id, dash_to_credits!(0.5), None, signer, None)
//!     .await?;
//! ```

use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::signer::Signer;
use dpp::identity::{
    Identity, IdentityPublicKey, KeyID, PartialIdentity, Purpose, SecurityLevel, TimestampMillis,
};
use dpp::state_transition::identity_key_limits_update_transition::methods::IdentityKeyLimitsUpdateTransitionMethodsV0;
use dpp::state_transition::identity_key_limits_update_transition::IdentityKeyLimitsUpdateTransition;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};

use super::waitable::Waitable;

#[async_trait::async_trait]
pub trait UpdateIdentityKeyLimits: Waitable {
    /// Raises the limits of the key `key_id` of this identity: `total_budget` is the new total
    /// (greater than the current one), `expires_at` the new expiry (later than the current one);
    /// at least one must be given. The identity must be as it currently is in state, since the
    /// transition claims its next revision.
    ///
    /// If `signing_key_to_use` is not set, the first MASTER key, else the first CRITICAL
    /// authentication key without limits, that the signer can sign with is used.
    ///
    /// This method resolves once the state transition is proved executed, with the key as it is
    /// stored after the update.
    #[allow(clippy::too_many_arguments)]
    async fn update_key_limits<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error>;

    /// Adds `amount` credits to the total budget of the key `key_id`, and to what is left of it.
    async fn top_up_key_budget<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        amount: Credits,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error>;

    /// Moves the expiry of the key `key_id` to `expires_at`, later than its current expiry.
    async fn extend_key_expiry<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        expires_at: TimestampMillis,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error>;
}

#[async_trait::async_trait]
impl UpdateIdentityKeyLimits for Identity {
    #[allow(clippy::too_many_arguments)]
    async fn update_key_limits<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        total_budget: Option<Credits>,
        expires_at: Option<TimestampMillis>,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error> {
        let signing_key_id = match signing_key_to_use {
            Some(key) => key.id(),
            None => signing_key_for_key_limits_update(self, &signer)?,
        };
        let new_identity_nonce = sdk.get_identity_nonce(self.id(), true, settings).await?;
        let user_fee_increase = settings.and_then(|settings| settings.user_fee_increase);
        let state_transition = IdentityKeyLimitsUpdateTransition::try_from_identity_with_signer(
            self,
            &signing_key_id,
            key_id,
            total_budget,
            expires_at,
            new_identity_nonce,
            user_fee_increase.unwrap_or_default(),
            &signer,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        // The proof binds the rewritten key and the revision, so the strict wait applies.
        let identity: PartialIdentity = state_transition.broadcast_and_wait(sdk, settings).await?;

        identity
            .loaded_public_keys
            .get(&key_id)
            .cloned()
            .ok_or_else(|| {
                Error::Generic(format!(
                    "expected key {key_id} in the proved identity after the key limits update"
                ))
            })
    }

    async fn top_up_key_budget<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        amount: Credits,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error> {
        let current = self
            .public_keys()
            .get(&key_id)
            .ok_or_else(|| Error::Generic(format!("identity has no key {key_id}")))?
            .total_budget()
            .ok_or_else(|| {
                Error::Generic(format!(
                    "key {key_id} has no budget to top up: a budget can be raised, not added"
                ))
            })?;
        let total_budget = current.checked_add(amount).ok_or_else(|| {
            Error::Generic(format!(
                "adding {amount} credits to the budget of key {key_id} overflows"
            ))
        })?;
        self.update_key_limits(
            sdk,
            key_id,
            Some(total_budget),
            None,
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }

    async fn extend_key_expiry<S: Signer<IdentityPublicKey> + Send>(
        &self,
        sdk: &Sdk,
        key_id: KeyID,
        expires_at: TimestampMillis,
        signing_key_to_use: Option<&IdentityPublicKey>,
        signer: S,
        settings: Option<PutSettings>,
    ) -> Result<IdentityPublicKey, Error> {
        self.update_key_limits(
            sdk,
            key_id,
            None,
            Some(expires_at),
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }
}

/// The first MASTER key, else the first CRITICAL authentication key without limits, that is
/// enabled and that the signer can sign with.
fn signing_key_for_key_limits_update<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    signer: &S,
) -> Result<KeyID, Error> {
    let candidates = |security_level: SecurityLevel| {
        identity.public_keys().values().find(|key| {
            key.purpose() == Purpose::AUTHENTICATION
                && key.security_level() == security_level
                && key.disabled_at().is_none()
                && !key.has_limits()
                && signer.can_sign_with(key)
        })
    };
    candidates(SecurityLevel::MASTER)
        .or_else(|| candidates(SecurityLevel::CRITICAL))
        .map(|key| key.id())
        .ok_or_else(|| {
            Error::Generic(
                "the signer holds no MASTER key, and no CRITICAL authentication key without limits, of this identity"
                    .to_string(),
            )
        })
}
