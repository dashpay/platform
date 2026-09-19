//! JNI bridge for decode-any-kind state transition parsing — a thin
//! marshaler over `platform_wallet_parse_state_transition` (single Rust FFI
//! entry point, per the `packages/kotlin-sdk/CLAUDE.md` boundary rule).
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.TransactionsNative
//! .parseStateTransition`, driven by
//! `org.dashfoundation.dashsdk.identity.StateTransitionParser` — the
//! Android analog of Swift's `ManagedPlatformWallet.parseStateTransition`.
//!
//! ## Result convention
//!
//! `ParsedStateTransitionFFI` is a pointer graph (owned C strings and
//! arrays per kind). Rather than exposing a native handle plus N accessors
//! across JNI, the whole result is copied ONCE into a packed big-endian
//! blob (the convention `tx_decode.rs` and `pubkey_rows.rs` use) and every
//! Rust allocation is released via `platform_wallet_parse_state_transition_free`
//! before the export returns. Errors throw `DashSDKException` through the
//! shared `take_pwffi_error` mapping.
//!
//! ## BLOB layout (big-endian; keep in sync with `StateTransitionParser.parseBlob`)
//!
//! ```text
//! u8     kind                 (PARSED_STATE_TRANSITION_KIND_*)
//! u16    kind_name_len, u8[kind_name_len] kind_name (UTF-8)
//! u8     has_owner_id;  if 1: u8[32] owner_id
//! u8     is_signed
//! u32    serialized_len, u8[serialized_len] serialized (tagged DPP bytes)
//! kind 1 (IdentityUpdate):
//!   u8[32] identity_id
//!   u32    add_count; repeat add_count times (same field order as
//!          `IdentityPubkeyCodec`):
//!     u32 key_id, u8 key_type, u8 purpose, u8 security_level, u8 read_only,
//!     u8  contract_bounds_kind, u16 data_len, u8[data_len] data,
//!     if contract_bounds_kind != 0: u8[32] contract_bounds_id,
//!     if contract_bounds_kind == 2: u16 doc_type_len, u8[doc_type_len] doc_type,
//!     u8  limits_flags (bit 0: u64 total_budget follows, bit 1: u64 expires_at follows)
//!   u32    disable_count, u32[disable_count] disable_ids
//! kind 2 (Batch):
//!   u8[32] owner_id
//!   u32    count; repeat count times:
//!     u8  family (0 document, 1 token), u8[32] data_contract_id,
//!     u16 action_len, u8[action_len] action,
//!     family 0: u16 doc_type_len, u8[doc_type_len] doc_type, u8[32] document_id
//!     family 1: u16 token_contract_position, u8[32] token_id
//!     u8  has_amount;    if 1: u64 amount
//!     u8  has_recipient; if 1: u8[32] recipient_id
//! kind 3 (IdentityCreditTransfer):
//!   u8[32] identity_id, u8[32] recipient_id, u64 amount
//! kind 4 / 5 (DataContractCreate / DataContractUpdate):
//!   u8[32] contract_id, u8[32] owner_id,
//!   u32    count; repeat count times: u16 len, u8[len] document_type_name
//! kind 255 (other): nothing further
//! ```

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::jbyteArray;
use jni::JNIEnv;
use platform_wallet_ffi::identity_update::ParsedIdentityUpdatePublicKeyFFI;
use platform_wallet_ffi::parse_state_transition::{
    platform_wallet_parse_state_transition, platform_wallet_parse_state_transition_free,
    ParsedBatchedTransitionFFI, ParsedStateTransitionFFI,
    PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT, PARSED_STATE_TRANSITION_KIND_BATCH,
    PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER,
    PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE,
    PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE,
    PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE,
};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::ptr;

/// Append `u16 len + bytes` for a required C string. A null pointer or an
/// over-long string is a bug in the producing FFI, so it is reported as an
/// error rather than silently encoded as empty.
///
/// # Safety
/// `ptr` must be null or a valid NUL-terminated C string.
unsafe fn push_cstr(blob: &mut Vec<u8>, ptr: *const c_char, what: &str) -> Result<(), String> {
    if ptr.is_null() {
        return Err(format!("{what} is null"));
    }
    let bytes = CStr::from_ptr(ptr).to_bytes();
    let len = u16::try_from(bytes.len()).map_err(|_| format!("{what} exceeds u16 length"))?;
    blob.extend_from_slice(&len.to_be_bytes());
    blob.extend_from_slice(bytes);
    Ok(())
}

/// # Safety
/// `key` must be a row owned by a live `ParsedIdentityUpdateFFI`.
unsafe fn push_parsed_public_key(
    blob: &mut Vec<u8>,
    key: &ParsedIdentityUpdatePublicKeyFFI,
) -> Result<(), String> {
    blob.extend_from_slice(&key.key_id.to_be_bytes());
    blob.push(key.key_type);
    blob.push(key.purpose);
    blob.push(key.security_level);
    blob.push(u8::from(key.read_only));
    blob.push(key.contract_bounds_kind);
    let data_len = u16::try_from(key.data_len).map_err(|_| "key data exceeds u16 length")?;
    blob.extend_from_slice(&data_len.to_be_bytes());
    if key.data_len > 0 {
        if key.data_ptr.is_null() {
            return Err("key data pointer is null".to_string());
        }
        blob.extend_from_slice(std::slice::from_raw_parts(key.data_ptr, key.data_len));
    }
    if key.contract_bounds_kind != 0 {
        blob.extend_from_slice(&key.contract_bounds_id);
    }
    if key.contract_bounds_kind == 2 {
        push_cstr(
            blob,
            key.contract_bounds_document_type,
            "contract bounds document type",
        )?;
    }
    let flags = u8::from(key.has_total_budget) | (u8::from(key.has_expires_at) << 1);
    blob.push(flags);
    if key.has_total_budget {
        blob.extend_from_slice(&key.total_budget.to_be_bytes());
    }
    if key.has_expires_at {
        blob.extend_from_slice(&key.expires_at.to_be_bytes());
    }
    Ok(())
}

/// # Safety
/// `row` must be a row owned by a live `ParsedBatchFFI`.
unsafe fn push_batched_transition(
    blob: &mut Vec<u8>,
    row: &ParsedBatchedTransitionFFI,
) -> Result<(), String> {
    blob.push(row.family);
    blob.extend_from_slice(&row.data_contract_id);
    push_cstr(blob, row.action, "batched transition action")?;
    if row.family == PARSED_BATCHED_TRANSITION_FAMILY_DOCUMENT {
        push_cstr(blob, row.document_type, "batched document type")?;
        blob.extend_from_slice(&row.document_id);
    } else {
        blob.extend_from_slice(&row.token_contract_position.to_be_bytes());
        blob.extend_from_slice(&row.token_id);
    }
    blob.push(u8::from(row.has_amount));
    if row.has_amount {
        blob.extend_from_slice(&row.amount.to_be_bytes());
    }
    blob.push(u8::from(row.has_recipient));
    if row.has_recipient {
        blob.extend_from_slice(&row.recipient_id);
    }
    Ok(())
}

fn push_count(blob: &mut Vec<u8>, count: usize, what: &str) -> Result<(), String> {
    let count = u32::try_from(count).map_err(|_| format!("{what} count exceeds u32"))?;
    blob.extend_from_slice(&count.to_be_bytes());
    Ok(())
}

/// Copy a `ParsedStateTransitionFFI` pointer graph into the packed blob.
///
/// # Safety
/// `parsed` must be a successful, not yet freed result of
/// `platform_wallet_parse_state_transition`.
pub(crate) unsafe fn encode_parsed_state_transition(
    parsed: &ParsedStateTransitionFFI,
) -> Result<Vec<u8>, String> {
    let mut blob = Vec::with_capacity(256 + parsed.serialized_len);
    blob.push(parsed.kind);
    push_cstr(&mut blob, parsed.kind_name, "kind name")?;
    blob.push(u8::from(parsed.has_owner_id));
    if parsed.has_owner_id {
        blob.extend_from_slice(&parsed.owner_id);
    }
    blob.push(u8::from(parsed.is_signed));
    push_count(&mut blob, parsed.serialized_len, "serialized")?;
    if parsed.serialized_len > 0 {
        if parsed.serialized.is_null() {
            return Err("serialized pointer is null".to_string());
        }
        blob.extend_from_slice(std::slice::from_raw_parts(
            parsed.serialized,
            parsed.serialized_len,
        ));
    }

    match parsed.kind {
        PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE => {
            let update = &parsed.identity_update;
            blob.extend_from_slice(&update.identity_id);
            push_count(&mut blob, update.add_public_keys_count, "added keys")?;
            if update.add_public_keys_count > 0 {
                if update.add_public_keys.is_null() {
                    return Err("added keys pointer is null".to_string());
                }
                for key in
                    std::slice::from_raw_parts(update.add_public_keys, update.add_public_keys_count)
                {
                    push_parsed_public_key(&mut blob, key)?;
                }
            }
            push_count(
                &mut blob,
                update.disable_public_key_ids_count,
                "disabled keys",
            )?;
            if update.disable_public_key_ids_count > 0 {
                if update.disable_public_key_ids.is_null() {
                    return Err("disabled key ids pointer is null".to_string());
                }
                for id in std::slice::from_raw_parts(
                    update.disable_public_key_ids,
                    update.disable_public_key_ids_count,
                ) {
                    blob.extend_from_slice(&id.to_be_bytes());
                }
            }
        }
        PARSED_STATE_TRANSITION_KIND_BATCH => {
            let batch = &parsed.batch;
            blob.extend_from_slice(&batch.owner_id);
            push_count(&mut blob, batch.transitions_count, "batched transitions")?;
            if batch.transitions_count > 0 {
                if batch.transitions.is_null() {
                    return Err("batched transitions pointer is null".to_string());
                }
                for row in std::slice::from_raw_parts(batch.transitions, batch.transitions_count) {
                    push_batched_transition(&mut blob, row)?;
                }
            }
        }
        PARSED_STATE_TRANSITION_KIND_CREDIT_TRANSFER => {
            let transfer = &parsed.credit_transfer;
            blob.extend_from_slice(&transfer.identity_id);
            blob.extend_from_slice(&transfer.recipient_id);
            blob.extend_from_slice(&transfer.amount.to_be_bytes());
        }
        PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_CREATE
        | PARSED_STATE_TRANSITION_KIND_DATA_CONTRACT_UPDATE => {
            let contract = &parsed.data_contract;
            blob.extend_from_slice(&contract.contract_id);
            blob.extend_from_slice(&contract.owner_id);
            push_count(
                &mut blob,
                contract.document_type_names_count,
                "document type names",
            )?;
            if contract.document_type_names_count > 0 {
                if contract.document_type_names.is_null() {
                    return Err("document type names pointer is null".to_string());
                }
                for name in std::slice::from_raw_parts(
                    contract.document_type_names,
                    contract.document_type_names_count,
                ) {
                    push_cstr(&mut blob, *name, "document type name")?;
                }
            }
        }
        _ => {}
    }
    Ok(blob)
}

/// Decode `bytes` as any DPP state transition and return the packed blob
/// described in the module doc. Throws `DashSDKException` on
/// undecodable bytes.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_TransactionsNative_parseStateTransition(
    mut env: JNIEnv,
    _class: JClass,
    transition_bytes: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let bytes = match env.convert_byte_array(&transition_bytes) {
            Ok(b) => b,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "transitionBytes byte[] was null/invalid");
                return ptr::null_mut();
            }
        };
        if bytes.is_empty() {
            throw_sdk_exception(env, 1, "transitionBytes must not be empty");
            return ptr::null_mut();
        }

        let mut out = ParsedStateTransitionFFI::default();
        let result = unsafe {
            platform_wallet_parse_state_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };
        if take_pwffi_error(env, result) {
            unsafe { platform_wallet_parse_state_transition_free(&mut out) };
            return ptr::null_mut();
        }

        let encoded = unsafe { encode_parsed_state_transition(&out) };
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };

        match encoded {
            Ok(blob) => env
                .byte_array_from_slice(&blob)
                .map(|a| a.into_raw())
                .unwrap_or(ptr::null_mut()),
            Err(message) => {
                throw_sdk_exception(
                    env,
                    99,
                    &format!("parsed transition encode failed: {message}"),
                );
                ptr::null_mut()
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::identity::identity_public_key::contract_bounds::ContractBounds;
    use dpp::identity::{KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    use dpp::prelude::Identifier;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::batched_transition::token_transfer_transition::v0::TokenTransferTransitionV0;
    use dpp::state_transition::batch_transition::batched_transition::{
        BatchedTransition, TokenTransition,
    };
    use dpp::state_transition::batch_transition::token_base_transition::v0::TokenBaseTransitionV0;
    use dpp::state_transition::batch_transition::token_base_transition::TokenBaseTransition;
    use dpp::state_transition::batch_transition::{
        BatchTransition, BatchTransitionV1, TokenTransferTransition,
    };
    use dpp::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
    use dpp::state_transition::public_key_in_creation::v1::IdentityPublicKeyInCreationV1;
    use dpp::state_transition::StateTransition;

    /// A DashPay Connect session-key registration: one HIGH auth key bound
    /// to a contract group with a budget and an expiry, one key disabled.
    /// The same fixture `StateTransitionParserTest` decodes on the Kotlin
    /// side from the pinned hex below.
    fn identity_update_bytes() -> Vec<u8> {
        StateTransition::IdentityUpdate(
            IdentityUpdateTransitionV0 {
                signature: BinaryData::new(vec![]),
                signature_public_key_id: 0,
                identity_id: Identifier::from([0x11; 32]),
                revision: 7,
                nonce: 9,
                add_public_keys: vec![IdentityPublicKeyInCreationV1 {
                    id: 18,
                    key_type: KeyType::ECDSA_SECP256K1,
                    purpose: Purpose::AUTHENTICATION,
                    security_level: SecurityLevel::HIGH,
                    read_only: false,
                    data: BinaryData::new(vec![0x03; 33]),
                    signature: BinaryData::new(vec![]),
                    contract_bounds: Some(ContractBounds::ContractGroup {
                        id: Identifier::from([0x66; 32]),
                    }),
                    total_budget: Some(10_000_000_000),
                    expires_at: Some(1_800_000_000_000),
                }
                .into()],
                disable_public_keys: vec![4],
                user_fee_increase: 0,
            }
            .into(),
        )
        .serialize_to_bytes()
        .expect("fixture identity update serializes")
    }

    fn token_transfer_batch_bytes() -> Vec<u8> {
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([0x21; 32]),
            transitions: vec![BatchedTransition::Token(TokenTransition::Transfer(
                TokenTransferTransition::V0(TokenTransferTransitionV0 {
                    base: TokenBaseTransition::V0(TokenBaseTransitionV0 {
                        identity_contract_nonce: 4,
                        token_contract_position: 3,
                        data_contract_id: Identifier::from([0x42; 32]),
                        token_id: Identifier::from([0x77; 32]),
                        using_group_info: None,
                    }),
                    amount: 250,
                    recipient_id: Identifier::from([0x22; 32]),
                    public_note: None,
                    shared_encrypted_note: None,
                    private_encrypted_note: None,
                }),
            ))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![]),
        }))
        .serialize_to_bytes()
        .expect("fixture batch serializes")
    }

    fn parse_to_blob(bytes: &[u8]) -> Vec<u8> {
        let mut out = ParsedStateTransitionFFI::default();
        let result = unsafe {
            platform_wallet_parse_state_transition(bytes.as_ptr(), bytes.len(), &mut out)
        };
        assert_eq!(
            result.code,
            platform_wallet_ffi::error::PlatformWalletFFIResultCode::Success
        );
        let blob = unsafe { encode_parsed_state_transition(&out) }.expect("blob encodes");
        unsafe { platform_wallet_parse_state_transition_free(&mut out) };
        blob
    }

    struct Reader<'a>(&'a [u8]);
    impl<'a> Reader<'a> {
        fn take(&mut self, n: usize) -> &'a [u8] {
            let (head, tail) = self.0.split_at(n);
            self.0 = tail;
            head
        }
        fn u8(&mut self) -> u8 {
            self.take(1)[0]
        }
        fn u16(&mut self) -> u16 {
            u16::from_be_bytes(self.take(2).try_into().unwrap())
        }
        fn u32(&mut self) -> u32 {
            u32::from_be_bytes(self.take(4).try_into().unwrap())
        }
        fn u64(&mut self) -> u64 {
            u64::from_be_bytes(self.take(8).try_into().unwrap())
        }
        fn str16(&mut self) -> String {
            let len = self.u16() as usize;
            String::from_utf8(self.take(len).to_vec()).unwrap()
        }
    }

    #[test]
    fn identity_update_blob_round_trips_and_is_pinned_for_kotlin() {
        let bytes = identity_update_bytes();
        let blob = parse_to_blob(&bytes);

        let mut r = Reader(&blob);
        assert_eq!(r.u8(), PARSED_STATE_TRANSITION_KIND_IDENTITY_UPDATE);
        assert_eq!(r.str16(), "IdentityUpdate");
        assert_eq!(r.u8(), 1);
        assert_eq!(r.take(32), &[0x11; 32]);
        assert_eq!(r.u8(), 0, "unsigned");
        let serialized_len = r.u32() as usize;
        assert_eq!(r.take(serialized_len), bytes.as_slice());
        assert_eq!(r.take(32), &[0x11; 32]);
        assert_eq!(r.u32(), 1);
        assert_eq!(r.u32(), 18);
        assert_eq!(r.u8(), 0); // key type
        assert_eq!(r.u8(), 0); // purpose
        assert_eq!(r.u8(), 2); // HIGH
        assert_eq!(r.u8(), 0); // read only
        assert_eq!(r.u8(), 3); // ContractGroup
        assert_eq!(r.u16(), 33);
        assert_eq!(r.take(33), &[0x03; 33]);
        assert_eq!(r.take(32), &[0x66; 32]);
        assert_eq!(r.u8(), 0b11);
        assert_eq!(r.u64(), 10_000_000_000);
        assert_eq!(r.u64(), 1_800_000_000_000);
        assert_eq!(r.u32(), 1);
        assert_eq!(r.u32(), 4);
        assert!(r.0.is_empty(), "trailing bytes");

        assert_eq!(
            blob, IDENTITY_UPDATE_GOLDEN,
            "identity-update blob drifted from the golden shared with StateTransitionParserTest"
        );
    }

    #[test]
    fn token_transfer_batch_blob_round_trips_and_is_pinned_for_kotlin() {
        let bytes = token_transfer_batch_bytes();
        let blob = parse_to_blob(&bytes);

        let mut r = Reader(&blob);
        assert_eq!(r.u8(), PARSED_STATE_TRANSITION_KIND_BATCH);
        assert_eq!(r.str16(), "DocumentsBatch([TokenTransfer])");
        assert_eq!(r.u8(), 1);
        assert_eq!(r.take(32), &[0x21; 32]);
        assert_eq!(r.u8(), 0);
        let serialized_len = r.u32() as usize;
        assert_eq!(r.take(serialized_len), bytes.as_slice());
        assert_eq!(r.take(32), &[0x21; 32]);
        assert_eq!(r.u32(), 1);
        assert_eq!(r.u8(), 1); // token family
        assert_eq!(r.take(32), &[0x42; 32]);
        assert_eq!(r.str16(), "Transfer");
        assert_eq!(r.u16(), 3);
        assert_eq!(r.take(32), &[0x77; 32]);
        assert_eq!(r.u8(), 1);
        assert_eq!(r.u64(), 250);
        assert_eq!(r.u8(), 1);
        assert_eq!(r.take(32), &[0x22; 32]);
        assert!(r.0.is_empty(), "trailing bytes");

        assert_eq!(
            blob, TOKEN_TRANSFER_GOLDEN,
            "token-transfer blob drifted from the golden shared with StateTransitionParserTest"
        );
    }

    /// The checked-in golden blobs, shared byte-for-byte with the Kotlin
    /// decoder test (`StateTransitionParserTest`). Referenced from the single
    /// canonical copy in the Kotlin SDK's test resources so the two cannot
    /// drift, the way `pubkey_rows.rs` anchors its registration golden.
    const IDENTITY_UPDATE_GOLDEN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/test/resources/golden/parsed_identity_update_v1.bin"
    ));
    const TOKEN_TRANSFER_GOLDEN: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/test/resources/golden/parsed_token_transfer_batch_v1.bin"
    ));
}
