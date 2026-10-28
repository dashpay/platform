//! The DIP-11 rule on disabling an identity's authentication keys.
//!
//! DIP-11: "Keys can only be disabled if another valid key is enabled in the same security
//! level. This can happen atomically in the same state transition." Consensus enforces this for
//! MASTER keys only, so for every other level the client is the only enforcer. Every client path
//! that builds an identity update calls [`ensure_keys_can_be_disabled`] before building or
//! signing anything; [`check_keys_can_be_disabled`] is the same rule at a given time, for an
//! interface that wants to offer only the keys it may disable.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel, TimestampMillis};

use crate::Error;

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
/// A valid key is enabled and not expired at `now_ms`, whatever its key type: `ECDSA_HASH160`
/// keys, which identities created by older Dash Evo Tool versions carry, count. The keys the
/// identity keeps count, and so do the keys the same update adds. Ids the identity does not hold
/// are not judged here: Platform refuses them.
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

/// [`check_keys_can_be_disabled`] at the current wall-clock time, as
/// [`Error::LastAuthenticationKey`]. Call it before building or signing an identity update.
pub fn ensure_keys_can_be_disabled(
    identity: &Identity,
    add_public_keys: &[IdentityPublicKey],
    disable_public_keys: &[KeyID],
) -> Result<(), Error> {
    check_keys_can_be_disabled(identity, add_public_keys, disable_public_keys, now_ms()?)?;
    Ok(())
}

/// Whether a key at `candidate` satisfies a requirement for `required`: the same level, or a
/// stronger one other than MASTER, which signs nothing but identity updates. The single place
/// the level comparison of this rule is made.
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

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyType;
    use dpp::platform_value::{BinaryData, Identifier};
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

    #[test]
    fn should_count_hash160_keys_as_valid() {
        // Identities created by older Dash Evo Tool versions carry ECDSA_HASH160 keys.
        let hash160 = IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 4,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::CRITICAL,
            contract_bounds: None,
            key_type: KeyType::ECDSA_HASH160,
            read_only: false,
            data: BinaryData::new(vec![7; 20]),
            disabled_at: None,
        });
        let mut identity = one_key_per_level();
        identity.add_public_key(hash160.clone());
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1], NOW_MS),
            Ok(())
        );
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[hash160], &[1], NOW_MS),
            Ok(())
        );
    }

    #[test]
    fn should_report_the_refusal_as_a_typed_sdk_error() {
        let result = ensure_keys_can_be_disabled(&one_key_per_level(), &[], &[1]);
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
        assert!(ensure_keys_can_be_disabled(&one_key_per_level(), &[], &[2]).is_ok());
    }
}
