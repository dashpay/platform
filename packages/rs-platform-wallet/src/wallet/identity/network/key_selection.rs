//! Choosing the identity key that signs a state transition, now that a key may carry limits
//! or contract bounds (protocol version 14).

use dpp::consensus::signature::InvalidSignaturePublicKeySecurityLevelError;
use dpp::consensus::signature::{
    ContractBoundedKeyNonBatchError, ContractBoundedKeyOutOfBoundsError,
    InvalidIdentityPublicKeyTypeError, InvalidSignaturePublicKeyPurposeError,
    PublicKeyExpiredError, PublicKeyIsDisabledError, SignatureError,
};
use dpp::consensus::ConsensusError;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::TokenContractPosition;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::accessors::v1::IdentityPublicKeyGettersV1;
use dpp::identity::identity_public_key::contract_bounds::{
    BatchedTransitionBoundsCheck, ContractBoundsScope,
};
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::ProtocolError;

use super::signing_key::{first_available, require_available};
use std::cmp::Reverse;
use std::time::{SystemTime, UNIX_EPOCH};

/// The key types Platform verifies an identity signature by (Drive's
/// `validate_state_transition_identity_signed`). Pass these as `key_types` to
/// [`check_signing_key`] for a key the caller chose, so it is refused exactly where Platform
/// refuses it. The wallet's own choice of a credit transfer or withdrawal key uses them too,
/// so an `ECDSA_HASH160` TRANSFER key stays usable. Other automatic selection uses the
/// narrower [`WALLET_SIGNING_KEY_TYPES`].
pub const PLATFORM_SIGNING_KEY_TYPES: [KeyType; 3] = [
    KeyType::ECDSA_SECP256K1,
    KeyType::BLS12_381,
    KeyType::ECDSA_HASH160,
];

/// The key types the wallet picks its own signing keys from, except for a credit transfer or
/// withdrawal key, which may be any of [`PLATFORM_SIGNING_KEY_TYPES`]. Pass these as
/// `key_types` to [`usable_signing_key`] to choose the key the wallet itself would; a key the
/// caller chose is checked against [`PLATFORM_SIGNING_KEY_TYPES`] instead.
pub const WALLET_SIGNING_KEY_TYPES: [KeyType; 1] = [KeyType::ECDSA_SECP256K1];

/// What a signature authorizes, as far as a key's contract bounds go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SigningScope<'a> {
    /// A transition that is not a batch: identity update, credit transfer or withdrawal,
    /// contract create or update. A contract-bound AUTHENTICATION key cannot sign it.
    NonBatch,
    /// A batch creating a `document_type` document. The key's bounds must cover the type and
    /// every type the create may consume, as Platform checks.
    DocumentCreate { document_type: DocumentTypeRef<'a> },
    /// A batch acting on existing `document_type_name` documents of `contract_id`.
    DocumentType {
        contract_id: Identifier,
        document_type_name: &'a str,
    },
    /// A batch acting on all of `contract_id`: a flow that writes more than one of its
    /// document types.
    Contract { contract_id: Identifier },
    /// A batch acting on the token at `position` of `contract_id`.
    Token {
        contract_id: Identifier,
        position: TokenContractPosition,
    },
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
            SigningScope::DocumentCreate { ref document_type } => (
                document_type.data_contract_id(),
                ContractBoundsScope::DocumentType(document_type.name()),
            ),
            SigningScope::Contract { contract_id } => (contract_id, ContractBoundsScope::Contract),
            SigningScope::Token {
                contract_id,
                position,
            } => (contract_id, ContractBoundsScope::Token(position)),
        };
        let Some(bounds) = key
            .contract_bounds()
            .filter(|_| key.purpose() == Purpose::AUTHENTICATION)
        else {
            return BatchedTransitionBoundsCheck::Allowed;
        };
        let check = bounds.check_scope(contract_id, bounds_scope);
        let SigningScope::DocumentCreate { ref document_type } = *self else {
            return check;
        };
        if check != BatchedTransitionBoundsCheck::Allowed {
            return check;
        }
        // Drive's batch structure validation: a create deletes the documents it consumes with
        // the key that signs it, so the bounds must allow each consumed type too.
        let covers_consumed = document_type
            .consumable_document_type_names()
            .into_iter()
            .all(|consumed| {
                bounds.check_scope(contract_id, ContractBoundsScope::DocumentType(consumed))
                    == BatchedTransitionBoundsCheck::Allowed
            });
        if covers_consumed {
            BatchedTransitionBoundsCheck::Allowed
        } else {
            BatchedTransitionBoundsCheck::Denied
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
/// group the weakest qualifying security level goes first, then the lowest key id: DIP-11 has
/// clients ask for stronger authentication the stronger the key, so a HIGH write signs with a
/// HIGH key rather than a CRITICAL one, and a stronger key signs only when the signer cannot
/// reach a weaker one. A disabled key is skipped, and so is a key whose expiry has
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
    let mut candidates: Vec<&IdentityPublicKey> = identity
        .public_keys()
        .values()
        .filter(|key| qualifies(key))
        .collect();
    // Unlimited keys first, then within each group the weakest level (the highest
    // `SecurityLevel` discriminant: MASTER is 0, MEDIUM 3), then the lowest key id.
    candidates.sort_by_key(|key| {
        (
            key.has_limits(),
            Reverse(key.security_level() as u8),
            key.id(),
        )
    });
    first_available(candidates.into_iter(), signer)
}

/// Check a key the caller chose to sign in `scope` against the rules [`usable_signing_key`]
/// applies, so an explicit choice is refused before anything is built or signed.
///
/// Each refusal is the consensus signature error Platform itself returns for it, checked in
/// Drive's order: a key type outside `key_types`, a purpose outside `purposes`, a security
/// level outside `security_levels`, a disabled key (identity-signed validation); then contract
/// bounds that do not cover `scope` (structure validation); then an expired key (fee
/// validation). A key bound to
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
    // Drive's identity-signed validation checks in this order: key type, purpose, security
    // level, disabled.
    if !key_types.contains(&key.key_type()) {
        return Err(signature_refusal(
            SignatureError::InvalidIdentityPublicKeyTypeError(
                InvalidIdentityPublicKeyTypeError::new(key.key_type()),
            ),
        ));
    }
    if !purposes.contains(&key.purpose()) {
        return Err(signature_refusal(
            SignatureError::InvalidSignaturePublicKeyPurposeError(
                InvalidSignaturePublicKeyPurposeError::new(key.purpose(), purposes.to_vec()),
            ),
        ));
    }
    if !security_levels.contains(&key.security_level()) {
        return Err(signature_refusal(
            SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    key.security_level(),
                    security_levels.to_vec(),
                ),
            ),
        ));
    }
    if key.is_disabled() {
        return Err(signature_refusal(SignatureError::PublicKeyIsDisabledError(
            PublicKeyIsDisabledError::new(key.id()),
        )));
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

/// The AUTHENTICATION key of `identity` at one of `security_levels`, of one of `key_types`,
/// that can sign a `document_type_name` document of `contract_id` now AND that `signer` can
/// sign with, preferred in [`usable_signing_key`]'s order: [`usable_signing_key`] for one
/// document type.
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
    fn should_sign_with_the_weakest_level_the_requirement_accepts() {
        // DIP-11: a CRITICAL key asks for stronger authentication than a HIGH write needs
        let identity = identity(vec![
            key(0, SecurityLevel::MASTER),
            key(1, SecurityLevel::CRITICAL),
            key(2, SecurityLevel::HIGH),
        ]);
        assert_eq!(
            pick(&identity).map(|key| key.id()),
            Some(2),
            "a HIGH requirement signs with the HIGH key, not the lower-id CRITICAL one"
        );
    }

    #[test]
    fn should_take_the_lowest_key_id_among_keys_of_the_weakest_level() {
        let identity = identity(vec![
            key(1, SecurityLevel::CRITICAL),
            key(3, SecurityLevel::HIGH),
            key(2, SecurityLevel::HIGH),
            key(4, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(5, SecurityLevel::HIGH).with_limits(Some(1_000), None),
        ]);
        assert_eq!(pick(&identity).map(|key| key.id()), Some(2));
    }

    #[test]
    fn should_order_limited_keys_weakest_level_first_too() {
        let identity = identity(vec![
            key(1, SecurityLevel::CRITICAL).with_limits(Some(1_000), None),
            key(2, SecurityLevel::HIGH).with_limits(Some(1_000), None),
        ]);
        assert_eq!(pick(&identity).map(|key| key.id()), Some(2));
    }

    #[test]
    fn should_fall_back_to_a_stronger_key_the_signer_holds() {
        let subject = identity(vec![
            key(1, SecurityLevel::CRITICAL),
            key(2, SecurityLevel::HIGH),
        ]);
        assert_eq!(
            pick_with_signer(&subject, &[1]).unwrap(),
            Some(1),
            "the signer cannot reach the HIGH key, so the CRITICAL one signs"
        );
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

    /// The consensus signature error `error` carries, if it is one.
    pub(crate) fn signature_refusal_of(error: &dash_sdk::Error) -> Option<&SignatureError> {
        match error {
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(error)) => match error.as_ref()
            {
                ConsensusError::SignatureError(error) => Some(error),
                _ => None,
            },
            _ => None,
        }
    }

    /// The purposes `error` names when it is Platform's purpose refusal.
    pub(crate) fn refused_purposes(error: &dash_sdk::Error) -> Option<Vec<Purpose>> {
        match error {
            dash_sdk::Error::Protocol(ProtocolError::ConsensusError(error)) => match error.as_ref()
            {
                ConsensusError::SignatureError(
                    SignatureError::InvalidSignaturePublicKeyPurposeError(error),
                ) => Some(error.allowed_key_purposes().clone()),
                _ => None,
            },
            _ => None,
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
        assert_eq!(
            check(&wrong_purpose, document_scope())
                .as_ref()
                .err()
                .and_then(refused_purposes),
            Some(vec![Purpose::AUTHENTICATION])
        );
        assert!(matches!(
            check(&key(1, SecurityLevel::MASTER), document_scope())
                .as_ref()
                .err()
                .and_then(signature_refusal_of),
            Some(SignatureError::InvalidSignaturePublicKeySecurityLevelError(
                _
            ))
        ));
        let mut hash160 = key(1, SecurityLevel::HIGH);
        hash160.set_key_type(KeyType::ECDSA_HASH160);
        assert!(matches!(
            check(&hash160, document_scope())
                .as_ref()
                .err()
                .and_then(signature_refusal_of),
            Some(SignatureError::InvalidIdentityPublicKeyTypeError(_))
        ));
        let mut disabled = key(1, SecurityLevel::HIGH);
        disabled.set_disabled_at(5);
        assert!(matches!(
            check(&disabled, document_scope())
                .as_ref()
                .err()
                .and_then(signature_refusal_of),
            Some(SignatureError::PublicKeyIsDisabledError(_))
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

    #[test]
    fn should_report_a_token_scope_as_the_token_consensus_checks() {
        use dpp::contract_group::ContractGroupMember;
        let contract_id = Identifier::from(CONTRACT);
        let token = SigningScope::Token {
            contract_id,
            position: 3,
        };
        let group_bound = bound_key(
            1,
            SecurityLevel::CRITICAL,
            Some(ContractBounds::ContractGroup {
                id: Identifier::from(OTHER_CONTRACT),
            }),
        );
        assert!(
            matches!(
                token.bounds_check(&group_bound),
                BatchedTransitionBoundsCheck::RequiresContractGroupMembership {
                    contract_id: member_contract,
                    member: ContractGroupMember::Token(3),
                    ..
                } if member_contract == contract_id
            ),
            "{:?}",
            token.bounds_check(&group_bound)
        );
        let bound_here = bound_key(
            2,
            SecurityLevel::CRITICAL,
            Some(ContractBounds::SingleContract { id: contract_id }),
        );
        assert_eq!(
            token.bounds_check(&bound_here),
            BatchedTransitionBoundsCheck::Allowed
        );
        let bound_to_a_document_type = bound_key(
            3,
            SecurityLevel::CRITICAL,
            Some(ContractBounds::SingleContractDocumentType {
                id: contract_id,
                document_type_name: DOCUMENT_TYPE.to_string(),
            }),
        );
        assert_eq!(
            token.bounds_check(&bound_to_a_document_type),
            BatchedTransitionBoundsCheck::Denied
        );
    }

    #[test]
    fn should_report_a_wrong_key_type_before_a_wrong_purpose_as_drive_does() {
        let mut both_wrong = key(1, SecurityLevel::HIGH);
        both_wrong.set_purpose(Purpose::TRANSFER);
        both_wrong.set_key_type(KeyType::ECDSA_HASH160);
        let refusal = check(&both_wrong, document_scope());
        assert!(
            matches!(
                refusal.as_ref().err().and_then(signature_refusal_of),
                Some(SignatureError::InvalidIdentityPublicKeyTypeError(_))
            ),
            "{refusal:?}"
        );
    }

    #[test]
    fn should_report_contract_bounds_before_expiry_as_drive_does() {
        let expired_elsewhere = bound_elsewhere(1).with_limits(None, Some(1));
        assert!(matches!(
            signature_error(check(&expired_elsewhere, document_scope())),
            SignatureError::ContractBoundedKeyOutOfBoundsError(_)
        ));
        let expired_bound_here = bound_key(
            2,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract {
                id: Identifier::from(CONTRACT),
            }),
        )
        .with_limits(None, Some(1));
        assert!(matches!(
            signature_error(check(&expired_bound_here, SigningScope::NonBatch)),
            SignatureError::ContractBoundedKeyNonBatchError(_)
        ));
    }

    #[test]
    fn should_require_a_create_key_to_cover_the_types_the_create_consumes() {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
        use dpp::version::PlatformVersion;

        let dpns = load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
            .expect("bundled DPNS contract");
        let domain = dpns.document_type_for_name("domain").expect("domain");
        assert!(domain.consumable_document_type_names().contains("preorder"));
        let scope = SigningScope::DocumentCreate {
            document_type: domain,
        };
        let bound_to_domain = bound_key(
            1,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContractDocumentType {
                id: dpns.id(),
                document_type_name: "domain".to_string(),
            }),
        );
        let bound_to_dpns = bound_key(
            2,
            SecurityLevel::HIGH,
            Some(ContractBounds::SingleContract { id: dpns.id() }),
        );

        assert_eq!(
            scope.bounds_check(&bound_to_domain),
            BatchedTransitionBoundsCheck::Denied
        );
        assert!(matches!(
            signature_error(check(&bound_to_domain, scope)),
            SignatureError::ContractBoundedKeyOutOfBoundsError(_)
        ));
        assert_eq!(
            scope.bounds_check(&bound_to_dpns),
            BatchedTransitionBoundsCheck::Allowed
        );
        check(&bound_to_dpns, scope).expect("a key bound to the whole contract signs the create");
        let picked = usable_signing_key(
            &identity(vec![bound_to_domain, bound_to_dpns]),
            &KeyFilter(|_| true),
            Purpose::AUTHENTICATION,
            &LEVELS,
            &TYPES,
            scope,
        )
        .unwrap()
        .map(|key| key.id());
        assert_eq!(
            picked,
            Some(2),
            "the key bound to the created type only is skipped"
        );
    }
}
