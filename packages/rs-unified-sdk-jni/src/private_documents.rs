//! JNI export for reading a private document type's entries (protocol version 15): bridges
//! `platform_wallet_read_private_document_entries` and releases its result with
//! `platform_wallet_private_document_read_free` before returning.
//!
//! The read (which identities, which keys, how a payload reads, where to read on from) is
//! `rs-platform-wallet`'s. This export copies the C result into one JSON string, frees it,
//! and hands the string to Kotlin, so no native memory outlives the call. The cursor crosses
//! back as its typed fields; the key set and store fingerprints only cross back unchanged.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.PrivateDocumentsNative`.

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::error::PlatformWalletFFIResultCode;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::private_documents::{
    platform_wallet_private_document_read_free, platform_wallet_read_private_document_entries,
    PrivateDocumentCursorFFI, PrivateDocumentEntryFFI, PrivateDocumentReadFFI,
};
use rs_sdk_ffi::MnemonicResolverHandle;
use serde_json::{json, Value};
use std::ffi::{CStr, CString};
use std::fmt::Write;
use std::os::raw::c_char;
use std::ptr;

/// `bytes` as lowercase hex, written into one buffer.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Writing to a `String` cannot fail
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The cursor a read goes on from, given back field by field as a previous read returned it;
/// `Ok(None)` when both fingerprints are absent, which reads from the store's start. A cursor
/// needs both fingerprints, and its position and count cannot be negative.
fn cursor_from_parts(
    position: i64,
    proved_total_count: i64,
    has_proved_total_count: bool,
    key_set: Option<[u8; 32]>,
    store: Option<[u8; 32]>,
) -> Result<Option<PrivateDocumentCursorFFI>, &'static str> {
    let (key_set, store) = match (key_set, store) {
        (None, None) => return Ok(None),
        (Some(key_set), Some(store)) => (key_set, store),
        _ => return Err("a cursor needs both fromKeySet and fromStore"),
    };
    let (Ok(position), Ok(proved_total_count)) =
        (u64::try_from(position), u64::try_from(proved_total_count))
    else {
        return Err("the cursor's position and count must not be negative");
    };
    Ok(Some(PrivateDocumentCursorFFI {
        position,
        proved_total_count,
        key_set,
        store,
        has_proved_total_count,
    }))
}

/// A required 32-byte array; throws `InvalidParameter` and returns `None` otherwise. Shared
/// with `private_document_create`.
pub(crate) fn read_32(env: &mut JNIEnv, array: &JByteArray, field: &str) -> Option<[u8; 32]> {
    if array.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let bytes = match env.convert_byte_array(array) {
        Ok(bytes) => bytes,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was invalid"));
            return None;
        }
    };
    match <[u8; 32]>::try_from(bytes.as_slice()) {
        Ok(bytes) => Some(bytes),
        Err(_) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} must be 32 bytes, got {}", bytes.len()),
            );
            None
        }
    }
}

/// The C string `ptr` points to, as an owned string; `None` for null.
///
/// # Safety
/// `ptr` must be null or point to a NUL-terminated string.
unsafe fn optional_string(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(
            unsafe { CStr::from_ptr(ptr) }
                .to_string_lossy()
                .into_owned(),
        )
    }
}

/// One entry as the JSON object Kotlin decodes. `direction` and `auth_mode` cross as the
/// discriminants the FFI defines; Kotlin maps them and refuses one it does not know.
///
/// # Safety
/// `entry`'s string pointers must be null or valid NUL-terminated strings.
unsafe fn entry_json(entry: &PrivateDocumentEntryFFI) -> Value {
    json!({
        "position": entry.position,
        "block_height": entry.block_height,
        "block_time_ms": entry.block_time_ms,
        "properties_json": unsafe { optional_string(entry.properties_json) },
        "properties_refusal": unsafe { optional_string(entry.properties_refusal) },
        "wallet_identity_id": hex(&entry.wallet_identity_id),
        "wallet_key_id": entry.wallet_key_id,
        "recipient_identity_id": hex(&entry.recipient_identity_id),
        "claimed_sender_identity_id": entry
            .has_claimed_sender
            .then(|| hex(&entry.claimed_sender_identity_id)),
        "claimed_sender_key_id": entry.has_claimed_sender.then_some(entry.claimed_sender_key_id),
        "commitment": hex(&entry.commitment),
        "direction": entry.direction,
        "auth_mode": entry.auth_mode,
    })
}

/// `read` as the JSON object Kotlin decodes.
///
/// # Safety
/// `read` must be a result `platform_wallet_read_private_document_entries` filled and that
/// has not been freed.
unsafe fn read_json(read: &PrivateDocumentReadFFI) -> Value {
    let entries: &[PrivateDocumentEntryFFI] = if read.entries.is_null() || read.entry_count == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(read.entries, read.entry_count) }
    };
    let cursor = &read.cursor;
    json!({
        "entries": entries
            .iter()
            .map(|entry| unsafe { entry_json(entry) })
            .collect::<Vec<_>>(),
        "cursor": {
            "position": cursor.position,
            "proved_total_count": cursor
                .has_proved_total_count
                .then_some(cursor.proved_total_count),
            "key_set": hex(&cursor.key_set),
            "store": hex(&cursor.store),
        },
        "pages_read": read.pages_read,
        "rejected_count": read.rejected_count,
        "identity_count": read.identity_count,
        "unscanned_key_count": read.unscanned_key_count,
        "complete": read.complete,
        "restarted": read.restarted,
    })
}

/// Read the store of the private document type `documentTypeName` of the contract
/// `contractId` (32 bytes) for every identity of the wallet, at most `maxPages` proved pages
/// (bridges `platform_wallet_read_private_document_entries`). The keys are derived inside
/// `coreSignerHandle`, the manager's `MnemonicResolverHandle`.
///
/// The read starts at the store's start when `fromKeySet` is null; otherwise it reads on from
/// the cursor a previous read returned, given back field by field as it came
/// (`fromPosition`, `fromProvedTotalCount` with `fromHasProvedTotalCount`, `fromKeySet`,
/// `fromStore`). Returns the result as JSON (see `read_json`), or null after throwing.
/// Blocking (network).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_PrivateDocumentsNative_readPrivateDocumentEntries(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    core_signer_handle: jlong,
    contract_id: JByteArray,
    document_type_name: JString,
    from_position: jlong,
    from_proved_total_count: jlong,
    from_has_proved_total_count: jboolean,
    from_key_set: JByteArray,
    from_store: JByteArray,
    max_pages: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Ok(max_pages) = u32::try_from(max_pages) else {
            throw_sdk_exception(env, 1, "maxPages must not be negative");
            return ptr::null_mut();
        };
        let Some(contract_id) = read_32(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        if document_type_name.is_null() {
            throw_sdk_exception(env, 1, "documentTypeName was null");
            return ptr::null_mut();
        }
        let name: String = match env.get_string(&document_type_name) {
            Ok(name) => name.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "documentTypeName was not a readable string");
                return ptr::null_mut();
            }
        };
        let Ok(name) = CString::new(name) else {
            throw_sdk_exception(env, 1, "documentTypeName holds a NUL character");
            return ptr::null_mut();
        };
        // Each fingerprint read only when given, so a missing one reaches `cursor_from_parts`
        // as absent and a wrong-length one throws here
        let key_set = if from_key_set.is_null() {
            None
        } else {
            let Some(key_set) = read_32(env, &from_key_set, "fromKeySet") else {
                return ptr::null_mut();
            };
            Some(key_set)
        };
        let store = if from_store.is_null() {
            None
        } else {
            let Some(store) = read_32(env, &from_store, "fromStore") else {
                return ptr::null_mut();
            };
            Some(store)
        };
        let from = match cursor_from_parts(
            from_position,
            from_proved_total_count,
            from_has_proved_total_count != 0,
            key_set,
            store,
        ) {
            Ok(from) => from,
            Err(reason) => {
                throw_sdk_exception(env, 1, reason);
                return ptr::null_mut();
            }
        };
        let from_ptr = from.as_ref().map_or(ptr::null(), |cursor| {
            cursor as *const PrivateDocumentCursorFFI
        });

        let mut read = PrivateDocumentReadFFI::empty();
        let result = unsafe {
            platform_wallet_read_private_document_entries(
                wallet_handle as Handle,
                core_signer_handle as *mut MnemonicResolverHandle,
                contract_id.as_ptr(),
                name.as_ptr(),
                from_ptr,
                max_pages,
                &mut read,
            )
        };
        // The read leaves the empty value on every failure, so the JSON is built only on
        // success and the result is freed on both paths.
        let json = if result.code == PlatformWalletFFIResultCode::Success {
            // SAFETY: `read` was filled by the call above and is freed only below.
            Some(unsafe { read_json(&read) }.to_string())
        } else {
            None
        };
        unsafe { platform_wallet_private_document_read_free(&mut read) };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Some(json) = json else {
            return ptr::null_mut();
        };
        env.new_string(json)
            .map(|string| string.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> PrivateDocumentEntryFFI {
        PrivateDocumentEntryFFI {
            position: 7,
            block_height: 120,
            block_time_ms: 1_700_000_000_000,
            properties_json: ptr::null_mut(),
            properties_refusal: ptr::null_mut(),
            wallet_identity_id: [0xA1; 32],
            recipient_identity_id: [0xB2; 32],
            claimed_sender_identity_id: [0; 32],
            commitment: [0xC3; 32],
            wallet_key_id: 3,
            claimed_sender_key_id: 0,
            direction: 0,
            auth_mode: 0,
            has_claimed_sender: false,
        }
    }

    #[test]
    fn should_write_an_anonymous_entry_without_a_sender() {
        let properties = CString::new(r#"{"note":"hi"}"#).expect("no NUL");
        let mut anonymous = entry();
        anonymous.properties_json = properties.as_ptr() as *mut _;
        let json = unsafe { entry_json(&anonymous) };
        assert_eq!(json["position"], 7);
        assert_eq!(json["block_height"], 120);
        assert_eq!(json["block_time_ms"], 1_700_000_000_000u64);
        assert_eq!(json["properties_json"], r#"{"note":"hi"}"#);
        assert_eq!(json["properties_refusal"], Value::Null);
        assert_eq!(json["wallet_identity_id"], "a1".repeat(32));
        assert_eq!(json["recipient_identity_id"], "b2".repeat(32));
        assert_eq!(json["commitment"], "c3".repeat(32));
        assert_eq!(json["wallet_key_id"], 3);
        assert_eq!(json["claimed_sender_identity_id"], Value::Null);
        assert_eq!(json["claimed_sender_key_id"], Value::Null);
        assert_eq!(json["direction"], 0);
        assert_eq!(json["auth_mode"], 0);
    }

    #[test]
    fn should_write_a_claimed_sender_and_a_refusal() {
        let refusal = CString::new("missing note").expect("no NUL");
        let mut signed = entry();
        signed.properties_refusal = refusal.as_ptr() as *mut _;
        signed.has_claimed_sender = true;
        signed.claimed_sender_identity_id = [0xD4; 32];
        signed.claimed_sender_key_id = 9;
        signed.direction = 1;
        signed.auth_mode = 2;
        let json = unsafe { entry_json(&signed) };
        assert_eq!(json["properties_json"], Value::Null);
        assert_eq!(json["properties_refusal"], "missing note");
        assert_eq!(json["claimed_sender_identity_id"], "d4".repeat(32));
        assert_eq!(json["claimed_sender_key_id"], 9);
        assert_eq!(json["direction"], 1);
        assert_eq!(json["auth_mode"], 2);
    }

    #[test]
    fn should_write_the_counts_and_a_cursor_with_an_unknown_store_size() {
        let mut read = PrivateDocumentReadFFI::empty();
        read.cursor = PrivateDocumentCursorFFI {
            position: 40,
            proved_total_count: 0,
            key_set: [0x01; 32],
            store: [0x02; 32],
            has_proved_total_count: false,
        };
        read.pages_read = 2;
        read.rejected_count = 1;
        read.identity_count = 3;
        read.unscanned_key_count = 4;
        read.complete = true;
        read.restarted = true;
        let json = unsafe { read_json(&read) };
        assert_eq!(json["entries"], json!([]));
        assert_eq!(json["cursor"]["position"], 40);
        assert_eq!(json["cursor"]["proved_total_count"], Value::Null);
        assert_eq!(json["cursor"]["key_set"], "01".repeat(32));
        assert_eq!(json["cursor"]["store"], "02".repeat(32));
        assert_eq!(json["pages_read"], 2);
        assert_eq!(json["rejected_count"], 1);
        assert_eq!(json["identity_count"], 3);
        assert_eq!(json["unscanned_key_count"], 4);
        assert_eq!(json["complete"], true);
        assert_eq!(json["restarted"], true);

        read.cursor.has_proved_total_count = true;
        read.cursor.proved_total_count = 55;
        assert_eq!(
            unsafe { read_json(&read) }["cursor"]["proved_total_count"],
            55
        );
    }

    #[test]
    fn should_write_every_entry_of_a_read_in_store_order() {
        let first_properties = CString::new(r#"{"note":"one"}"#).expect("no NUL");
        let second_refusal = CString::new("missing note").expect("no NUL");
        let mut first = entry();
        first.properties_json = first_properties.as_ptr() as *mut _;
        let mut second = entry();
        second.position = 8;
        second.direction = 1;
        second.properties_refusal = second_refusal.as_ptr() as *mut _;
        let mut entries = [first, second];
        // A stack read over stack entries: never passed to the free function
        let mut read = PrivateDocumentReadFFI::empty();
        read.entries = entries.as_mut_ptr();
        read.entry_count = entries.len();
        let json = unsafe { read_json(&read) };
        let written = json["entries"].as_array().expect("an array");
        assert_eq!(written.len(), 2);
        assert_eq!(written[0]["position"], 7);
        assert_eq!(written[0]["properties_json"], r#"{"note":"one"}"#);
        assert_eq!(written[1]["position"], 8);
        assert_eq!(written[1]["direction"], 1);
        assert_eq!(written[1]["properties_refusal"], "missing note");
    }

    #[test]
    fn should_write_bytes_as_lowercase_hex() {
        assert_eq!(hex(&[0x00, 0x0a, 0xff, 0x5c]), "000aff5c");
        assert_eq!(hex(&[]), "");
    }

    #[test]
    fn should_read_from_the_start_without_fingerprints() {
        assert_eq!(cursor_from_parts(5, 9, true, None, None), Ok(None));
    }

    #[test]
    fn should_rebuild_a_cursor_from_its_parts() {
        let cursor = cursor_from_parts(40, 55, true, Some([1; 32]), Some([2; 32]))
            .expect("valid")
            .expect("a cursor");
        assert_eq!(
            cursor,
            PrivateDocumentCursorFFI {
                position: 40,
                proved_total_count: 55,
                key_set: [1; 32],
                store: [2; 32],
                has_proved_total_count: true,
            }
        );
    }

    #[test]
    fn should_refuse_a_cursor_with_one_fingerprint_or_a_negative_field() {
        assert!(cursor_from_parts(0, 0, false, Some([1; 32]), None).is_err());
        assert!(cursor_from_parts(0, 0, false, None, Some([2; 32])).is_err());
        assert!(cursor_from_parts(-1, 0, false, Some([1; 32]), Some([2; 32])).is_err());
        assert!(cursor_from_parts(0, -1, true, Some([1; 32]), Some([2; 32])).is_err());
    }
}
