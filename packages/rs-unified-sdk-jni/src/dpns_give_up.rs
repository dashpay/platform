//! JNI bridge for giving up a DPNS name (protocol version 15): which ways the
//! network accepts, the give-up itself, and the main-name rule a host applies
//! after a confirmed give-up.
//!
//! Every decision stays in `platform-wallet` (which ways are offered, the
//! re-check before signing, the protocol version refusal, what is persisted
//! and when, whether the main-name pick goes). This module only reads JVM
//! values, calls the `platform-wallet-ffi` entry points and copies their
//! plain-data results into compact JSON.

#![allow(clippy::missing_safety_doc)]

use crate::dpns_marketplace::{hex32, new_jstring, read_cstring, read_id32};
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::dpns_marketplace::{DpnsNameGiveUpChoicesFFI, DpnsNameGivenUpFFI};
use platform_wallet_ffi::handle::Handle;
use rs_sdk_ffi::SignerHandle;
use std::ptr;

fn choices_json(choices: &DpnsNameGiveUpChoicesFFI) -> String {
    format!(
        "{{\"documentId\":\"{}\",\"canDelete\":{},\"canTombstone\":{}}}",
        hex32(&choices.document_id),
        choices.can_delete,
        choices.can_tombstone,
    )
}

fn given_up_json(given_up: &DpnsNameGivenUpFFI) -> String {
    format!(
        "{{\"documentId\":\"{}\",\"method\":{},\"tombstoneId\":{}}}",
        hex32(&given_up.document_id),
        given_up.method,
        if given_up.has_tombstone {
            format!("\"{}\"", hex32(&given_up.tombstone_id))
        } else {
            "null".into()
        },
    )
}

/// `DpnsGiveUpNative.choices(walletHandle, ownerIdentityId, name): String`,
/// over `platform_wallet_dpns_name_give_up_choices`. Returns
/// `{"documentId":hex,"canDelete":bool,"canTombstone":bool}`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsGiveUpNative_choices(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    owner: JByteArray,
    name: JString,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner, "ownerIdentityId") else {
            return ptr::null_mut();
        };
        let Some(name) = read_cstring(env, &name, "name") else {
            return ptr::null_mut();
        };
        let mut out = DpnsNameGiveUpChoicesFFI::default();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_name_give_up_choices(
                wallet as Handle,
                owner.as_ptr(),
                name.as_ptr(),
                &mut out,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        new_jstring(env, choices_json(&out))
    })
}

/// `DpnsGiveUpNative.giveUp(walletHandle, ownerIdentityId, name,
/// expectedDocumentId, method, signerHandle): String`, over
/// `platform_wallet_dpns_give_up_name`. Returns
/// `{"documentId":hex,"method":int,"tombstoneId":hex|null}`.
///
/// `method` crosses as the FFI byte. A value outside `0..=255` is refused
/// here, so it can not truncate into a known method; Rust refuses any byte
/// it does not know.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsGiveUpNative_giveUp(
    mut env: JNIEnv,
    _class: JClass,
    wallet: jlong,
    owner: JByteArray,
    name: JString,
    expected_document_id: JByteArray,
    method: jint,
    signer: jlong,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(owner) = read_id32(env, &owner, "ownerIdentityId") else {
            return ptr::null_mut();
        };
        let Some(name) = read_cstring(env, &name, "name") else {
            return ptr::null_mut();
        };
        let Some(expected) = read_id32(env, &expected_document_id, "expectedDocumentId") else {
            return ptr::null_mut();
        };
        let Ok(method) = u8::try_from(method) else {
            throw_sdk_exception(
                env,
                1,
                &format!("method {method} is not a give-up method byte"),
            );
            return ptr::null_mut();
        };
        if signer == 0 {
            throw_sdk_exception(env, 1, "signerHandle must not be 0");
            return ptr::null_mut();
        }
        let mut out = DpnsNameGivenUpFFI::default();
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_give_up_name(
                wallet as Handle,
                owner.as_ptr(),
                name.as_ptr(),
                expected.as_ptr(),
                method,
                signer as *mut SignerHandle,
                &mut out,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        new_jstring(env, given_up_json(&out))
    })
}

/// `DpnsGiveUpNative.clearsMainName(mainName: String?, givenUpName: String):
/// Boolean`, over `platform_wallet_dpns_give_up_clears_main_name`. A null
/// `mainName` (no pick) crosses as a null pointer.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DpnsGiveUpNative_clearsMainName(
    mut env: JNIEnv,
    _class: JClass,
    main_name: JString,
    given_up_name: JString,
) -> jboolean {
    guard(&mut env, JNI_FALSE, |env| {
        let main = if main_name.is_null() {
            None
        } else {
            let Some(main) = read_cstring(env, &main_name, "mainName") else {
                return JNI_FALSE;
            };
            Some(main)
        };
        let Some(given_up) = read_cstring(env, &given_up_name, "givenUpName") else {
            return JNI_FALSE;
        };
        let mut clears = false;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_dpns_give_up_clears_main_name(
                main.as_ref().map_or(ptr::null(), |main| main.as_ptr()),
                given_up.as_ptr(),
                &mut clears,
            )
        };
        if take_pwffi_error(env, result) {
            return JNI_FALSE;
        }
        if clears {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_encode_a_tombstone_with_its_id_and_a_delete_with_none() {
        let tombstoned = DpnsNameGivenUpFFI {
            document_id: [0xAB; 32],
            method: platform_wallet_ffi::dpns_marketplace::DPNS_NAME_GIVE_UP_METHOD_TOMBSTONE,
            has_tombstone: true,
            tombstone_id: [0x01; 32],
        };
        assert_eq!(
            given_up_json(&tombstoned),
            format!(
                "{{\"documentId\":\"{}\",\"method\":1,\"tombstoneId\":\"{}\"}}",
                "ab".repeat(32),
                "01".repeat(32)
            )
        );
        let deleted = DpnsNameGivenUpFFI {
            document_id: [0xAB; 32],
            method: platform_wallet_ffi::dpns_marketplace::DPNS_NAME_GIVE_UP_METHOD_DELETE,
            has_tombstone: false,
            tombstone_id: [0x01; 32],
        };
        assert_eq!(
            given_up_json(&deleted),
            format!(
                "{{\"documentId\":\"{}\",\"method\":0,\"tombstoneId\":null}}",
                "ab".repeat(32)
            )
        );
    }

    #[test]
    fn should_encode_the_choices_flags_as_json_booleans() {
        let choices = DpnsNameGiveUpChoicesFFI {
            document_id: [0x0F; 32],
            can_delete: false,
            can_tombstone: true,
        };
        assert_eq!(
            choices_json(&choices),
            format!(
                "{{\"documentId\":\"{}\",\"canDelete\":false,\"canTombstone\":true}}",
                "0f".repeat(32)
            )
        );
    }
}
