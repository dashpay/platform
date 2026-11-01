//! The DIP-11 rule on disabling an identity's authentication keys.
//!
//! DIP-11: "Keys can only be disabled if another valid key is enabled in the same security
//! level. This can happen atomically in the same state transition."
//!
//! **This is signing-side policy, not consensus.** Drive enforces it for MASTER keys only and
//! accepts an identity update that disables the last key of any other level, so this module
//! never produces a `ConsensusError` and nothing on a block-execution path calls it. Clients
//! enforce it: `IdentityUpdateTransition::try_from_identity_with_signer` (the
//! `state-transition-signing` builder) refuses such an update before signing anything, and an
//! interface with a clock can call [`check_keys_can_be_disabled`] to offer only the keys it may
//! disable.

use crate::identity::accessors::IdentityGettersV0;
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use crate::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use crate::identity::{Identity, IdentityPublicKey, KeyID, Purpose, SecurityLevel};
use crate::prelude::TimestampMillis;

/// An identity update would disable the last valid authentication key of a security level,
/// which DIP-11 forbids. Signing-side policy, not a consensus error: Platform would accept the
/// update for every level but MASTER.
///
/// A security level is a minimum: a key of a stronger level stands in for it, except MASTER,
/// which signs identity updates and key-limits updates only. So a CRITICAL key keeps a HIGH key
/// disableable, a MASTER key does not, and a MASTER key needs another MASTER key.
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
/// disables.
///
/// A valid key is an enabled authentication key that has not expired and whose key type can sign
/// state transitions ([`KeyType::can_sign_state_transitions`](crate::identity::KeyType::can_sign_state_transitions)): `ECDSA_HASH160` keys, which
/// identities created by older Dash Evo Tool versions carry, count; `BIP13_SCRIPT_HASH` and
/// `EDDSA_25519_HASH160` keys do not. The keys the identity keeps count, and so do the keys the
/// same update adds. A key stands in for the disabled one only where it can sign everything the
/// disabled one could:
///
/// - **Budget.** A key with a `total_budget` never counts: what is left of the budget lives in
///   Drive, and an exhausted key can no longer sign.
/// - **Contract bounds.** An unbounded key is replaced only by an unbounded key; a bound key by
///   an unbounded key or one with identical bounds. Platform refuses a bound key outside its
///   bounds.
///
/// `now_ms` is the caller's current time in Unix milliseconds. dpp reads no clock, so `None`
/// judges conservatively: a key with any `expires_at` does not count as remaining. With
/// `Some(now)`, a key counts until `now` reaches its `expires_at`. Each of these is stricter
/// than DIP-11 and therefore still compliant.
///
/// Ids the identity does not hold, and keys of other purposes, are not judged here: Platform
/// refuses unknown ids, and DIP-11 speaks of authentication keys.
pub fn check_keys_can_be_disabled(
    identity: &Identity,
    add_public_keys: &[IdentityPublicKey],
    disable_public_keys: &[KeyID],
    now_ms: Option<TimestampMillis>,
) -> Result<(), LastAuthenticationKeyError> {
    let remaining_valid_keys = || {
        identity
            .public_keys()
            .values()
            .filter(|key| !disable_public_keys.contains(&key.id()))
            .chain(add_public_keys)
            .filter(|key| {
                key.purpose() == Purpose::AUTHENTICATION
                    && key.key_type().can_sign_state_transitions()
                    && key.disabled_at().is_none()
                    && key.total_budget().is_none()
                    && is_unexpired(key, now_ms)
            })
    };
    for key_id in disable_public_keys {
        let Some(key) = identity.public_keys().get(key_id) else {
            continue;
        };
        if key.purpose() != Purpose::AUTHENTICATION {
            continue;
        }
        if !remaining_valid_keys().any(|remaining| stands_in_for(remaining, key)) {
            return Err(LastAuthenticationKeyError {
                key_id: *key_id,
                security_level: key.security_level(),
            });
        }
    }
    Ok(())
}

/// Whether `key` is unexpired at `now_ms`; without a time, only a key with no expiry is.
fn is_unexpired(key: &IdentityPublicKey, now_ms: Option<TimestampMillis>) -> bool {
    match now_ms {
        Some(now_ms) => !key.is_expired_at(now_ms),
        None => key.expires_at().is_none(),
    }
}

/// Whether `candidate` can sign whatever `disabled` could, as far as level and contract bounds
/// go: the same level, or a stronger one other than MASTER, which signs identity updates and
/// key-limits updates only; and no bounds, or the same bounds as `disabled`. The single place
/// these comparisons of the rule are made.
fn stands_in_for(candidate: &IdentityPublicKey, disabled: &IdentityPublicKey) -> bool {
    let (candidate_level, required_level) = (candidate.security_level(), disabled.security_level());
    let level_suffices = candidate_level == required_level
        || (candidate_level != SecurityLevel::MASTER
            && candidate_level.stronger_security_than(required_level));
    let bounds_suffice = match candidate.contract_bounds() {
        None => true,
        bounds => bounds == disabled.contract_bounds(),
    };
    level_suffices && bounds_suffice
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use crate::identity::identity_public_key::contract_bounds::ContractBounds;
    use crate::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use crate::identity::v0::IdentityV0;
    use crate::identity::KeyType;
    use platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    const NOW_MS: TimestampMillis = 1_800_000_000_000;

    pub(crate) fn key(
        id: KeyID,
        purpose: Purpose,
        security_level: SecurityLevel,
        key_type: KeyType,
    ) -> IdentityPublicKey {
        let data = match key_type {
            KeyType::ECDSA_HASH160 => vec![id as u8; 20],
            _ => vec![id as u8; 33],
        };
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose,
            security_level,
            contract_bounds: None,
            key_type,
            read_only: false,
            data: BinaryData::new(data),
            disabled_at: None,
        })
    }

    pub(crate) fn authentication_key(
        id: KeyID,
        security_level: SecurityLevel,
    ) -> IdentityPublicKey {
        key(
            id,
            Purpose::AUTHENTICATION,
            security_level,
            KeyType::ECDSA_SECP256K1,
        )
    }

    pub(crate) fn identity(keys: Vec<IdentityPublicKey>) -> Identity {
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
    pub(crate) fn one_key_per_level() -> Identity {
        identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
            authentication_key(2, SecurityLevel::HIGH),
            authentication_key(3, SecurityLevel::MEDIUM),
        ])
    }

    #[test]
    fn should_refuse_disabling_the_only_critical_key() {
        let result = check_keys_can_be_disabled(&one_key_per_level(), &[], &[1], Some(NOW_MS));
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
        let result = check_keys_can_be_disabled(&one_key_per_level(), &[], &[0], Some(NOW_MS));
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
            check_keys_can_be_disabled(&one_key_per_level(), &[], &[2, 3], Some(NOW_MS)),
            Ok(())
        );
    }

    #[test]
    fn should_not_let_a_master_key_stand_in_for_critical() {
        // MASTER signs identity and key-limits updates only, so it cannot replace the CRITICAL key.
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            authentication_key(1, SecurityLevel::CRITICAL),
        ]);
        assert!(check_keys_can_be_disabled(&identity, &[], &[1], Some(NOW_MS)).is_err());
    }

    #[test]
    fn should_not_count_a_key_type_that_cannot_sign_as_remaining() {
        for key_type in [KeyType::BIP13_SCRIPT_HASH, KeyType::EDDSA_25519_HASH160] {
            let unsignable = key(
                4,
                Purpose::AUTHENTICATION,
                SecurityLevel::CRITICAL,
                key_type,
            );
            let refused = Err(LastAuthenticationKeyError {
                key_id: 1,
                security_level: SecurityLevel::CRITICAL,
            });

            let held = identity(vec![
                authentication_key(0, SecurityLevel::MASTER),
                authentication_key(1, SecurityLevel::CRITICAL),
                unsignable.clone(),
            ]);
            assert_eq!(
                check_keys_can_be_disabled(&held, &[], &[1], Some(NOW_MS)),
                refused,
                "a held {key_type:?} key must not stand in"
            );
            assert_eq!(
                check_keys_can_be_disabled(&one_key_per_level(), &[unsignable], &[1], Some(NOW_MS)),
                refused,
                "an added {key_type:?} key must not stand in"
            );
        }
    }

    #[test]
    fn should_allow_disabling_a_key_replaced_in_the_same_update() {
        let replacement = authentication_key(4, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[replacement], &[1], Some(NOW_MS)),
            Ok(())
        );
        let master_replacement = authentication_key(4, SecurityLevel::MASTER);
        assert_eq!(
            check_keys_can_be_disabled(
                &one_key_per_level(),
                &[master_replacement],
                &[0],
                Some(NOW_MS)
            ),
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
            check_keys_can_be_disabled(&identity, &[], &[1], Some(NOW_MS)),
            Ok(())
        );
        assert!(check_keys_can_be_disabled(&identity, &[], &[1, 2], Some(NOW_MS)).is_err());
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
        assert!(check_keys_can_be_disabled(&identity, &[], &[1], Some(NOW_MS)).is_err());

        let expired_replacement =
            authentication_key(4, SecurityLevel::CRITICAL).with_limits(None, Some(NOW_MS));
        assert!(
            check_keys_can_be_disabled(&identity, &[expired_replacement], &[1], Some(NOW_MS))
                .is_err()
        );

        let unexpired =
            authentication_key(5, SecurityLevel::CRITICAL).with_limits(None, Some(NOW_MS + 1));
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[unexpired], &[1], Some(NOW_MS)),
            Ok(())
        );
    }

    #[test]
    fn should_not_count_a_key_with_any_expiry_without_a_clock() {
        // Without a time the rule cannot tell an expiring key from an expired one, so it counts
        // neither: stricter than DIP-11, never looser.
        let far_future =
            authentication_key(4, SecurityLevel::CRITICAL).with_limits(None, Some(u64::MAX));
        let mut identity = one_key_per_level();
        identity.add_public_key(far_future.clone());
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1], None),
            Err(LastAuthenticationKeyError {
                key_id: 1,
                security_level: SecurityLevel::CRITICAL,
            })
        );
        assert!(
            check_keys_can_be_disabled(&one_key_per_level(), &[far_future], &[1], None).is_err()
        );
        // A key without limits counts the same either way.
        let replacement = authentication_key(5, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[replacement], &[1], None),
            Ok(())
        );
    }

    #[test]
    fn should_not_count_a_key_with_a_budget_as_remaining() {
        // What is left of a budget lives in Drive: an exhausted budgeted key can no longer
        // sign, and the rule cannot tell it from a fresh one.
        let budgeted = authentication_key(4, SecurityLevel::CRITICAL).with_limits(Some(1), None);
        let mut identity = one_key_per_level();
        identity.add_public_key(budgeted.clone());
        for now_ms in [Some(NOW_MS), None] {
            assert_eq!(
                check_keys_can_be_disabled(&identity, &[], &[1], now_ms),
                Err(LastAuthenticationKeyError {
                    key_id: 1,
                    security_level: SecurityLevel::CRITICAL,
                })
            );
            assert!(check_keys_can_be_disabled(
                &one_key_per_level(),
                std::slice::from_ref(&budgeted),
                &[1],
                now_ms
            )
            .is_err());
        }
        // An unbudgeted replacement added in the same update still counts.
        let replacement = authentication_key(5, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[replacement], &[1], Some(NOW_MS)),
            Ok(())
        );
    }

    fn bound_to(key: IdentityPublicKey, contract: u8) -> IdentityPublicKey {
        let IdentityPublicKey::V0(mut v0) = key else {
            panic!("test keys are V0");
        };
        v0.contract_bounds = Some(ContractBounds::SingleContract {
            id: Identifier::from([contract; 32]),
        });
        v0.into()
    }

    #[test]
    fn should_not_let_a_contract_bound_key_stand_in_for_an_unbounded_one() {
        // A bound key signs only inside its contract, so it cannot do the unbounded key's work.
        let bound = bound_to(authentication_key(4, SecurityLevel::CRITICAL), 7);
        let mut identity = one_key_per_level();
        identity.add_public_key(bound.clone());
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1], Some(NOW_MS)),
            Err(LastAuthenticationKeyError {
                key_id: 1,
                security_level: SecurityLevel::CRITICAL,
            })
        );
        assert!(
            check_keys_can_be_disabled(&one_key_per_level(), &[bound], &[1], Some(NOW_MS)).is_err()
        );
    }

    #[test]
    fn should_let_an_unbounded_or_identically_bound_key_stand_in_for_a_bound_one() {
        let bound = bound_to(authentication_key(4, SecurityLevel::CRITICAL), 7);
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            bound.clone(),
        ]);
        let unbounded = authentication_key(5, SecurityLevel::CRITICAL);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[unbounded], &[4], Some(NOW_MS)),
            Ok(())
        );
        let same_bounds = bound_to(authentication_key(5, SecurityLevel::CRITICAL), 7);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[same_bounds], &[4], Some(NOW_MS)),
            Ok(())
        );
        let other_bounds = bound_to(authentication_key(5, SecurityLevel::CRITICAL), 8);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[other_bounds], &[4], Some(NOW_MS)),
            Err(LastAuthenticationKeyError {
                key_id: 4,
                security_level: SecurityLevel::CRITICAL,
            })
        );
    }

    #[test]
    fn should_not_judge_keys_of_other_purposes_or_unknown_ids() {
        let identity = identity(vec![
            authentication_key(0, SecurityLevel::MASTER),
            key(
                1,
                Purpose::TRANSFER,
                SecurityLevel::CRITICAL,
                KeyType::ECDSA_SECP256K1,
            ),
            key(
                2,
                Purpose::ENCRYPTION,
                SecurityLevel::MEDIUM,
                KeyType::ECDSA_SECP256K1,
            ),
        ]);
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1, 2, 9], Some(NOW_MS)),
            Ok(())
        );
        // A non-authentication key does not keep an authentication level valid.
        let identity_with_critical = {
            let mut identity = identity;
            identity.add_public_key(authentication_key(3, SecurityLevel::CRITICAL));
            identity
        };
        assert!(
            check_keys_can_be_disabled(&identity_with_critical, &[], &[3], Some(NOW_MS)).is_err()
        );
    }

    #[test]
    fn should_count_hash160_keys_as_valid() {
        // Identities created by older Dash Evo Tool versions carry ECDSA_HASH160 keys.
        let hash160 = key(
            4,
            Purpose::AUTHENTICATION,
            SecurityLevel::CRITICAL,
            KeyType::ECDSA_HASH160,
        );
        let mut identity = one_key_per_level();
        identity.add_public_key(hash160.clone());
        assert_eq!(
            check_keys_can_be_disabled(&identity, &[], &[1], None),
            Ok(())
        );
        assert_eq!(
            check_keys_can_be_disabled(&one_key_per_level(), &[hash160], &[1], None),
            Ok(())
        );
    }
}
