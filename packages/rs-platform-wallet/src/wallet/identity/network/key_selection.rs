//! Choosing the identity key that signs a state transition, now that a key may carry limits
//! or contract bounds (protocol version 14).

use dpp::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use dpp::consensus::signature::{
    ContractBoundedKeyNonBatchError, ContractBoundedKeyOutOfBoundsError, PublicKeyExpiredError,
    PublicKeyIsDisabledError, SignatureError,
};
use dpp::consensus::ConsensusError;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::contract_bounds::{
    BatchedTransitionBoundsCheck, ContractBoundsScope,
};
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::state_transition::errors::{
    InvalidIdentityPublicKeyTypeError, WrongPublicKeyPurposeError,
};
use dpp::ProtocolError;

use super::signing_key::{first_available, require_available};
use std::time::{SystemTime, UNIX_EPOCH};

/// The key types Platform verifies an identity signature by (Drive's
/// `validate_state_transition_identity_signed`). A key the caller chose is checked against
/// these, not against the narrower set the wallet picks its own keys from.
pub(crate) const PLATFORM_SIGNING_KEY_TYPES: [KeyType; 3] = [
    KeyType::ECDSA_SECP256K1,
    KeyType::BLS12_381,
    KeyType::ECDSA_HASH160,
];

/// The key types the wallet picks its own signing keys from. A key the caller chose is checked
/// against [`PLATFORM_SIGNING_KEY_TYPES`] instead.
pub(crate) const WALLET_SIGNING_KEY_TYPES: [KeyType; 1] = [KeyType::ECDSA_SECP256K1];

/// What a signature authorizes, as far as a key's contract bounds go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigningScope<'a> {
    /// A transition that is not a batch: identity update, credit transfer or withdrawal,
    /// contract create or update. A contract-bound AUTHENTICATION key cannot sign it.
    NonBatch,
    /// A batch of `document_type_name` documents of `contract_id`.
    DocumentType {
        contract_id: Identifier,
        document_type_name: &'a str,
    },
    /// A batch acting on all of `contract_id`: a token operation, or a flow that writes more
    /// than one of its document types.
    Contract { contract_id: Identifier },
}

impl SigningScope<'_> {
    /// What `key`'s contract bounds say about signing in this scope: `Allowed`, `Denied`, or
    /// `RequiresContractGroupMembership` for a group-bound key, which only Platform can answer.
    /// Bounds limit only AUTHENTICATION keys, as in consensus, and a contract-bound
    /// AUTHENTICATION key is `Denied` outside a batch ([`IdentityPublicKey::can_sign_non_batch`]).
    pub fn bounds_check(&self, key: &IdentityPublicKey) -> BatchedTransitionBoundsCheck {
        let (contract_id, bounds_scope) = match *self {
            SigningScope::NonBatch if key.can_sign_non_batch() => {
                return BatchedTransitionBoundsCheck::Allowed
            }
            SigningScope::NonBatch => return BatchedTransitionBoundsCheck::Denied,
            SigningScope::DocumentType {
                contract_id,
                document_type_name,
            } => (
                contract_id,
                ContractBoundsScope::DocumentType(document_type_name),
            ),
            SigningScope::Contract { contract_id } => (contract_id, ContractBoundsScope::Contract),
        };
        match key.contract_bounds() {
            Some(bounds) if key.purpose() == Purpose::AUTHENTICATION => {
                bounds.check_scope(contract_id, bounds_scope)
            }
            _ => BatchedTransitionBoundsCheck::Allowed,
        }
    }
}

/// The key of `identity` that should sign in `scope`: one of `purpose`, at one of
/// `security_levels`, of one of `key_types`, that Platform would accept now AND that `signer`
/// reports it can sign with.
///
/// A key whose contract bounds do not cover `scope` is skipped, and so is a key bound to a
/// contract group: membership is state the wallet does not hold. A key without limits is
/// preferred over any key with limits, whatever their security levels: a key with a budget or
/// an expiry is an application key, taken only when no unlimited key qualifies. Within each
/// group keys go in key-id order. A disabled key is skipped, and so is a key whose expiry has
/// passed the wall clock (the block time trails it by seconds at most), or any key with an
/// expiry when the system clock reads before the Unix epoch. What is left of a
/// budget is not known offline; a spent key is refused by Platform.
///
/// `Ok(None)` means no key is eligible at all; `Err` means at least one eligible key exists
/// but the signer cannot reach any of them.
pub fn usable_signing_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    purpose: Purpose,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
    scope: SigningScope<'_>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key_at(
        identity,
        signer,
        purpose,
        security_levels,
        key_types,
        scope,
        wall_clock_ms(),
    )
}

/// [`usable_signing_key`] at wall-clock time `now`, in milliseconds since the Unix epoch;
/// `None` when the clock cannot be read.
fn usable_signing_key_at<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    purpose: Purpose,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
    scope: SigningScope<'_>,
    now: Option<u64>,
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    let qualifies = |key: &IdentityPublicKey| {
        key.purpose() == purpose
            && security_levels.contains(&key.security_level())
            && key_types.contains(&key.key_type())
            && !key.is_disabled()
            && !expired_or_unknown(key, now)
            && scope.bounds_check(key) == BatchedTransitionBoundsCheck::Allowed
    };
    let keys = identity.public_keys();
    let unlimited = keys
        .values()
        .filter(|key| qualifies(key) && !key.has_limits());
    let limited = keys
        .values()
        .filter(|key| qualifies(key) && key.has_limits());
    first_available(unlimited.chain(limited), signer)
}

/// Check a key the caller chose to sign in `scope` against the rules [`usable_signing_key`]
/// applies, so an explicit choice is refused before anything is built or signed.
///
/// Each refusal is the typed error Platform itself returns for it: a purpose outside
/// `purposes`, a security level outside `security_levels`, a key type outside `key_types`, a
/// disabled key, an expired key, or contract bounds that do not cover `scope`. A key bound to
/// a contract group is accepted: only Platform can read the group's memberships. A key
/// `signer` cannot sign with is refused with the signer-unavailable error. A key with an
/// expiry is refused with [`dash_sdk::Error::Config`] when the system clock reads before the
/// Unix epoch, since its expiry cannot be checked.
pub fn check_signing_key(
    key: &IdentityPublicKey,
    signer: &impl Signer<IdentityPublicKey>,
    purposes: &[Purpose],
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
    scope: SigningScope<'_>,
) -> Result<(), dash_sdk::Error> {
    check_signing_key_at(
        key,
        signer,
        purposes,
        security_levels,
        key_types,
        scope,
        wall_clock_ms(),
    )
}

/// [`check_signing_key`] at wall-clock time `now`, in milliseconds since the Unix epoch;
/// `None` when the clock cannot be read.
fn check_signing_key_at(
    key: &IdentityPublicKey,
    signer: &impl Signer<IdentityPublicKey>,
    purposes: &[Purpose],
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
    scope: SigningScope<'_>,
    now: Option<u64>,
) -> Result<(), dash_sdk::Error> {
    if !purposes.contains(&key.purpose()) {
        return Err(
            ProtocolError::WrongPublicKeyPurposeError(WrongPublicKeyPurposeError::new(
                key.purpose(),
                purposes.to_vec(),
            ))
            .into(),
        );
    }
    if !security_levels.contains(&key.security_level()) {
        return Err(ProtocolError::InvalidSignaturePublicKeySecurityLevelError(
            InvalidSignaturePublicKeySecurityLevelError::new(
                key.security_level(),
                security_levels.to_vec(),
            ),
        )
        .into());
    }
    if !key_types.contains(&key.key_type()) {
        return Err(ProtocolError::InvalidIdentityPublicKeyTypeError(
            InvalidIdentityPublicKeyTypeError::new(key.key_type()),
        )
        .into());
    }
    if key.is_disabled() {
        return Err(
            ProtocolError::PublicKeyIsDisabledError(PublicKeyIsDisabledError::new(key.id())).into(),
        );
    }
    if let Some(expires_at) = key.expires_at() {
        let Some(now) = now else {
            return Err(dash_sdk::Error::Config(format!(
                "the system clock reads before the Unix epoch, so whether key {} has expired \
                 cannot be told",
                key.id()
            )));
        };
        if key.is_expired_at(now) {
            return Err(signature_refusal(SignatureError::PublicKeyExpiredError(
                PublicKeyExpiredError::new(key.id(), expires_at, now),
            )));
        }
    }
    match scope.bounds_check(key) {
        BatchedTransitionBoundsCheck::Allowed
        | BatchedTransitionBoundsCheck::RequiresContractGroupMembership { .. } => {}
        BatchedTransitionBoundsCheck::Denied if scope == SigningScope::NonBatch => {
            return Err(signature_refusal(
                SignatureError::ContractBoundedKeyNonBatchError(
                    ContractBoundedKeyNonBatchError::new(key.id()),
                ),
            ));
        }
        BatchedTransitionBoundsCheck::Denied => {
            return Err(signature_refusal(
                SignatureError::ContractBoundedKeyOutOfBoundsError(
                    ContractBoundedKeyOutOfBoundsError::new(key.id()),
                ),
            ));
        }
    }
    require_available(key, signer).map(|_| ())
}

/// Whether `key` cannot be shown unexpired at `now`: its expiry has passed, or it has one and
/// the clock could not be read. A key without an expiry never expires.
fn expired_or_unknown(key: &IdentityPublicKey, now: Option<u64>) -> bool {
    match (key.expires_at(), now) {
        (None, _) => false,
        (Some(_), None) => true,
        (Some(_), Some(now)) => key.is_expired_at(now),
    }
}

/// Milliseconds since the Unix epoch, or `None` when the system clock reads before it.
fn wall_clock_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_millis() as u64)
}

fn signature_refusal(error: SignatureError) -> dash_sdk::Error {
    ProtocolError::ConsensusError(Box::new(ConsensusError::SignatureError(error))).into()
}

/// The first AUTHENTICATION key of `identity` at one of `security_levels`, of one of
/// `key_types`, that can sign a `document_type_name` document of `contract_id` now AND that
/// `signer` can sign with: [`usable_signing_key`] for one document type.
pub(crate) fn usable_authentication_key<'a>(
    identity: &'a Identity,
    signer: &impl Signer<IdentityPublicKey>,
    contract_id: Identifier,
    document_type_name: &str,
    security_levels: &[SecurityLevel],
    key_types: &[KeyType],
) -> Result<Option<&'a IdentityPublicKey>, dash_sdk::Error> {
    usable_signing_key(
        identity,
        signer,
        Purpose::AUTHENTICATION,
        security_levels,
        key_types,
        SigningScope::DocumentType {
            contract_id,
            document_type_name,
        },
    )
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::signing_key::tests::{available, KeyFilter};
    use super::*;
    use crate::error::SIGNER_KEY_UNAVAILABLE_PREFIX;
    use crate::util::now_ms;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeySettersV0;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::KeyID;
    use dpp::platform_value::BinaryData;
    use std::collections::BTreeMap;

    const CONTRACT: [u8; 32] = [0xDA; 32];
    const OTHER_CONTRACT: [u8; 32] = [0x0E; 32];
    const DOCUMENT_TYPE: &str = "profile";

    pub(crate) fn key(id: KeyID, security_level: SecurityLevel) -> IdentityPublicKey {
        bound_key(id, security_level, None)
    }

    pub(crate) fn bound_key(
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

    fn pick(identity: &Identity) -> Option<&IdentityPublicKey> {
        usable_authentication_key(
            identity,
            &KeyFilter(|_| true),
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
        disabled.set_disabled_at(5);
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
        ids: &[KeyID],
    ) -> Result<Option<KeyID>, dash_sdk::Error> {
        usable_authentication_key(
            identity,
            &available(ids),
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

    fn transfer_key(id: KeyID, contract_bounds: Option<ContractBounds>) -> IdentityPublicKey {
        let mut key = bound_key(id, SecurityLevel::CRITICAL, contract_bounds);
        key.set_purpose(Purpose::TRANSFER);
        key
    }

    fn pick_in(identity: &Identity, purpose: Purpose, scope: SigningScope<'_>) -> Option<KeyID> {
        usable_signing_key(
            identity,
            &KeyFilter(|_| true),
            purpose,
            &SecurityLevel::full_range(),
            &TYPES,
            scope,
        )
        .unwrap()
        .map(|key| key.id())
    }

    fn whole_contract() -> SigningScope<'static> {
        SigningScope::Contract {
            contract_id: Identifier::from(CONTRACT),
        }
    }

    #[test]
    fn should_rank_every_unlimited_key_before_any_limited_one_across_levels() {
        let identity = identity(vec![
            key(1, SecurityLevel::CRITICAL),
            key(2, SecurityLevel::HIGH).with_limits(Some(1_000), None),
        ]);
        assert_eq!(
            pick(&identity).map(|key| key.id()),
            Some(1),
            "a budgeted HIGH key is not spent while an unlimited CRITICAL key qualifies"
        );
    }

    #[test]
    fn should_refuse_a_contract_bound_authentication_key_for_a_non_batch_transition() {
        let identity = identity(vec![
            bound_key(
                1,
                SecurityLevel::CRITICAL,
                Some(ContractBounds::SingleContract {
                    id: Identifier::from(CONTRACT),
                }),
            ),
            key(2, SecurityLevel::CRITICAL),
        ]);
        assert_eq!(
            pick_in(&identity, Purpose::AUTHENTICATION, SigningScope::NonBatch),
            Some(2)
        );
    }

    #[test]
    fn should_not_apply_contract_bounds_to_keys_of_other_purposes() {
        let bounds = Some(ContractBounds::SingleContract {
            id: Identifier::from(OTHER_CONTRACT),
        });
        let identity = identity(vec![transfer_key(1, bounds)]);
        assert_eq!(
            pick_in(&identity, Purpose::TRANSFER, SigningScope::NonBatch),
            Some(1)
        );
        assert_eq!(
            pick_in(&identity, Purpose::TRANSFER, whole_contract()),
            Some(1)
        );
    }

    #[test]
    fn should_cover_a_whole_contract_only_with_unbound_or_whole_contract_keys() {
        let identity = identity(vec![
            bound_key(
                1,
                SecurityLevel::CRITICAL,
                Some(ContractBounds::SingleContractDocumentType {
                    id: Identifier::from(CONTRACT),
                    document_type_name: DOCUMENT_TYPE.to_string(),
                }),
            ),
            bound_key(
                2,
                SecurityLevel::CRITICAL,
                Some(ContractBounds::SingleContract {
                    id: Identifier::from(CONTRACT),
                }),
            ),
        ]);
        assert_eq!(
            pick_in(&identity, Purpose::AUTHENTICATION, whole_contract()),
            Some(2),
            "a document-type bound key cannot act on the rest of the contract"
        );
    }

    fn check(key: &IdentityPublicKey, scope: SigningScope<'_>) -> Result<(), dash_sdk::Error> {
        check_signing_key(
            key,
            &KeyFilter(|_| true),
            &[Purpose::AUTHENTICATION],
            &LEVELS,
            &TYPES,
            scope,
        )
    }

    fn document_scope() -> SigningScope<'static> {
        SigningScope::DocumentType {
            contract_id: Identifier::from(CONTRACT),
            document_type_name: DOCUMENT_TYPE,
        }
    }

    fn signature_error(result: Result<(), dash_sdk::Error>) -> SignatureError {
        match result {
            Err(dash_sdk::Error::Protocol(ProtocolError::ConsensusError(error))) => match *error {
                ConsensusError::SignatureError(error) => error,
                other => panic!("not a signature error: {other:?}"),
            },
            other => panic!("not a consensus error: {other:?}"),
        }
    }

    #[test]
    fn should_accept_an_explicit_key_that_qualifies() {
        let far_future = now_ms() + 1_000_000;
        let limited = key(1, SecurityLevel::HIGH).with_limits(Some(1), Some(far_future));
        check(&limited, document_scope()).expect("a limited key that has not expired");
        let group_bound = bound_key(
            2,
            SecurityLevel::HIGH,
            Some(ContractBounds::ContractGroup {
                id: Identifier::from(OTHER_CONTRACT),
            }),
        );
        check(&group_bound, document_scope())
            .expect("group membership is left to Platform for an explicit choice");
    }

    #[test]
    fn should_refuse_an_explicit_key_with_the_error_platform_returns() {
        let mut wrong_purpose = key(1, SecurityLevel::HIGH);
        wrong_purpose.set_purpose(Purpose::TRANSFER);
        assert!(matches!(
            check(&wrong_purpose, document_scope()),
            Err(dash_sdk::Error::Protocol(
                ProtocolError::WrongPublicKeyPurposeError(_)
            ))
        ));
        assert!(matches!(
            check(&key(1, SecurityLevel::MASTER), document_scope()),
            Err(dash_sdk::Error::Protocol(
                ProtocolError::InvalidSignaturePublicKeySecurityLevelError(_)
            ))
        ));
        let mut hash160 = key(1, SecurityLevel::HIGH);
        hash160.set_key_type(KeyType::ECDSA_HASH160);
        assert!(matches!(
            check(&hash160, document_scope()),
            Err(dash_sdk::Error::Protocol(
                ProtocolError::InvalidIdentityPublicKeyTypeError(_)
            ))
        ));
        let mut disabled = key(1, SecurityLevel::HIGH);
        disabled.set_disabled_at(5);
        assert!(matches!(
            check(&disabled, document_scope()),
            Err(dash_sdk::Error::Protocol(
                ProtocolError::PublicKeyIsDisabledError(_)
            ))
        ));
        let expired = key(1, SecurityLevel::HIGH).with_limits(None, Some(1));
        assert!(matches!(
            signature_error(check(&expired, document_scope())),
            SignatureError::PublicKeyExpiredError(error) if error.expires_at() == 1
        ));
        assert!(matches!(
            signature_error(check(&bound_elsewhere(1), document_scope())),
            SignatureError::ContractBoundedKeyOutOfBoundsError(_)
        ));
        let bound_here = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract {
                id: Identifier::from(CONTRACT),
            }),
        );
        assert!(matches!(
            signature_error(check(&bound_here, SigningScope::NonBatch)),
            SignatureError::ContractBoundedKeyNonBatchError(_)
        ));
    }

    #[test]
    fn should_refuse_an_explicit_key_the_signer_cannot_use() {
        let err = check_signing_key(
            &key(1, SecurityLevel::HIGH),
            &available(&[]),
            &[Purpose::AUTHENTICATION],
            &LEVELS,
            &TYPES,
            document_scope(),
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains(SIGNER_KEY_UNAVAILABLE_PREFIX), "{err}");
    }

    #[test]
    fn should_report_the_bounds_verdict_for_each_scope() {
        use BatchedTransitionBoundsCheck::{Allowed, Denied, RequiresContractGroupMembership};
        let contract = Identifier::from(CONTRACT);
        let group_id = Identifier::from(OTHER_CONTRACT);
        let group_bound = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::ContractGroup { id: group_id }),
        );
        let bound_here = bound_key(
            2,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract { id: contract }),
        );
        assert_eq!(
            document_scope().bounds_check(&key(1, SecurityLevel::HIGH)),
            Allowed
        );
        assert_eq!(document_scope().bounds_check(&bound_here), Allowed);
        assert_eq!(document_scope().bounds_check(&bound_elsewhere(3)), Denied);
        assert_eq!(SigningScope::NonBatch.bounds_check(&bound_here), Denied);
        assert_eq!(
            SigningScope::NonBatch
                .bounds_check(&transfer_key(4, bound_here.contract_bounds().cloned())),
            Allowed,
            "bounds limit only AUTHENTICATION keys"
        );
        assert!(matches!(
            document_scope().bounds_check(&group_bound),
            RequiresContractGroupMembership { contract_group_id, .. } if contract_group_id == group_id
        ));
        assert!(matches!(
            whole_contract().bounds_check(&group_bound),
            RequiresContractGroupMembership { contract_id, .. } if contract_id == contract
        ));
    }

    #[test]
    fn should_deny_a_non_batch_signature_exactly_when_dpp_says_the_key_cannot_sign_one() {
        // Every purpose dpp decodes, so a purpose added later is covered too.
        let purposes: Vec<Purpose> = (0..=u8::MAX)
            .filter_map(|byte| Purpose::try_from(byte).ok())
            .collect();
        assert!(purposes.len() >= 7, "{purposes:?}");
        let contract_id = Identifier::from(CONTRACT);
        let bounds = [
            None,
            Some(ContractBounds::SingleContract { id: contract_id }),
            Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name: DOCUMENT_TYPE.to_string(),
            }),
            Some(ContractBounds::ContractGroup { id: contract_id }),
        ];
        for &purpose in &purposes {
            for contract_bounds in &bounds {
                let mut key = bound_key(1, SecurityLevel::HIGH, contract_bounds.clone());
                key.set_purpose(purpose);
                let expected = if key.can_sign_non_batch() {
                    BatchedTransitionBoundsCheck::Allowed
                } else {
                    BatchedTransitionBoundsCheck::Denied
                };
                assert_eq!(
                    SigningScope::NonBatch.bounds_check(&key),
                    expected,
                    "{purpose:?} {contract_bounds:?}"
                );
            }
        }
    }

    #[test]
    fn should_not_trust_an_expiry_without_a_valid_clock() {
        let far_future = now_ms() + 1_000_000;
        let expiring = key(1, SecurityLevel::HIGH).with_limits(None, Some(far_future));
        let pick_without_clock = |identity: &Identity| {
            usable_signing_key_at(
                identity,
                &KeyFilter(|_| true),
                Purpose::AUTHENTICATION,
                &LEVELS,
                &TYPES,
                document_scope(),
                None,
            )
            .unwrap()
            .map(|key| key.id())
        };
        assert_eq!(
            pick_without_clock(&identity(vec![expiring.clone()])),
            None,
            "without a clock an expiry cannot be told from a passed one"
        );
        let budget_only = key(2, SecurityLevel::HIGH).with_limits(Some(1_000), None);
        assert_eq!(
            pick_without_clock(&identity(vec![expiring.clone(), budget_only])),
            Some(2),
            "a key without an expiry does not need the clock"
        );
        let refusal = check_signing_key_at(
            &expiring,
            &KeyFilter(|_| true),
            &[Purpose::AUTHENTICATION],
            &LEVELS,
            &TYPES,
            document_scope(),
            None,
        );
        assert!(
            matches!(refusal, Err(dash_sdk::Error::Config(_))),
            "an explicit key with an expiry is refused without a clock: {refusal:?}"
        );
    }
}
