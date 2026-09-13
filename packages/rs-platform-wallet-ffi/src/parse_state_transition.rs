//! FFI parser for raw DPP state transitions handed to the wallet by a dApp
//! (DashConnect `dash-st:` links / QRs).
//!
//! The wallet must never sign opaque bytes a web page hands it. This module
//! only reads the *intent* out of the payload so the app can show it to the
//! user; after approval the app rebuilds and signs the operation through the
//! normal wallet path (`platform_wallet_token_purchase` /
//! `platform_wallet_update_identity_with_signer`). There is deliberately no
//! "sign these transition bytes" entry point here or anywhere else in this
//! crate.
//!
//! One umbrella parser (rather than one probe per transition kind) so the
//! app can branch on the returned `kind` discriminant instead of driving
//! control flow off "expected X, got Y" errors: DashConnect key registration
//! arrives as an `IdentityUpdateTransition`, while a dApp token purchase
//! (e.g. Yappr) arrives as a `BatchTransition` carrying a single
//! `TokenDirectPurchase` — both come through the same `dash-st:` channel and
//! are only distinguishable after deserialization.

use std::borrow::Cow;
use std::slice;

use dpp::serialization::PlatformDeserializable;
use dpp::state_transition::batch_transition::accessors::DocumentsBatchTransitionAccessorsV0;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
use dpp::state_transition::batch_transition::token_base_transition::token_base_transition_accessors::TokenBaseTransitionAccessors;
use dpp::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::v0_methods::TokenDirectPurchaseTransitionV0Methods;
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::{StateTransition, StateTransitionOwned};

use crate::check_ptr;
use crate::error::*;
use crate::identity_update::{
    platform_wallet_parse_identity_update_transition_free, project_parsed_identity_update,
    ParsedIdentityUpdateFFI, IDENTITY_UPDATE_VARIANT_TAG,
};
use crate::unwrap_result_or_return;

/// Positional bincode variant tag of `StateTransition::Batch`.
pub(crate) const BATCH_VARIANT_TAG: u8 = 2;

/// `ParsedStateTransitionFFI::kind`: nothing was parsed (default state).
pub const PARSED_STATE_TRANSITION_KIND_NONE: u8 = 0;
/// `ParsedStateTransitionFFI::kind`: `identity_update` is populated.
pub const PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE: u8 = 1;
/// `ParsedStateTransitionFFI::kind`: `token_direct_purchase` is populated.
pub const PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE: u8 = 2;

/// Variant tags tried when the payload appears to use a tagless framing.
/// `IdentityUpdate` first: it is the framing Yappr has actually been observed
/// to send tagless; `Batch` payloads have so far arrived properly tagged.
const TAGLESS_FRAMING_CANDIDATES: &[(u8, &str)] = &[
    (IDENTITY_UPDATE_VARIANT_TAG, "IdentityUpdate"),
    (BATCH_VARIANT_TAG, "Batch"),
];

/// Owned C representation of the inspectable parts of a token direct
/// purchase carried by a parsed `BatchTransition`.
///
/// Plain old data — no heap allocations, so nothing beyond the containing
/// [`ParsedStateTransitionFFI`] needs freeing.
///
/// Carries everything `platform_wallet_token_purchase` needs to rebuild the
/// purchase after user approval, plus what the user must see before
/// approving (`owner_id` — the identity that will be charged — and the
/// `token_id` the price is quoted for).
#[repr(C)]
#[derive(Default)]
pub struct ParsedTokenDirectPurchaseFFI {
    /// The identity whose credits pay for the purchase.
    pub owner_id: [u8; 32],
    /// The data contract defining the token.
    pub data_contract_id: [u8; 32],
    /// The token being bought.
    pub token_id: [u8; 32],
    /// Position of the token within the contract.
    pub token_contract_position: u16,
    /// How many tokens the dApp asks to buy.
    pub token_count: u64,
    /// Credits the owner would agree to pay in total.
    pub total_agreed_price: u64,
}

/// Owned C representation of one parsed state transition, discriminated by
/// `kind`. Exactly one payload field is populated; the other stays in its
/// zeroed default state. Must be released via
/// [`platform_wallet_parse_state_transition_free`] regardless of `kind`
/// (freeing a default / token-purchase value is a safe no-op).
#[repr(C)]
#[derive(Default)]
pub struct ParsedStateTransitionFFI {
    /// One of the `PARSED_STATE_TRANSITION_KIND_*` constants.
    pub kind: u8,
    /// Populated when `kind == PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE`.
    pub identity_update: ParsedIdentityUpdateFFI,
    /// Populated when
    /// `kind == PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE`.
    pub token_direct_purchase: ParsedTokenDirectPurchaseFFI,
}

/// Deserializes `bytes` as a `StateTransition`, tolerating both normal
/// tagged DPP framing and the tagless framing Yappr sends, where the
/// positional bincode enum variant tag has to be prepended first.
///
/// A leading known variant tag usually means the payload is already framed
/// as a state transition, and a tagless body needs one of the `candidates`
/// tags prepended. Neither test is conclusive — a tagless body can start
/// with a tag byte by coincidence — so the likelier framing is only tried
/// first, and the others are still tried before the payload is rejected.
pub(crate) fn deserialize_transition_with_flexible_framing(
    bytes: &[u8],
    candidates: &[(u8, &str)],
) -> Result<StateTransition, PlatformWalletFFIResult> {
    let leads_with_candidate_tag = bytes
        .first()
        .is_some_and(|first| candidates.iter().any(|(tag, _)| tag == first));

    let as_is: (Cow<'_, [u8]>, String) = (Cow::Borrowed(bytes), "as-is".to_string());
    let prepended = candidates.iter().map(|(tag, name)| {
        let mut prefixed = Vec::with_capacity(bytes.len() + 1);
        prefixed.push(*tag);
        prefixed.extend_from_slice(bytes);
        (
            Cow::Owned(prefixed),
            format!("{name} variant tag prepended"),
        )
    });

    let mut attempts: Vec<(Cow<'_, [u8]>, String)> = Vec::with_capacity(candidates.len() + 1);
    if leads_with_candidate_tag {
        attempts.push(as_is);
        attempts.extend(prepended);
    } else {
        attempts.extend(prepended);
        attempts.push(as_is);
    }

    let mut failures: Vec<String> = Vec::with_capacity(attempts.len());
    for (payload, label) in &attempts {
        match StateTransition::deserialize_from_bytes(payload) {
            Ok(state_transition) => return Ok(state_transition),
            Err(error) => failures.push(format!("{label}: {error}")),
        }
    }

    Err(PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorDeserialization,
        format!(
            "Failed to deserialize state transition in any supported framing ({})",
            failures.join("; ")
        ),
    ))
}

/// Projects the single `TokenDirectPurchase` out of `batch`.
///
/// A batch may in principle carry several transitions, but anything other
/// than exactly one `TokenDirectPurchase` is rejected: the approval sheet
/// shows the user one purchase ("buy N of token T for P credits"), so a
/// multi-transition or mixed batch would execute more than the user
/// approved. The rebuild path (`platform_wallet_token_purchase`) can also
/// only reproduce a single purchase, so a wider batch could not be signed
/// faithfully even after approval.
///
/// `batch_description` is `StateTransition::name()` for the whole batch —
/// it lists the kinds of every inner transition, which makes the rejection
/// messages actionable without this module enumerating every batched
/// transition variant itself.
fn project_parsed_token_direct_purchase(
    batch: &BatchTransition,
    batch_description: &str,
) -> Result<ParsedTokenDirectPurchaseFFI, PlatformWalletFFIResult> {
    let transitions_len = batch.transitions_len();
    if transitions_len != 1 {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "Refusing to parse a batch of {transitions_len} transitions as a token \
                 purchase — the user can only approve exactly one TokenDirectPurchase \
                 ({batch_description})"
            ),
        ));
    }

    let Some(BatchedTransitionRef::Token(TokenTransition::DirectPurchase(purchase))) =
        batch.first_transition()
    else {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!(
                "Expected the batch to carry a single TokenDirectPurchase transition, \
                 got {batch_description}"
            ),
        ));
    };

    let base = purchase.base();

    // DirectPurchase is not group-gated and the rebuild path submits it
    // without group info, so a payload that smuggles group info in would be
    // approved as one thing and signed as another. Reject it instead.
    if base.using_group_info().is_some() {
        return Err(PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "TokenDirectPurchase carries group-action info, which the wallet's \
             rebuild-and-sign purchase path does not support"
                .to_string(),
        ));
    }

    Ok(ParsedTokenDirectPurchaseFFI {
        owner_id: batch.owner_id().to_buffer(),
        data_contract_id: base.data_contract_id().to_buffer(),
        token_id: base.token_id().to_buffer(),
        token_contract_position: base.token_contract_position(),
        token_count: purchase.token_count(),
        total_agreed_price: purchase.total_agreed_price(),
    })
}

/// Deserializes a raw DPP state transition (as carried by a DashConnect
/// `dash-st:` link / QR) into its inspectable parts, reporting which
/// supported kind it found in `out.kind` so the caller can branch without
/// probing kind-specific parsers and branching on their errors.
///
/// Supported kinds:
/// - `IdentityUpdateTransition` (DashConnect key registration) →
///   `PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE`, `identity_update`
///   populated exactly as by
///   `platform_wallet_parse_identity_update_transition`.
/// - `BatchTransition` carrying exactly one `TokenDirectPurchase` (a dApp
///   token purchase) →
///   `PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE`,
///   `token_direct_purchase` populated.
///
/// Every other transition kind — including batches that are empty, carry
/// several transitions, or carry anything other than a direct purchase — is
/// rejected with `ErrorInvalidParameter` naming what was found.
///
/// Accepts both normal tagged DPP state-transition bytes and Yappr's
/// tagless framing, where the positional bincode enum variant tag has to be
/// prepended before deserialization.
///
/// Does NOT sign and does NOT broadcast — the caller shows the parsed
/// intent to the user and rebuilds the operation through the normal signing
/// path (`platform_wallet_token_purchase` /
/// `platform_wallet_update_identity_with_signer`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_state_transition(
    transition_bytes: *const u8,
    transition_len: usize,
    out: *mut ParsedStateTransitionFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(transition_bytes);
    check_ptr!(out);

    *out = ParsedStateTransitionFFI::default();

    let bytes = slice::from_raw_parts(transition_bytes, transition_len);
    let transition = unwrap_result_or_return!(deserialize_transition_with_flexible_framing(
        bytes,
        TAGLESS_FRAMING_CANDIDATES,
    ));

    match &transition {
        StateTransition::IdentityUpdate(identity_update) => {
            (*out).identity_update =
                unwrap_result_or_return!(project_parsed_identity_update(identity_update));
            (*out).kind = PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE;
        }
        StateTransition::Batch(batch) => {
            (*out).token_direct_purchase = unwrap_result_or_return!(
                project_parsed_token_direct_purchase(batch, &transition.name())
            );
            (*out).kind = PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE;
        }
        other => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!(
                    "Unsupported state transition kind for dApp intent parsing: {}",
                    other.name()
                ),
            );
        }
    }

    PlatformWalletFFIResult::ok()
}

/// Frees a parsed transition previously returned by
/// [`platform_wallet_parse_state_transition`]. Safe to call for any `kind`,
/// including the zeroed default — only the identity-update payload owns
/// heap allocations.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_state_transition_free(
    out: *mut ParsedStateTransitionFFI,
) {
    if out.is_null() {
        return;
    }

    let parsed = &mut *out;
    platform_wallet_parse_identity_update_transition_free(&mut parsed.identity_update);
    *parsed = ParsedStateTransitionFFI::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::group::GroupStateTransitionInfo;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::token_burn_transition::v0::TokenBurnTransitionV0;
    use dpp::state_transition::batch_transition::token_direct_purchase_transition::v0::TokenDirectPurchaseTransitionV0;
    use dpp::state_transition::batch_transition::{
        BatchTransitionV1, TokenBurnTransition, TokenDirectPurchaseTransition,
    };
    use dpp::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;

    fn purchase_base() -> TokenBaseTransition {
        TokenBaseTransition::V0(TokenBaseTransitionV0 {
            identity_contract_nonce: 4,
            token_contract_position: 3,
            data_contract_id: Identifier::from([0x42; 32]),
            token_id: Identifier::from([0x77; 32]),
            using_group_info: None,
        })
    }

    fn direct_purchase() -> BatchedTransition {
        BatchedTransition::Token(TokenTransition::DirectPurchase(
            TokenDirectPurchaseTransition::V0(TokenDirectPurchaseTransitionV0 {
                base: purchase_base(),
                token_count: 100,
                total_agreed_price: 100_000_000,
            }),
        ))
    }

    fn batch_transition_bytes(transitions: Vec<BatchedTransition>) -> Vec<u8> {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([0x21; 32]),
            transitions,
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![0x88; 65]),
        }))
        .serialize_to_bytes()
        .expect("fixture batch serializes")
    }

    fn identity_update_transition_bytes() -> Vec<u8> {
        StateTransition::IdentityUpdate(
            IdentityUpdateTransitionV0 {
                signature: BinaryData::new(vec![0x99; 65]),
                signature_public_key_id: 3,
                identity_id: Identifier::from([0x11; 32]),
                revision: 7,
                nonce: 9,
                add_public_keys: vec![IdentityPublicKeyInCreationV0 {
                    id: 17,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    read_only: false,
                    data: BinaryData::new(vec![0x02; 33]),
                    signature: BinaryData::new(vec![0xaa; 65]),
                    contract_bounds: None,
                }
                .into()],
                disable_public_keys: vec![4, 8],
                user_fee_increase: 2,
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture identity update serializes")
    }

    fn parse(bytes: &[u8]) -> (PlatformWalletFFIResult, ParsedStateTransitionFFI) {
        let mut out = ParsedStateTransitionFFI::default();
        let result = unsafe {
            platform_wallet_parse_state_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };
        (result, out)
    }

    #[test]
    fn parses_a_tagged_token_direct_purchase_batch() {
        let bytes = batch_transition_bytes(vec![direct_purchase()]);
        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE);
        assert_eq!(out.token_direct_purchase.owner_id, [0x21; 32]);
        assert_eq!(out.token_direct_purchase.data_contract_id, [0x42; 32]);
        assert_eq!(out.token_direct_purchase.token_id, [0x77; 32]);
        assert_eq!(out.token_direct_purchase.token_contract_position, 3);
        assert_eq!(out.token_direct_purchase.token_count, 100);
        assert_eq!(out.token_direct_purchase.total_agreed_price, 100_000_000);
        // The unused payload stays in its default state.
        assert!(out.identity_update.add_public_keys.is_null());
        assert_eq!(out.identity_update.add_public_keys_count, 0);

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert_eq!(out.token_direct_purchase.token_count, 0);
    }

    #[test]
    fn parses_a_tagless_token_purchase_by_prepending_the_batch_tag() {
        let tagged = batch_transition_bytes(vec![direct_purchase()]);
        assert_eq!(
            tagged[0], BATCH_VARIANT_TAG,
            "StateTransition::Batch variant tag drifted"
        );
        let tagless = tagged[1..].to_vec();

        let (result, mut out) = parse(&tagless);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_TOKEN_DIRECT_PURCHASE);
        assert_eq!(out.token_direct_purchase.token_count, 100);

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
    }

    #[test]
    fn parses_an_identity_update_and_reports_its_kind() {
        let bytes = identity_update_transition_bytes();
        let (result, mut out) = parse(&bytes);

        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE);
        assert_eq!(out.identity_update.identity_id, [0x11; 32]);
        assert_eq!(out.identity_update.add_public_keys_count, 1);
        assert_eq!(out.identity_update.disable_public_key_ids_count, 2);
        let keys = unsafe {
            slice::from_raw_parts(
                out.identity_update.add_public_keys,
                out.identity_update.add_public_keys_count,
            )
        };
        assert_eq!(keys[0].key_id, 17);
        // The unused payload stays in its default state.
        assert_eq!(out.token_direct_purchase.token_count, 0);

        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        assert!(out.identity_update.add_public_keys.is_null());
        assert_eq!(out.identity_update.add_public_keys_count, 0);
    }

    #[test]
    fn rejects_a_batch_with_more_than_one_transition() {
        let bytes = batch_transition_bytes(vec![direct_purchase(), direct_purchase()]);
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn rejects_an_empty_batch() {
        let bytes = batch_transition_bytes(vec![]);
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn rejects_a_batch_whose_transition_is_not_a_direct_purchase() {
        let bytes = batch_transition_bytes(vec![BatchedTransition::Token(TokenTransition::Burn(
            TokenBurnTransition::V0(TokenBurnTransitionV0 {
                base: purchase_base(),
                burn_amount: 5,
                public_note: None,
            }),
        ))]);
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn rejects_a_group_gated_direct_purchase() {
        let bytes = batch_transition_bytes(vec![BatchedTransition::Token(
            TokenTransition::DirectPurchase(TokenDirectPurchaseTransition::V0(
                TokenDirectPurchaseTransitionV0 {
                    base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                        identity_contract_nonce: 4,
                        token_contract_position: 3,
                        data_contract_id: Identifier::from([0x42; 32]),
                        token_id: Identifier::from([0x77; 32]),
                        using_group_info: Some(GroupStateTransitionInfo {
                            group_contract_position: 1,
                            action_id: Identifier::from([0x55; 32]),
                            action_is_proposer: true,
                        }),
                    }),
                    token_count: 100,
                    total_agreed_price: 100_000_000,
                },
            )),
        )]);
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn rejects_an_unsupported_state_transition_kind() {
        let bytes = StateTransition::IdentityCreditTransfer(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([0x11; 32]),
                recipient_id: Identifier::from([0x22; 32]),
                amount: 1_000,
                nonce: 1,
                user_fee_increase: 0,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0x99; 65]),
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture credit transfer serializes");
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn rejects_malformed_state_transition_bytes() {
        let bytes = [0xde, 0xad, 0xbe, 0xef];
        let (result, out) = parse(&bytes);

        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorDeserialization
        );
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
    }

    #[test]
    fn freeing_a_default_value_is_a_safe_no_op() {
        let mut out = ParsedStateTransitionFFI::default();
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        assert_eq!(out.kind, PARSED_STATE_TRANSITION_KIND_NONE);
        // Double free of an already-freed value must also be safe.
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
    }
}
