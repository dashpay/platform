//! Choosing an authentication key to sign a document with, now that a key may carry limits
//! or contract bounds.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;

use crate::util::now_ms;

/// The first AUTHENTICATION key of `identity` at one of `security_levels`, of one of
/// `key_types`, that can sign a `document_type_name` document of `contract_id` now.
///
/// A key bound to another contract, or to another document type of this contract, cannot
/// authorize the write and is skipped. A key without limits is preferred: a key with a budget
/// or an expiry is an application key, taken only when no unlimited key qualifies. A disabled
/// key is skipped, and so is a key whose expiry has passed the wall clock (the block time
/// trails it by seconds at most). What is left of a budget is not known offline; a spent key
/// is refused by Platform.
pub(crate) fn usable_authentication_key<'a>(
    identity: &'a Identity,
    contract_id: Identifier,
    document_type_name: &str,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
) -> Option<&'a IdentityPublicKey> {
    let now = now_ms();
    let qualifies = |key: &IdentityPublicKey| {
        key.purpose() == Purpose::AUTHENTICATION
            && security_levels.contains(&key.security_level())
            && key_types.contains(&key.key_type())
            && bounds_cover(key.contract_bounds(), contract_id, document_type_name)
            && !key.is_disabled()
            && !key.is_expired_at(now)
    };
    let keys = identity.public_keys();
    keys.values()
        .find(|key| qualifies(key) && !key.has_limits())
        .or_else(|| keys.values().find(|key| qualifies(key)))
}

/// Whether a key carrying `bounds` may sign a `document_type_name` document of `contract_id`.
///
/// A contract group bound is answered by group membership, state the wallet does not hold,
/// so a group-bound key is never chosen here: the DashPay writes this picker serves are
/// signed with an unbound key or one bound to DashPay itself.
fn bounds_cover(
    bounds: Option<&ContractBounds>,
    contract_id: Identifier,
    document_type_name: &str,
) -> bool {
    match bounds {
        None => true,
        Some(ContractBounds::SingleContract { id }) => *id == contract_id,
        Some(ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: bound_document_type_name,
        }) => *id == contract_id && bound_document_type_name == document_type_name,
        Some(ContractBounds::ContractGroup { .. }) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyID;
    use dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;

    const CONTRACT: [u8; 32] = [0xDA; 32];
    const OTHER_CONTRACT: [u8; 32] = [0x0E; 32];
    const DOCUMENT_TYPE: &str = "profile";

    fn key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        bound_key(id, security_level, None)
    }

    fn bound_key(
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

    fn pick(identity: &Identity) -> Option<&IdentityPublicKey> {
        usable_authentication_key(
            identity,
            Identifier::from(CONTRACT),
            DOCUMENT_TYPE,
            &LEVELS,
            &TYPES,
        )
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
        let chosen = pick(&identity).expect("a key");
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
        let chosen = pick(&identity).expect("a key");
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
        assert!(pick(&identity).is_none());
    }

    #[test]
    fn prefers_an_unbound_limited_key_over_an_unlimited_key_bound_to_another_contract() {
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            key(1, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            bound_key(
                2,
                SecurityLevel::HIGH,
                Some(ContractBounds::SingleContract {
                    id: Identifier::from(OTHER_CONTRACT),
                }),
            ),
        ]);
        let chosen = pick(&identity).expect("a key");
        assert_eq!(
            chosen.id(),
            1,
            "the key bound to another contract cannot authorize this write"
        );
    }

    #[test]
    fn accepts_a_key_bound_to_this_contract_or_this_document_type() {
        let whole_contract = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract {
                id: Identifier::from(CONTRACT),
            }),
        );
        assert_eq!(
            pick(&identity(vec![whole_contract])).map(|k| k.id()),
            Some(1)
        );

        let this_document_type = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContractDocumentType {
                id: Identifier::from(CONTRACT),
                document_type_name: DOCUMENT_TYPE.to_string(),
            }),
        );
        assert_eq!(
            pick(&identity(vec![this_document_type])).map(|k| k.id()),
            Some(1)
        );
    }

    #[test]
    fn skips_a_key_bound_to_another_document_type_or_to_a_contract_group() {
        let other_document_type = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContractDocumentType {
                id: Identifier::from(CONTRACT),
                document_type_name: "contactInfo".to_string(),
            }),
        );
        assert!(pick(&identity(vec![other_document_type])).is_none());

        let group = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::ContractGroup {
                id: Identifier::from(CONTRACT),
            }),
        );
        assert!(
            pick(&identity(vec![group])).is_none(),
            "group membership is state the wallet does not hold"
        );
    }
}
