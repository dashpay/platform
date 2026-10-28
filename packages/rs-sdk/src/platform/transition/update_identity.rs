//! Add and disable an identity's public keys.
//!
//! [`UpdateIdentity::update_identity_keys`] refuses, before anything is signed, an update that
//! disables the last valid authentication key of a security level. DIP-11: "Keys can only be
//! disabled if another valid key is enabled in the same security level. This can happen
//! atomically in the same state transition." Consensus enforces this for MASTER keys only, so
//! for every other level the client is the only enforcer. [`check_keys_can_be_disabled`] is the
//! same rule as a query, for an interface that wants to offer only the keys it may disable.
//!
//! ```ignore
//! let result = identity
//!     .update_identity_keys(&sdk, master_key_id, vec![new_key], vec![old_key_id], &signer, None)
//!     .await?;
//! ```

use dpp::identity::accessors::{IdentityGettersV0, IdentitySettersV0};
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel, TimestampMillis};
use dpp::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use dpp::state_transition::identity_update_transition::IdentityUpdateTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;

use crate::platform::transition::broadcast::BroadcastStateTransition;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::transition::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};

/// An identity update would disable the last valid authentication key of a security level.
///
/// A security level is a minimum: a key of a stronger level stands in for it, except MASTER,
/// which signs identity updates only. So a CRITICAL key keeps a HIGH key disableable, a MASTER
/// key does not, and a MASTER key needs another MASTER key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "cannot disable authentication key {key_id}: no other enabled, unexpired authentication key \
     at security level {security_level} or one standing in for it would remain"
)]
pub struct LastAuthenticationKeyError {
    /// The key the update would disable.
    pub key_id: KeyID,
    /// The security level that would be left without a valid authentication key.
    pub security_level: SecurityLevel,
}

/// Whether an identity update that adds `add_public_keys` and disables `disable_public_keys`
/// keeps a valid authentication key at the security level of every authentication key it
/// disables, at the time `now_ms` (Unix milliseconds).
///
/// A valid key is enabled and not expired at `now_ms`. The keys the identity keeps count, and
/// so do the keys the same update adds. Ids the identity does not hold are not judged here:
/// Platform refuses them.
pub fn check_keys_can_be_disabled(
    identity: &Identity,
    add_public_keys: &[IdentityPublicKey],
    disable_public_keys: &[KeyID],
    now_ms: TimestampMillis,
) -> Result<(), LastAuthenticationKeyError> {
    let remaining_valid_keys = || {
        identity
            .public_keys()
            .values()
            .filter(|key| !disable_public_keys.contains(&key.id()))
            .chain(add_public_keys)
            .filter(|key| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.disabled_at().is_none()
                    && !key.is_expired_at(now_ms)
            })
    };
    for key_id in disable_public_keys {
        let Some(key) = identity.public_keys().get(key_id) else {
            continue;
        };
        if key.purpose() != Purpose::AUTHENTICATION {
            continue;
        }
        let security_level = key.security_level();
        if !remaining_valid_keys()
            .any(|remaining| stands_in_for(remaining.security_level(), security_level))
        {
            return Err(LastAuthenticationKeyError {
                key_id: *key_id,
                security_level,
            });
        }
    }
    Ok(())
}

/// Whether a key at `candidate` satisfies a requirement for `required`: the same level, or a
/// stronger one other than MASTER, which signs nothing but identity updates.
fn stands_in_for(candidate: SecurityLevel, required: SecurityLevel) -> bool {
    candidate == required
        || (candidate != SecurityLevel::MASTER && candidate.stronger_security_than(required))
}

/// The current wall-clock time in Unix milliseconds, on native targets and in the browser.
fn now_ms() -> Result<TimestampMillis, Error> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as TimestampMillis)
            .map_err(|e| Error::Generic(format!("system time is before the Unix epoch: {e}")))
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(js_sys::Date::now() as TimestampMillis)
    }
}

#[async_trait::async_trait]
pub trait UpdateIdentity {
    /// Adds `add_public_keys` to this identity and disables the keys `disable_public_keys`,
    /// signing with its MASTER key `master_key_id`, and waits for the proved result.
    ///
    /// `self` is the identity as Platform holds it; the update claims the next revision.
    /// Refused before anything is signed, with [`Error::LastAuthenticationKey`], when it would
    /// disable the last valid authentication key of a security level: see
    /// [`check_keys_can_be_disabled`].
    ///
    /// Keys in `add_public_keys` carry their final ids. Their private keys must be reachable
    /// through `signer`, which signs each unique-type key's proof of possession as well as the
    /// transition.
    async fn update_identity_keys<S: Signer<IdentityPublicKey>>(
        &self,
        sdk: &Sdk,
        master_key_id: KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl UpdateIdentity for Identity {
    async fn update_identity_keys<S: Signer<IdentityPublicKey>>(
        &self,
        sdk: &Sdk,
        master_key_id: KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error> {
        check_keys_can_be_disabled(self, &add_public_keys, &disable_public_keys, now_ms()?)?;

        let mut identity = self.clone();
        let revision = identity
            .revision()
            .checked_add(1)
            .ok_or_else(|| Error::Generic("identity revision overflows".to_string()))?;
        identity.set_revision(revision);

        let identity_nonce = sdk
            .get_identity_nonce(identity.id(), true, settings)
            .await?;
        let user_fee_increase = settings
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();
        let state_transition = IdentityUpdateTransition::try_from_identity_with_signer(
            &identity,
            &master_key_id,
            add_public_keys,
            disable_public_keys,
            identity_nonce,
            user_fee_increase,
            signer,
            sdk.version(),
            None,
        )
        .await?;
        ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

        state_transition.broadcast_and_wait(sdk, settings).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyType;
    use dpp::platform_value::{BinaryData, Identifier};
    use dpp::ProtocolError;
    use std::collections::BTreeMap;

    const NOW_MS: TimestampMillis = 1_800_000_000_000;

    fn key(id: KeyID, purpose: Purpose, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at: None,
        })
    }

    fn authentication_key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        key(id, Purpose::AUTHENTICATION, security_level)
    }

    fn identity(keys: Vec<IdentityPublicKey>) -> Identity {
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

    /// MASTER 0, CRITICAL 1, HIGH 2, MEDIUM 3: one authentication key per level.
    fn one_key_per_level() -> Identity {
        identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
            authentication_key(2, SecurityLevel::HIGH),
            authentication_key(3, SecurityLevel::MEDIUM),
        ])
    }

    #[test]
    fn should_refuse_disabling_the_only_critical_key() {
        let result = check_keys_can_be_disabled(&one_key_per_level(), &[], &[1], NOW_MS);
        assert_eq!(
            result,
            Err(LastAuthenticationKeyError {
                key_id: 1,
                security_level: SecurityLevel::CRITICAL,
            })
        );
    }

    #[test]
    fn should_refuse_disabling_the_only_master_key_without_adding_one() {
        let result = check_keys_can_be_disabled(&one_key_per_level(), &[], &[0], NOW_MS);
        assert_eq!(
            result,
            Err(LastAuthenticationKeyError {
                key_id: 0,
                security_level: SecurityLevel::MASTER,
            })
        );
    }

    #[test]
    fn should_let_a_stronger_non_master_key_stand_in_for_a_weaker_level() {
        // CRITICAL 1 still meets HIGH and MEDIUM requirements.
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[], &[2, 3], NOW_MS),
            Ok(())
        );
    }

    #[test]
    fn should_not_let_a_master_key_stand_in_for_critical() {
        // MASTER signs identity updates only, so it cannot replace the CRITICAL key.
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
        ]);
        assert!(check_keys_can_be_disabled(&identity, &[], &[1], NOW_MS).is_err());
    }

    #[test]
    fn should_allow_disabling_a_key_replaced_in_the_same_update() {
        let replacement = authentication_key(4, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[replacement], &[1], NOW_MS),
            Ok(())
        );
        let master_replacement = authentication_key(4, SecurityLevel::MASTER);
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[master_replacement], &[0], NOW_MS),
            Ok(())
        );
    }

    #[test]
    fn should_allow_disabling_one_of_two_keys_of_a_level_but_not_both() {
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
            authentication_key(2, SecurityLevel::CRITICAL),
        ]);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1], NOW_MS),
            Ok(())
        );
        assert!(check_keys_can_be_disabled(&identity, &[], &[1, 2], NOW_MS).is_err());
    }

    #[test]
    fn should_not_count_disabled_or_expired_keys_as_remaining() {
        let mut disabled = authentication_key(2, SecurityLevel::CRITICAL);
        disabled.set_disabled_at(NOW_MS - 1);
        let expired =
            authentication_key(3, SecurityLevel::CRITICAL).with_limits(None, Some(NOW_MS));
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
            disabled,
            expired,
        ]);
        assert!(check_keys_can_be_disabled(&identity, &[], &[1], NOW_MS).is_err());

        let expired_replacement =
            authentication_key(4, SecurityLevel::CRITICAL).with_limits(None, Some(NOW_MS));
        assert!(
            check_keys_can_be_disabled(&identity, &[expired_replacement], &[1], NOW_MS).is_err()
        );

        let unexpired =
            authentication_key(5, SecurityLevel::CRITICAL).with_limits(None, Some(NOW_MS + 1));
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[unexpired], &[1], NOW_MS),
            Ok(())
        );
    }

    #[test]
    fn should_not_judge_keys_of_other_purposes_or_unknown_ids() {
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            key(1, Purpose::TRANSFER, SecurityLevel::CRITICAL),
            key(2, Purpose::ENCRYPTION, SecurityLevel::MEDIUM),
        ]);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1, 2, 9], NOW_MS),
            Ok(())
        );
        // A non-authentication key does not keep an authentication level valid.
        let identity_with_critical = {
            let mut identity = identity;
            identity.add_public_key(authentication_key(3, SecurityLevel::CRITICAL));
            identity
        };
        assert!(check_keys_can_be_disabled(&identity_with_critical, &[], &[3], NOW_MS).is_err());
    }

    /// The refusal comes before the nonce is fetched: the mock answers no request, so a
    /// refusal that reached the network would be a different error.
    #[cfg(feature = "mocks")]
    #[tokio::test]
    async fn should_refuse_the_update_before_sending_anything() {
        #[derive(Debug)]
        struct NoSigner;

        #[async_trait::async_trait]
        impl Signer<IdentityPublicKey> for NoSigner {
            async fn sign(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<BinaryData, ProtocolError> {
                Err(ProtocolError::Generic("not signing".to_string()))
            }

            async fn sign_create_witness(
                &self,
                _key: &IdentityPublicKey,
                _data: &[u8],
            ) -> Result<AddressWitness, ProtocolError> {
                Err(ProtocolError::Generic("not signing".to_string()))
            }

            fn can_sign_with(&self, _key: &IdentityPublicKey) -> bool {
                false
            }
        }

        let sdk = Sdk::new_mock();
        let result = one_key_per_level()
            .update_identity_keys(&sdk, 0, vec![], vec![1], &NoSigner, None)
            .await;
        assert!(
            matches!(
                result,
                Err(Error::LastAuthenticationKey(LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                }))
            ),
            "{result:?}"
        );
    }
}
