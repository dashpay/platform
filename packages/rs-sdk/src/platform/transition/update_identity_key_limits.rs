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
    /// at least one must be given. No identity revision is claimed, so the identity only needs
    /// to hold the signing key: a total computed from a stale key still executes if it raises
    /// the stored one, and is refused, paid, if it does not.
    ///
    /// If `signing_key_to_use` is not set, the first MASTER key, else the first CRITICAL
    /// authentication key without limits and without contract bounds, that the signer can sign
    /// with is used.
    ///
    /// This method resolves once the key is proved to hold the requested limits, with the key
    /// as it is stored after the update.
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
    /// Refused before anything is signed when the key has no budget or `amount` is zero: see
    /// [`raised_key_limits`].
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
    /// Refused before anything is signed when the key has no expiry or `expires_at` does not
    /// move it later: see [`raised_key_limits`].
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

        // The proof shows the rewritten key, not this exact transition (the nonce is not
        // stored), so it is waited for as affected state.
        let identity: PartialIdentity = state_transition
            .broadcast_and_wait_for_affected_state(sdk, settings)
            .await?;

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
        let (total_budget, expires_at) = raised_key_limits(self, key_id, Some(amount), None)?;
        self.update_key_limits(
            sdk,
            key_id,
            total_budget,
            expires_at,
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
        let (total_budget, expires_at) = raised_key_limits(self, key_id, None, Some(expires_at))?;
        self.update_key_limits(
            sdk,
            key_id,
            total_budget,
            expires_at,
            signing_key_to_use,
            signer,
            settings,
        )
        .await
    }
}

/// The absolute limits to send for adding `add_budget` credits to the budget of the key
/// `key_id` of `identity` and moving its expiry to `expires_at`, computed from the key as the
/// identity holds it. What consensus would refuse and charge for is refused here for free: a
/// limit the key does not have (40220), and a value that does not raise it (40221).
pub fn raised_key_limits(
    identity: &Identity,
    key_id: KeyID,
    add_budget: Option<Credits>,
    expires_at: Option<TimestampMillis>,
) -> Result<(Option<Credits>, Option<TimestampMillis>), Error> {
    let key = identity
        .public_keys()
        .get(&key_id)
        .ok_or_else(|| Error::Generic(format!("identity has no key {key_id}")))?;

    let total_budget = add_budget
        .map(|amount| {
            let current = key.total_budget().ok_or_else(|| {
                Error::Generic(format!(
                    "key {key_id} has no budget to top up: a budget can be raised, not added"
                ))
            })?;
            if amount == 0 {
                return Err(Error::Generic(format!(
                    "adding nothing to the budget of key {key_id} would be refused as not raising it"
                )));
            }
            current.checked_add(amount).ok_or_else(|| {
                Error::Generic(format!(
                    "adding {amount} credits to the budget of key {key_id} overflows"
                ))
            })
        })
        .transpose()?;

    let expires_at = expires_at
        .map(|expires_at| {
            let current = key.expires_at().ok_or_else(|| {
                Error::Generic(format!(
                    "key {key_id} has no expiry to extend: an expiry can be moved later, not added"
                ))
            })?;
            if expires_at <= current {
                return Err(Error::Generic(format!(
                    "an expiry of {expires_at} ms does not move the expiry of key {key_id} later than {current} ms"
                )));
            }
            Ok(expires_at)
        })
        .transpose()?;

    Ok((total_budget, expires_at))
}

/// The first MASTER key, else the first CRITICAL authentication key without limits and without
/// contract bounds (a bound key may only sign batches), that is enabled and that the signer can
/// sign with.
fn signing_key_for_key_limits_update<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    signer: &S,
) -> Result<KeyID, Error> {
    let candidates = |security_level: SecurityLevel| {
        identity.public_keys().values().find(|key| {
            key.purpose() == Purpose::AUTHENTICATION
                && key.security_level() == security_level
                && key.disabled_at().is_none()
                && key.contract_bounds().is_none()
                && !key.has_limits()
                && signer.can_sign_with(key)
        })
    };
    candidates(SecurityLevel::MASTER)
        .or_else(|| candidates(SecurityLevel::CRITICAL))
        .map(|key| key.id())
        .ok_or_else(|| {
            Error::Generic(
                "the signer holds no MASTER key, and no CRITICAL authentication key without limits and without contract bounds, of this identity"
                    .to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyType;
    use dpp::platform_value::{BinaryData, Identifier};
    use dpp::ProtocolError;
    use std::collections::BTreeMap;

    /// Holds the ids of the keys it can sign with; nothing is signed in these tests.
    #[derive(Debug)]
    struct KeyIdSigner(Vec<KeyID>);

    #[async_trait::async_trait]
    impl Signer<IdentityPublicKey> for KeyIdSigner {
        async fn sign(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<BinaryData, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        async fn sign_create_witness(
            &self,
            _key: &IdentityPublicKey,
            _data: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            Err(ProtocolError::Generic(
                "not signing in this test".to_string(),
            ))
        }

        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.0.contains(&key.id())
        }
    }

    fn authentication_key(
        id: KeyID,
        security_level: SecurityLevel,
        contract_bounds: Option<ContractBounds>,
    ) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at: None,
        })
    }

    /// MASTER key 0, a contract-bound CRITICAL key 1, an unbound CRITICAL key 2, and a CRITICAL
    /// key 3 with a budget.
    fn identity() -> Identity {
        let bounds = ContractBounds::SingleContract {
            id: Identifier::from([7; 32]),
        };
        let keys = [
            authentication_key(0, SecurityLevel::MASTER, None),
            authentication_key(1, SecurityLevel::CRITICAL, Some(bounds)),
            authentication_key(2, SecurityLevel::CRITICAL, None),
            authentication_key(3, SecurityLevel::CRITICAL, None).with_limits(Some(1_000), None),
        ];
        IdentityV0 {
            id: Identifier::from([1; 32]),
            public_keys: keys
                .into_iter()
                .map(|key| (key.id(), key))
                .collect::<BTreeMap<_, _>>(),
            balance: 0,
            revision: 0,
        }
        .into()
    }

    #[test]
    fn should_prefer_the_master_key_the_signer_holds() {
        let signing_key_id =
            signing_key_for_key_limits_update(&identity(), &KeyIdSigner(vec![0, 1, 2, 3]))
                .expect("expected a signing key");
        assert_eq!(signing_key_id, 0);
    }

    #[test]
    fn should_skip_a_contract_bound_critical_key_when_the_master_key_is_unavailable() {
        // Key 1 has the lower id but may only sign batches; key 2 is the usable one.
        let signing_key_id =
            signing_key_for_key_limits_update(&identity(), &KeyIdSigner(vec![1, 2, 3]))
                .expect("expected a signing key");
        assert_eq!(signing_key_id, 2);
    }

    #[test]
    fn should_refuse_a_top_up_or_extension_consensus_would_charge_for() {
        let identity = identity();
        assert!(matches!(
            raised_key_limits(&identity, 3, Some(10), None),
            Ok((Some(1_010), None))
        ));
        for (add_budget, expires_at) in [
            (Some(0), None),
            (Some(u64::MAX), None),
            (None, Some(1)),
            (Some(10), Some(1)),
        ] {
            let result = raised_key_limits(&identity, 3, add_budget, expires_at);
            assert!(
                matches!(result, Err(Error::Generic(_))),
                "{add_budget:?} {expires_at:?} must be refused locally: {result:?}"
            );
        }
        // Key 2 has no budget to raise
        assert!(matches!(
            raised_key_limits(&identity, 2, Some(10), None),
            Err(Error::Generic(_))
        ));
    }

    #[test]
    fn should_refuse_when_the_signer_only_holds_bound_or_limited_keys() {
        let result = signing_key_for_key_limits_update(&identity(), &KeyIdSigner(vec![1, 3]));
        assert!(
            matches!(result, Err(Error::Generic(_))),
            "a bound key and a limited key are no use here: {result:?}"
        );
    }
}
