//! JNI exports for creating an entry of any private document type (protocol version 15):
//! bridges `platform_wallet_manager_private_document_recipient_keys` (released with
//! `platform_wallet_private_document_recipient_keys_free` before returning),
//! `platform_wallet_manager_private_document_create_fee` and
//! `platform_wallet_manager_create_private_document_entry`.
//!
//! Which recipient keys qualify, which of the sender's keys seals, the schema check, the fee
//! and the outcome are `rs-platform-wallet`'s. These exports convert the Java arguments to the
//! C ones (refusing a value the C type cannot hold rather than truncating it), call once, and
//! copy the result into a Java array, so no native memory outlives the call.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.PrivateDocumentCreateNative`.

use crate::private_documents::read_32;
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbyteArray, jint, jlong, jlongArray};
use jni::JNIEnv;
use platform_wallet_ffi::error::PlatformWalletFFIResultCode;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::private_document_create::{
    platform_wallet_manager_private_document_create_fee,
    platform_wallet_manager_private_document_recipient_keys,
    platform_wallet_private_document_recipient_keys_free, PrivateDocumentRecipientKeyFFI,
    PrivateDocumentRecipientKeysFFI,
};
use std::ffi::CString;
use std::ptr;

#[cfg(feature = "shielded")]
use platform_wallet_ffi::private_document_create::{
    platform_wallet_manager_create_private_document_entry, PrivateDocumentEntryCreatedFFI,
};
#[cfg(feature = "shielded")]
use rs_sdk_ffi::MnemonicResolverHandle;

/// The length of the array the create export returns: the commitment (32 bytes) and the
/// sender key id (`u32`, little-endian).
#[cfg(feature = "shielded")]
const CREATED_LEN: usize = 36;

/// A Java string's UTF-16 code units, decoded strictly, as a C string: an unpaired surrogate
/// (which no UTF-8 string can carry) is refused rather than replaced, and the NUL check runs on
/// the decoded text, so each refusal names its real cause whatever the VM's `GetStringUTFChars`
/// encoding would have been.
fn c_string_from_utf16(units: &[u16]) -> Result<CString, &'static str> {
    let decoded =
        String::from_utf16(units).map_err(|_| "is not valid UTF-16 (an unpaired surrogate)")?;
    CString::new(decoded).map_err(|_| "holds a NUL character")
}

/// The UTF-16 code units of the non-null Java string `string`, read with `GetStringLength` and
/// `GetStringRegion`; `None` (with any pending exception cleared) when the VM refuses.
fn java_string_units(env: &mut JNIEnv, string: &JString) -> Option<Vec<u16>> {
    let raw = env.get_raw();
    // SAFETY: `raw` is this thread's live JNIEnv and `string` a valid, non-null local
    // reference; the region read stays within the length the VM reported.
    let units = unsafe {
        let functions = &**raw;
        let length = (functions.GetStringLength?)(raw, string.as_raw());
        let mut units = vec![0u16; usize::try_from(length).ok()?];
        (functions.GetStringRegion?)(raw, string.as_raw(), 0, length, units.as_mut_ptr());
        units
    };
    if env.exception_check().unwrap_or(true) {
        let _ = env.exception_clear();
        return None;
    }
    Some(units)
}

/// A required Java string as a C string; throws `InvalidParameter` and returns `None` for
/// null, an unreadable string, an unpaired surrogate or a NUL ([`c_string_from_utf16`]).
fn read_c_string(env: &mut JNIEnv, string: &JString, field: &str) -> Option<CString> {
    if string.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let Some(units) = java_string_units(env, string) else {
        throw_sdk_exception(env, 1, &format!("{field} was not a readable string"));
        return None;
    };
    match c_string_from_utf16(&units) {
        Ok(value) => Some(value),
        Err(reason) => {
            throw_sdk_exception(env, 1, &format!("{field} {reason}"));
            None
        }
    }
}

/// `keys` as the flat array Kotlin decodes: `[keyId0, purpose0, keyId1, purpose1, ...]`.
fn recipient_keys_flat(keys: &[PrivateDocumentRecipientKeyFFI]) -> Vec<i64> {
    keys.iter()
        .flat_map(|key| [i64::from(key.key_id), i64::from(key.purpose)])
        .collect()
}

/// `created` as the array Kotlin decodes: the commitment, then the sender key id as a
/// little-endian `u32`.
#[cfg(feature = "shielded")]
fn created_bytes(created: &PrivateDocumentEntryCreatedFFI) -> [u8; CREATED_LEN] {
    let mut out = [0u8; CREATED_LEN];
    out[..32].copy_from_slice(&created.commitment);
    out[32..].copy_from_slice(&created.sender_key_id.to_le_bytes());
    out
}

/// The keys of the identity `identityId` (32 bytes), as the network proves it now, that an
/// entry of any private document type may be sealed to (bridges
/// `platform_wallet_manager_private_document_recipient_keys`). Returns them as
/// `[keyId, purpose]` pairs flattened into one array (purpose 1 `ENCRYPTION`, 2 `DECRYPTION`,
/// as dpp numbers it), empty for an identity with no such key, or null after throwing.
/// Blocking (network).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PrivateDocumentCreateNative_recipientKeys(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    identity_id: JByteArray,
) -> jlongArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(identity_id) = read_32(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let mut keys = PrivateDocumentRecipientKeysFFI::empty();
        let result = unsafe {
            platform_wallet_manager_private_document_recipient_keys(
                manager_handle as Handle,
                identity_id.as_ptr(),
                &mut keys,
            )
        };
        // The export leaves the empty value on every failure, so the keys are copied only on
        // success and freed on both paths.
        let flat = if result.code == PlatformWalletFFIResultCode::Success
            && !keys.keys.is_null()
            && keys.count > 0
        {
            // SAFETY: filled by the call above with `count` keys, freed only below.
            recipient_keys_flat(unsafe { std::slice::from_raw_parts(keys.keys, keys.count) })
        } else {
            Vec::new()
        };
        unsafe { platform_wallet_private_document_recipient_keys_free(&mut keys) };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Ok(len) = jint::try_from(flat.len()) else {
            throw_sdk_exception(env, 1, "too many recipient keys to return");
            return ptr::null_mut();
        };
        let Ok(array) = env.new_long_array(len) else {
            return ptr::null_mut();
        };
        if env.set_long_array_region(&array, 0, &flat).is_err() {
            return ptr::null_mut();
        }
        array.into_raw()
    })
}

/// The fee, in credits, a create of `entryCount` entries of the private type
/// `documentTypeName` of `contractId` (32 bytes) reserves when it spends one or two notes
/// (bridges `platform_wallet_manager_private_document_create_fee`): the least it pays.
/// Returns the unsigned fee as its two's-complement `long`, or 0 after throwing. Blocking
/// (network).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PrivateDocumentCreateNative_createFee(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    contract_id: JByteArray,
    document_type_name: JString,
    entry_count: jint,
) -> jlong {
    guard(&mut env, 0, |env| {
        let Ok(entry_count) = u32::try_from(entry_count) else {
            throw_sdk_exception(env, 1, "entryCount must not be negative");
            return 0;
        };
        let Some(contract_id) = read_32(env, &contract_id, "contractId") else {
            return 0;
        };
        let Some(name) = read_c_string(env, &document_type_name, "documentTypeName") else {
            return 0;
        };
        let mut fee: u64 = 0;
        let result = unsafe {
            platform_wallet_manager_private_document_create_fee(
                manager_handle as Handle,
                contract_id.as_ptr(),
                name.as_ptr(),
                entry_count,
                &mut fee,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        fee as jlong
    })
}

/// Create one entry of the private type `documentTypeName` of `contractId` from the wallet's
/// identity `senderIdentityId` to the key `recipientKeyId` of `recipientIdentityId`, paying
/// from `account`'s shielded notes of the wallet `walletId` (bridges
/// `platform_wallet_manager_create_private_document_entry`, type 29). Ids are 32 bytes.
///
/// `authMode` is the FFI's byte (0 anonymous, 1 deniable MAC); the FFI refuses any other.
/// `propertiesJson` is a JSON object of the entry's properties, which Rust checks against the
/// type's schema. `resolverHandle` is the manager's `MnemonicResolverHandle`, which supplies
/// the sender's encryption key and the spend authority.
///
/// Returns the 36 bytes of `created_bytes`, or null after throwing. Errors are thrown as the
/// other note spends throw them: the spend-unconfirmed code means the entry may already be in
/// the store and must not be created again. Blocking (Halo 2 proof, network).
#[cfg(feature = "shielded")]
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PrivateDocumentCreateNative_createEntry(
    mut env: JNIEnv,
    _class: JClass,
    manager_handle: jlong,
    wallet_id: JByteArray,
    resolver_handle: jlong,
    account: jint,
    contract_id: JByteArray,
    document_type_name: JString,
    sender_identity_id: JByteArray,
    recipient_identity_id: JByteArray,
    recipient_key_id: jlong,
    auth_mode: jint,
    properties_json: JString,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Ok(account) = u32::try_from(account) else {
            throw_sdk_exception(env, 1, "account must not be negative");
            return ptr::null_mut();
        };
        let Ok(recipient_key_id) = u32::try_from(recipient_key_id) else {
            throw_sdk_exception(
                env,
                1,
                &format!("recipientKeyId must be 0..=4294967295, got {recipient_key_id}"),
            );
            return ptr::null_mut();
        };
        // The FFI decides which modes exist; this only refuses a value its byte cannot hold
        let Ok(auth_mode) = u8::try_from(auth_mode) else {
            throw_sdk_exception(
                env,
                1,
                &format!("authMode must be 0..=255, got {auth_mode}"),
            );
            return ptr::null_mut();
        };
        let Some(wallet_id) = read_32(env, &wallet_id, "walletId") else {
            return ptr::null_mut();
        };
        let Some(contract_id) = read_32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(sender) = read_32(env, &sender_identity_id, "senderIdentityId") else {
            return ptr::null_mut();
        };
        let Some(recipient) = read_32(env, &recipient_identity_id, "recipientIdentityId") else {
            return ptr::null_mut();
        };
        let Some(name) = read_c_string(env, &document_type_name, "documentTypeName") else {
            return ptr::null_mut();
        };
        let Some(properties) = read_c_string(env, &properties_json, "propertiesJson") else {
            return ptr::null_mut();
        };
        // Allocated before anything is paid: once the wallet reports success there is no
        // allocation left that could fail and hide the entry's commitment.
        let Ok(out) = env.new_byte_array(CREATED_LEN as jint) else {
            return ptr::null_mut();
        };
        let mut created = PrivateDocumentEntryCreatedFFI::default();
        let result = unsafe {
            platform_wallet_manager_create_private_document_entry(
                manager_handle as Handle,
                wallet_id.as_ptr(),
                resolver_handle as *mut MnemonicResolverHandle,
                account,
                contract_id.as_ptr(),
                name.as_ptr(),
                sender.as_ptr(),
                recipient.as_ptr(),
                recipient_key_id,
                auth_mode,
                properties.as_ptr(),
                &mut created,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let bytes = created_bytes(&created).map(|byte| byte as i8);
        if env.set_byte_array_region(&out, 0, &bytes).is_err() {
            return ptr::null_mut();
        }
        out.into_raw()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_flatten_the_recipient_keys_into_id_and_purpose_pairs() {
        let keys = [
            PrivateDocumentRecipientKeyFFI {
                key_id: 1,
                purpose: 1,
            },
            PrivateDocumentRecipientKeyFFI {
                key_id: u32::MAX,
                purpose: 2,
            },
        ];
        assert_eq!(
            recipient_keys_flat(&keys),
            vec![1, 1, i64::from(u32::MAX), 2]
        );
        assert!(recipient_keys_flat(&[]).is_empty());
    }

    #[test]
    fn should_decode_a_java_string_strictly() {
        // U+1F600 is a surrogate pair in UTF-16
        let emoji: Vec<u16> = "\u{1F600}".encode_utf16().collect();
        assert_eq!(
            c_string_from_utf16(&emoji).expect("a valid pair"),
            CString::new("\u{1F600}").expect("no NUL")
        );
        let ascii: Vec<u16> = "{\"note\":\"hi\"}".encode_utf16().collect();
        assert_eq!(
            c_string_from_utf16(&ascii).expect("ASCII"),
            CString::new("{\"note\":\"hi\"}").expect("no NUL")
        );
        // A lone high or low surrogate is refused, not replaced
        assert_eq!(
            c_string_from_utf16(&[0x61, 0xD83D, 0x62]),
            Err("is not valid UTF-16 (an unpaired surrogate)")
        );
        assert_eq!(
            c_string_from_utf16(&[0xDE00]),
            Err("is not valid UTF-16 (an unpaired surrogate)")
        );
        // A NUL is refused as a NUL, also beside a valid pair
        let mut with_nul = emoji.clone();
        with_nul.extend([0x00, 0x62]);
        assert_eq!(c_string_from_utf16(&with_nul), Err("holds a NUL character"));
    }

    #[cfg(feature = "shielded")]
    #[test]
    fn should_write_the_commitment_then_the_sender_key_id_little_endian() {
        let created = PrivateDocumentEntryCreatedFFI {
            commitment: [0xC3; 32],
            sender_key_id: 0x0102_0304,
        };
        let bytes = created_bytes(&created);
        assert_eq!(&bytes[..32], &[0xC3; 32]);
        assert_eq!(&bytes[32..], &[0x04, 0x03, 0x02, 0x01]);
    }
}
