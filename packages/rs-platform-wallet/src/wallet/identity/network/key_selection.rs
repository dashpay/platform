//! Choosing an authentication key to sign with, now that a key may carry limits.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};

use crate::util::now_ms;

/// The first AUTHENTICATION key of `identity` at one of `security_levels`, of one of
/// `key_types`, that can sign now.
///
/// A key without limits is preferred: a key with a budget or an expiry is an application
/// key, taken only when no unlimited key qualifies. A disabled key is skipped, and so is a
/// key whose expiry has passed the wall clock (the block time trails it by seconds at most).
/// What is left of a budget is not known offline; a spent key is refused by Platform.
pub(crate) fn usable_authentication_key<'a>(
    identity: &'a Identity,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
) -> Option<&'a IdentityPublicKey> {
    let now = now_ms();
    let qualifies = |key: &IdentityPublicKey| {
        key.purpose() == Purpose::AUTHENTICATION
            && security_levels.contains(&key.security_level())
            && key_types.contains(&key.key_type())
            && !key.is_disabled()
            && !key.is_expired_at(now)
    };
    let keys = identity.public_keys();
    keys.values()
        .find(|key| qualifies(key) && !key.has_limits())
        .or_else(|| keys.values().find(|key| qualifies(key)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyID;
    use dpp::platform_value::{BinaryData, Identifier};
    use std::collections::BTreeMap;

    fn key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id,
            purpose: Purpose::AUTHENTICATION,
            security_level,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![id as u8; 33]),
            disabled_at: None,
        })
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

    const LEVELS: [SecurityLevel; 2] = [SecurityLevel::HIGH, SecurityLevel::CRITICAL];
    const TYPES: [KeyType; 1] = [KeyType::ECDSA_SECP256K1];

    #[test]
    fn prefers_a_key_without_limits_over_an_earlier_limited_one() {
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            key(1, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(2, SecurityLevel::HIGH),
        ]);
        let chosen = usable_authentication_key(&identity, &LEVELS, &TYPES).expect("a key");
        assert_eq!(chosen.id(), 2);
    }

    #[test]
    fn falls_back_to_a_limited_key_that_has_not_expired() {
        let far_future = now_ms() + 1_000_000;
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            key(1, SecurityLevel::CRITICAL).with_limits(None, Some(1)),
            key(2, SecurityLevel::CRITICAL).with_limits(Some(1_000), Some(far_future)),
        ]);
        let chosen = usable_authentication_key(&identity, &LEVELS, &TYPES).expect("a key");
        assert_eq!(chosen.id(), 2, "the expired key 1 is skipped");
    }

    #[test]
    fn answers_none_when_every_candidate_is_disabled_or_expired() {
        let mut disabled = key(1, SecurityLevel::HIGH);
        {
            use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
            disabled.set_disabled_at(5);
        }
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            disabled,
            key(2, SecurityLevel::CRITICAL).with_limits(None, Some(1)),
        ]);
        assert!(usable_authentication_key(&identity, &LEVELS, &TYPES).is_none());
    }
}
