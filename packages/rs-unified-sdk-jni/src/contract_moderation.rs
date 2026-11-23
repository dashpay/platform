//! JNI exports for the contract moderation and fee pot reads (protocol version 14). Each
//! export marshals ONE C function of `rs-sdk-ffi`: `dash_sdk_contract_moderation_get_*` or
//! `dash_sdk_contract_get_fee_pots`. It reads the JVM arguments, calls the C function, copies
//! the JSON it returns into a JVM string and frees the Rust string before returning.
//!
//! rs-sdk decides everything: which lists a contract keeps, the page caps, the next page's
//! cursor, and how a removal record's kept values read (the removal exports go through
//! rs-sdk's `fetch_with_kept_values`, which decodes a page's records under one budget). An
//! argument the JVM value cannot carry into the C type (a null required string, a list or
//! status that does not fit a byte, a limit outside `u32`) throws code 1
//! (`InvalidParameter`) before any C call; everything the C functions and the node check is
//! left to them. Errors of the C functions throw `DashSDKException` through
//! [`crate::results::take_error`].
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.ContractModerationNative`.

use crate::decision_proposals::{
    c_ptr, read_cstring, read_optional_cstring, string_or_null, to_unsigned,
};
use crate::support::{guard, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JObjectArray, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_TRUE};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_contract_get_fee_pots, dash_sdk_contract_moderation_get_action_counts,
    dash_sdk_contract_moderation_get_document_removals,
    dash_sdk_contract_moderation_get_document_removals_by_ids,
    dash_sdk_contract_moderation_get_entries, dash_sdk_contract_moderation_get_status,
    dash_sdk_contract_moderation_get_team_action_signers,
    dash_sdk_contract_moderation_get_team_actions, SDKHandle,
};
use std::ffi::CString;
use std::os::raw::c_char;
use std::ptr;

/// The `limit` the Kotlin side passes when it names no page bound: the C functions then get a
/// null pointer, which rs-sdk reads as the page cap of the network queried.
const NO_LIMIT: jlong = -1;

/// The page bound for the C functions' `*const u32`: `Some(None)` for [`NO_LIMIT`],
/// `Some(Some(value))` for a value that fits a `u32` (0 included, which the node judges), and
/// `None` after throwing for anything else.
fn read_limit(env: &mut JNIEnv, limit: jlong) -> Option<Option<u32>> {
    if limit == NO_LIMIT {
        return Some(None);
    }
    to_unsigned::<u32>(env, limit, "limit").map(Some)
}

/// The pointer the C functions take for an optional limit: null for `None`.
fn limit_ptr(limit: &Option<u32>) -> *const u32 {
    limit.as_ref().map_or(ptr::null(), |l| l as *const u32)
}

/// Read a `String[]` into owned `CString`s; throws and returns `None` for a null array, a
/// null or unreadable element, or one holding a NUL byte.
fn read_cstring_array(env: &mut JNIEnv, arr: &JObjectArray, field: &str) -> Option<Vec<CString>> {
    if arr.is_null() {
        throw_sdk_exception(env, 1, &format!("{field} was null"));
        return None;
    }
    let len = match env.get_array_length(arr) {
        Ok(len) => len,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} array was invalid"));
            return None;
        }
    };
    let mut strings = Vec::with_capacity(usize::try_from(len).unwrap_or(0));
    for index in 0..len {
        let element = match env.get_object_array_element(arr, index) {
            Ok(element) => JString::from(element),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, &format!("{field}[{index}] was invalid"));
                return None;
            }
        };
        // The element's field name is built only when a read fails
        let value = if element.is_null() {
            None
        } else {
            env.get_string(&element)
                .ok()
                .and_then(|value| CString::new(String::from(value)).ok())
        };
        let Some(value) = value else {
            let _ = env.exception_clear();
            throw_sdk_exception(
                env,
                1,
                &format!("{field}[{index}] was null, unreadable or held a NUL"),
            );
            return None;
        };
        let _ = env.delete_local_ref(element);
        strings.push(value);
    }
    Some(strings)
}

/// One identity's status on a moderated contract, on the lists read
/// (`dash_sdk_contract_moderation_get_status`), as its JSON object; null when rs-sdk returns
/// none, and after throwing.
///
/// `lists` holds the list bytes (0 banlist, 1 suspensions, 2 warnings): null for no selection
/// (rs-sdk reads every list the contract keeps), an empty array for an explicit empty
/// selection, which the node judges.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getStatus(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    identity_id: JString,
    lists: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(identity) = read_cstring(env, &identity_id, "identityId") else {
            return ptr::null_mut();
        };
        let lists: Option<Vec<u8>> = if lists.is_null() {
            None
        } else {
            match env.convert_byte_array(&lists) {
                Ok(bytes) => Some(bytes),
                Err(_) => {
                    let _ = env.exception_clear();
                    throw_sdk_exception(env, 1, "lists byte[] was invalid");
                    return ptr::null_mut();
                }
            }
        };
        // A null pointer with a count of 0 is no selection; an explicit empty selection
        // needs a non-null pointer, which an empty Vec's dangling pointer is.
        let (lists_ptr, lists_count) = match &lists {
            None => (ptr::null(), 0),
            Some(bytes) => (bytes.as_ptr(), bytes.len()),
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_status(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                identity.as_ptr(),
                lists_ptr,
                lists_count,
            );
            string_or_null(env, result)
        }
    })
}

/// One page of a moderation list, in identity id order
/// (`dash_sdk_contract_moderation_get_entries`), as its JSON object; null when rs-sdk returns
/// none, and after throwing. `list` is the list byte; `startAfter` (base58) is null for the
/// first page; `limit` is -1 for the network's page cap.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getEntries(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    list: jint,
    start_after: JString,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(list) = to_unsigned::<u8>(env, list.into(), "list") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        let Some(limit) = read_limit(env, limit) else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_entries(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                list,
                c_ptr(&start_after),
                limit_ptr(&limit),
            );
            string_or_null(env, result)
        }
    })
}

/// One page of the removal records of one document type, in document id order
/// (`dash_sdk_contract_moderation_get_document_removals`), as its JSON object; null when
/// rs-sdk returns none, and after throwing. Paging as for `getEntries`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getDocumentRemovals(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    document_type_name: JString,
    start_after: JString,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(type_name) = read_cstring(env, &document_type_name, "documentTypeName") else {
            return ptr::null_mut();
        };
        let Some(start_after) = read_optional_cstring(env, &start_after, "startAfter") else {
            return ptr::null_mut();
        };
        let Some(limit) = read_limit(env, limit) else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_document_removals(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                type_name.as_ptr(),
                c_ptr(&start_after),
                limit_ptr(&limit),
            );
            string_or_null(env, result)
        }
    })
}

/// The removal records of the documents named, within one document type
/// (`dash_sdk_contract_moderation_get_document_removals_by_ids`), as a JSON object without a
/// cursor; null when rs-sdk returns none, and after throwing. `documentIds` are base58.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getDocumentRemovalsByIds(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    document_type_name: JString,
    document_ids: JObjectArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(type_name) = read_cstring(env, &document_type_name, "documentTypeName") else {
            return ptr::null_mut();
        };
        let Some(ids) = read_cstring_array(env, &document_ids, "documentIds") else {
            return ptr::null_mut();
        };
        let pointers: Vec<*const c_char> = ids.iter().map(|id| id.as_ptr()).collect();
        unsafe {
            let result = dash_sdk_contract_moderation_get_document_removals_by_ids(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                type_name.as_ptr(),
                pointers.as_ptr(),
                pointers.len(),
            );
            string_or_null(env, result)
        }
    })
}

/// One page of an elected contract's team actions of one status, in action id order
/// (`dash_sdk_contract_moderation_get_team_actions`), as its JSON object; null when rs-sdk
/// returns none, and after throwing. `status` is the status byte (0 active, 1 closed);
/// `startAtActionId` (base58) is null for the first page, and `startAtActionIdIncluded` is
/// passed as it is (the C function refuses it without an action); `limit` is -1 for the
/// network's page cap.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getTeamActions(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    status: jint,
    start_at_action_id: JString,
    start_at_action_id_included: jboolean,
    limit: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(status) = to_unsigned::<u8>(env, status.into(), "status") else {
            return ptr::null_mut();
        };
        let Some(start) = read_optional_cstring(env, &start_at_action_id, "startAtActionId") else {
            return ptr::null_mut();
        };
        let Some(limit) = read_limit(env, limit) else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_team_actions(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                status,
                c_ptr(&start),
                start_at_action_id_included == JNI_TRUE,
                limit_ptr(&limit),
            );
            string_or_null(env, result)
        }
    })
}

/// Who approved one team action, looked up among the actions of `status` alone
/// (`dash_sdk_contract_moderation_get_team_action_signers`), as its JSON object; null when
/// rs-sdk returns none, and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getTeamActionSigners(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
    status: jint,
    action_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(status) = to_unsigned::<u8>(env, status.into(), "status") else {
            return ptr::null_mut();
        };
        let Some(action) = read_cstring(env, &action_id, "actionId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_team_action_signers(
                sdk as *const SDKHandle,
                contract.as_ptr(),
                status,
                action.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

/// How many counted moderation actions each member of an elected contract's seated team
/// signed since the moderators pot was last claimed or settled
/// (`dash_sdk_contract_moderation_get_action_counts`), as its JSON object; null when rs-sdk
/// returns none, and after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getActionCounts(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_moderation_get_action_counts(
                sdk as *const SDKHandle,
                contract.as_ptr(),
            );
            string_or_null(env, result)
        }
    })
}

/// A contract's owner and moderators fee pots and the last explicit claim of each
/// (`dash_sdk_contract_get_fee_pots`), as its JSON object; null when rs-sdk returns none, and
/// after throwing.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractModerationNative_getFeePots(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    contract_id: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(contract) = read_cstring(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        unsafe {
            let result = dash_sdk_contract_get_fee_pots(sdk as *const SDKHandle, contract.as_ptr());
            string_or_null(env, result)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::NO_LIMIT;
    use crate::decision_proposals::tests::kotlin_signature_in;

    const NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/ContractModerationNative.kt"
    ));
    const MODELS_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/moderation/ContractModerationModels.kt"
    ));
    /// Where rs-sdk-ffi declares the list and status bytes (`pub(crate)`, so read as text).
    const FFI_MODERATION_RS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../rs-sdk-ffi/src/contract_moderation/mod.rs"
    ));

    /// The discriminant of `{variant} = N,` in rs-sdk-ffi's moderation enums.
    fn ffi_discriminant(variant: &str) -> u8 {
        FFI_MODERATION_RS
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix(&format!("{variant} = ")))
            .and_then(|rest| rest.trim_end_matches(',').parse().ok())
            .unwrap_or_else(|| panic!("rs-sdk-ffi declares {variant} with a discriminant"))
    }

    /// The `ffiTag` of the Kotlin enum entry `{entry}("{json}", N)`.
    fn kotlin_ffi_tag(entry: &str, json: &str) -> u8 {
        MODELS_KT
            .lines()
            .map(str::trim)
            .find_map(|line| line.strip_prefix(&format!("{entry}(\"{json}\", ")))
            .and_then(|rest| rest.trim_end_matches("),").parse().ok())
            .unwrap_or_else(|| panic!("ContractModerationModels.kt declares {entry} with its byte"))
    }

    /// The list and status bytes Kotlin passes are the discriminants rs-sdk-ffi reads: a
    /// permutation on either side would query another list or status without failing.
    #[test]
    fn should_pass_the_list_and_status_bytes_rs_sdk_ffi_reads() {
        for (entry, json, variant) in [
            ("BANLIST", "banlist", "Banlist"),
            ("SUSPENSIONS", "suspensions", "Suspensions"),
            ("WARNINGS", "warnings", "Warnings"),
            ("ACTIVE", "active", "Active"),
            ("CLOSED", "closed", "Closed"),
        ] {
            assert_eq!(
                kotlin_ffi_tag(entry, json),
                ffi_discriminant(variant),
                "{entry}"
            );
        }
    }

    /// Kotlin's "no page bound" is the value this crate reads as no bound; another value
    /// would reach `to_unsigned` and be refused.
    #[test]
    fn should_share_the_no_limit_sentinel_with_kotlin() {
        let declaration = format!("const val NO_LIMIT: Long = {NO_LIMIT}");
        assert!(
            NATIVE_KT.lines().any(|line| line.trim() == declaration),
            "ContractModerationNative.kt no longer declares `{declaration}`"
        );
    }

    fn signature(name: &str) -> (Vec<String>, String) {
        kotlin_signature_in(NATIVE_KT, name)
    }

    fn expected(parameters: &[&str]) -> (Vec<String>, String) {
        (
            parameters.iter().map(|kind| kind.to_string()).collect(),
            "String?".to_string(),
        )
    }

    /// JNI binds an export by its name alone: a Kotlin declaration whose parameters drifted
    /// from the export's would hand it the wrong JVM values at run time, which only an
    /// instrumented test would otherwise catch.
    #[test]
    fn should_declare_the_moderation_reads_as_the_exports_take_them() {
        assert_eq!(
            signature("getStatus"),
            expected(&["Long", "String", "String", "ByteArray?"])
        );
        assert_eq!(
            signature("getEntries"),
            expected(&["Long", "String", "Int", "String?", "Long"])
        );
        assert_eq!(
            signature("getDocumentRemovals"),
            expected(&["Long", "String", "String", "String?", "Long"])
        );
        assert_eq!(
            signature("getDocumentRemovalsByIds"),
            expected(&["Long", "String", "String", "Array<String>"])
        );
        assert_eq!(
            signature("getTeamActions"),
            expected(&["Long", "String", "Int", "String?", "Boolean", "Long"])
        );
        assert_eq!(
            signature("getTeamActionSigners"),
            expected(&["Long", "String", "Int", "String"])
        );
        assert_eq!(signature("getActionCounts"), expected(&["Long", "String"]));
        assert_eq!(signature("getFeePots"), expected(&["Long", "String"]));
    }
}
