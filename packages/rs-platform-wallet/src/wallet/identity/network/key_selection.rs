//! Choosing an authentication key to sign a document with, now that a key may carry limits
//! or contract bounds.

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use super::signing_key::signing_key_unavailable;
use crate::util::now_ms;

/// The first AUTHENTICATION key of `identity` at one of `security_levels`, of one of
/// `key_types`, that can sign a `document_type_name` document of `contract_id` now
/// AND that `signer` reports it can sign with.
///
/// A key bound to another contract, or to another document type of this contract, cannot
/// authorize the write and is skipped. A key without limits is preferred: a key with a budget
/// or an expiry is an application key, taken only when no unlimited key qualifies. A disabled
/// key is skipped, and so is a key whose expiry has passed the wall clock (the block time
/// trails it by seconds at most). What is left of a budget is not known offline; a spent key
/// is refused by Platform.
///
/// `Ok(None)` means no key is eligible at all; `Err` means at least one eligible key exists
/// but the signer cannot reach any of them (see [`super::signing_key::available_signing_key`]
/// for the same distinction on the non-contract-bound selectors).
pub(crate) fn usable_authentication_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    contract_id: Identifier,
    document_type_name: &str,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
) -> Result<Option<&'a IdentityPublicKey>, ProtocolError> {
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
    // Unlimited-qualifying keys first, then limited ones — same preference order as before,
    // now also gated on signer availability within each tier.
    let unlimited = keys
        .values()
        .filter(|key| qualifies(key) && !key.has_limits());
    let limited = keys
        .values()
        .filter(|key| qualifies(key) && key.has_limits());
    let mut unavailable = None;
    for key in unlimited.chain(limited) {
        if signer.can_sign_with(key) {
            return Ok(Some(key));
        }
        unavailable.get_or_insert(key);
    }
    match unavailable {
        Some(key) => Err(signing_key_unavailable(key)),
        None => Ok(None),
    }
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
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use async_trait::async_trait;
    use dpp::address_funds::AddressWitness;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyID;
    use dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;

    const CONTRACT: [u8; 32] = [0xDA; 32];
    const OTHER_CONTRACT: [u8; 32] = [0x0E; 32];
    const DOCUMENT_TYPE: &str = "profile";

    /// A signer that reports every key as available — the tests inherited from before
    /// the signer parameter existed assert on eligibility, not availability.
    #[derive(Debug)]
    struct AllKeys;

    #[async_trait]
    impl Signer<IdentityPublicKey> for AllKeys {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            panic!("key selection must not sign")
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("key selection must not create a witness")
        }
        fn can_sign_with(&self, _: &IdentityPublicKey) -> bool {
            true
        }
    }

    /// A signer available only for the listed key ids.
    #[derive(Debug)]
    struct AvailableKeys(Vec<u32>);

    #[async_trait]
    impl Signer<IdentityPublicKey> for AvailableKeys {
        async fn sign(&self, _: &IdentityPublicKey, _: &[u8]) -> Result<BinaryData, ProtocolError> {
            panic!("key selection must not sign")
        }
        async fn sign_create_witness(
            &self,
            _: &IdentityPublicKey,
            _: &[u8],
        ) -> Result<AddressWitness, ProtocolError> {
            panic!("key selection must not create a witness")
        }
        fn can_sign_with(&self, key: &IdentityPublicKey) -> bool {
            self.0.contains(&key.id())
        }
    }

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
            &AllKeys,
            Identifier::from(CONTRACT),
            DOCUMENT_TYPE,
            &LEVELS,
            &TYPES,
        )
        .unwrap()
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

    fn pick_with_signer(
        identity: &Identity,
        available: &[KeyID],
    ) -> Result<Option<KeyID>, ProtocolError> {
        usable_authentication_key(
            identity,
            &AvailableKeys(available.to_vec()),
            Identifier::from(CONTRACT),
            DOCUMENT_TYPE,
            &LEVELS,
            &TYPES,
        )
        .map(|key| key.map(|k| k.id()))
    }

    fn bound_elsewhere(id: KeyID) -> IdentityPublicKey {
        bound_key(
            id,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract {
                id: Identifier::from(OTHER_CONTRACT),
            }),
        )
    }

    #[test]
    fn falls_back_past_an_unavailable_unlimited_key_to_an_available_limited_one() {
        let subject = identity(vec![
            key(1, SecurityLevel::CRITICAL),
            bound_elsewhere(2),
            key(3, SecurityLevel::HIGH).with_limits(Some(1_000), None),
        ]);
        assert_eq!(
            pick_with_signer(&subject, &[2, 3]).unwrap(),
            Some(3),
            "unavailable key 1 is skipped, and key 2 is not usable here even though \
             the signer holds it"
        );
    }

    #[test]
    fn errs_naming_the_eligible_key_when_only_ineligible_keys_are_available() {
        let subject = identity(vec![
            bound_elsewhere(1),
            key(2, SecurityLevel::HIGH).with_limits(Some(1_000), None),
            key(3, SecurityLevel::CRITICAL).with_limits(None, Some(1)),
        ]);
        let err = pick_with_signer(&subject, &[1, 3]).unwrap_err().to_string();
        assert!(
            err.contains(SIGNER_KEY_UNAVAILABLE_PREFIX),
            "must surface as signer-unavailable: {err}"
        );
        assert!(
            err.contains("Signing key 2 "),
            "only key 2 is eligible, so it is the one reported: {err}"
        );
    }

    #[test]
    fn answers_none_rather_than_err_when_no_key_is_eligible_at_all() {
        let subject = identity(vec![
            key(0, SecurityLevel::MASTER),
            bound_elsewhere(1),
            key(2, SecurityLevel::HIGH).with_limits(None, Some(1)),
        ]);
        assert_eq!(
            pick_with_signer(&subject, &[]).unwrap(),
            None,
            "unavailable but ineligible keys must not turn `None` into a signer error"
        );
    }
}
