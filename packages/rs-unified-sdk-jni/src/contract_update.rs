//! JNI exports for reviewing a data contract update before it is broadcast. Each export
//! marshals ONE C function of `platform-wallet-ffi`:
//! `platform_wallet_preview_data_contract_update` (what an update with these arguments would
//! change in the live contract, without signing or broadcasting) and
//! `platform_wallet_split_contract_update_source` (pasted or imported contract JSON split into
//! the sections an update takes). It reads the JVM arguments, calls the C function, copies
//! the JSON it wrote into a JVM string and frees the Rust string with
//! `platform_wallet_string_free` before returning.
//!
//! Rust decides everything: the preview fetches the live contract, checks the owner, merges
//! the sections and compares; the split decides which members are sections and refuses text
//! it cannot split. An argument the JVM value cannot carry into the C type (a null required
//! string, an id that is not 32 bytes, a string holding a NUL byte) throws code 1
//! (`InvalidParameter`) before any C call. Errors of the C functions throw
//! `DashSDKException` through [`crate::support::take_pwffi_error`].
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.ContractUpdateNative`.

use crate::decision_proposals::{
    c_ptr, read_cstring, read_fixed_bytes, read_optional_cstring, take_wallet_json,
};
use crate::support::{guard, take_pwffi_error};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::{
    platform_wallet_preview_data_contract_update, platform_wallet_split_contract_update_source,
};
use std::os::raw::c_char;
use std::ptr;

/// What `IdentityNative.updateDataContract` with the same arguments would change in the
/// contract `contractId` (`platform_wallet_preview_data_contract_update`), as the preview
/// JSON object; null after throwing. Nothing is signed or broadcast and no nonce is used.
///
/// `ownerIdentityId` and `contractId` are 32 bytes; `documentsSchemaJson` is required; the
/// other sections are null to keep the live ones (the C function also reads an empty string
/// as null).
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractUpdateNative_previewDataContractUpdate(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    owner_identity_id: JByteArray,
    contract_id: JByteArray,
    documents_schema_json: JString,
    tokens_schema_json: JString,
    groups_schema_json: JString,
    keywords_json: JString,
    description: JString,
    config_json: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_fixed_bytes::<32>(env, &owner_identity_id, "ownerIdentityId") else {
            return ptr::null_mut();
        };
        let Some(contract) = read_fixed_bytes::<32>(env, &contract_id, "contractId") else {
            return ptr::null_mut();
        };
        let Some(documents) = read_cstring(env, &documents_schema_json, "documentsSchemaJson")
        else {
            return ptr::null_mut();
        };
        let Some(tokens) = read_optional_cstring(env, &tokens_schema_json, "tokensSchemaJson")
        else {
            return ptr::null_mut();
        };
        let Some(groups) = read_optional_cstring(env, &groups_schema_json, "groupsSchemaJson")
        else {
            return ptr::null_mut();
        };
        let Some(keywords) = read_optional_cstring(env, &keywords_json, "keywordsJson") else {
            return ptr::null_mut();
        };
        let Some(description) = read_optional_cstring(env, &description, "description") else {
            return ptr::null_mut();
        };
        let Some(config) = read_optional_cstring(env, &config_json, "configJson") else {
            return ptr::null_mut();
        };

        let mut out_preview_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_preview_data_contract_update(
                wallet_handle as Handle,
                owner.as_ptr(),
                contract.as_ptr(),
                documents.as_ptr(),
                c_ptr(&tokens),
                c_ptr(&groups),
                c_ptr(&keywords),
                c_ptr(&description),
                c_ptr(&config),
                &mut out_preview_json,
            )
        };
        if take_pwffi_error(env, result) {
            // The C function leaves the out pointer null on every error.
            return ptr::null_mut();
        }
        let Some(json) = (unsafe {
            take_wallet_json(
                env,
                out_preview_json,
                "platform_wallet_preview_data_contract_update",
            )
        }) else {
            return ptr::null_mut();
        };
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// Pasted or imported contract JSON split into the sections a contract update takes
/// (`platform_wallet_split_contract_update_source`), as the sections JSON object (each
/// section JSON text in source order, or null when the text does not supply it); null after
/// throwing. Text that is not a JSON object, or supplies no section, is refused with a
/// message the app can show as is.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_ContractUpdateNative_splitContractUpdateSource(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(text) = read_cstring(env, &text, "text") else {
            return ptr::null_mut();
        };
        let mut out_sections_json: *mut c_char = ptr::null_mut();
        let result = unsafe {
            platform_wallet_split_contract_update_source(text.as_ptr(), &mut out_sections_json)
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Some(json) = (unsafe {
            take_wallet_json(
                env,
                out_sections_json,
                "platform_wallet_split_contract_update_source",
            )
        }) else {
            return ptr::null_mut();
        };
        env.new_string(json)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

#[cfg(test)]
mod tests {
    use crate::decision_proposals::tests::kotlin_signature_in;

    const CONTRACT_UPDATE_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/ContractUpdateNative.kt"
    ));
    const IDENTITY_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/IdentityNative.kt"
    ));

    fn types(kinds: &[&str]) -> Vec<String> {
        kinds.iter().map(|kind| kind.to_string()).collect()
    }

    /// JNI binds an export by its name alone: a Kotlin declaration whose parameters drifted
    /// from the export's would hand it the wrong JVM values at run time, which only an
    /// instrumented test would otherwise catch. The update's `expectedCurrentVersion` slot
    /// sits before the signer handle.
    #[test]
    fn should_declare_the_review_and_the_update_as_the_exports_take_them() {
        assert_eq!(
            kotlin_signature_in(CONTRACT_UPDATE_NATIVE_KT, "previewDataContractUpdate"),
            (
                types(&[
                    "Long",
                    "ByteArray",
                    "ByteArray",
                    "String",
                    "String?",
                    "String?",
                    "String?",
                    "String?",
                    "String?",
                ]),
                "String".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(CONTRACT_UPDATE_NATIVE_KT, "splitContractUpdateSource"),
            (types(&["String"]), "String".to_string())
        );
        assert_eq!(
            kotlin_signature_in(IDENTITY_NATIVE_KT, "updateDataContract"),
            (
                types(&[
                    "Long",
                    "ByteArray",
                    "ByteArray",
                    "String",
                    "String?",
                    "String?",
                    "String?",
                    "String?",
                    "String?",
                    "Long",
                    "Long",
                ]),
                "ByteArray".to_string()
            )
        );
    }
}
