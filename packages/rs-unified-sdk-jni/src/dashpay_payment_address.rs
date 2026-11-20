//! JNI bridge for the public payment addresses a DashPay profile publishes:
//! which of them the connected network lets a profile write set, and the
//! conversion between the address strings people read and paste and the
//! bytes the profile stores.
//!
//! Kotlin counterpart: the payment address natives on
//! `org.dashfoundation.dashsdk.ffi.DashpayNative`, wrapped by
//! `org.dashfoundation.dashsdk.tokens.Dashpay` and `DashPayPaymentAddresses`.
//! Every decision (address parsing, network and kind checks, field support)
//! is `platform-wallet`'s, reached through
//! `platform_wallet_ffi::dashpay_profile_payment_address`; these exports
//! only marshal.

use crate::support::{guard, net_from_ord, read_jstring, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jbooleanArray, jbyteArray, jint, jlong, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;
use platform_wallet_ffi::dashpay_profile_payment_address::{
    dashpay_profile_payment_address_from_string, dashpay_profile_payment_address_to_string,
    platform_wallet_dashpay_profile_payment_address_support, DashPayPaymentAddressSupportFFI,
};
use platform_wallet_ffi::handle::Handle;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

/// The support flags in the order Kotlin reads them: `[core, platform,
/// shielded]`.
fn support_flags(support: &DashPayPaymentAddressSupportFFI) -> [u8; 3] {
    let flag = |set: bool| if set { JNI_TRUE } else { JNI_FALSE };
    [
        flag(support.core),
        flag(support.platform),
        flag(support.shielded),
    ]
}

/// `DashpayNative.paymentAddressSupport`: which payment addresses a profile
/// write may set on the network this wallet is connected to, as
/// `[core, platform, shielded]`
/// (`platform_wallet_dashpay_profile_payment_address_support`). Fetches the
/// network's DashPay contract, so it blocks on the network. Throws and
/// returns null on failure.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_paymentAddressSupport(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
) -> jbooleanArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let mut support = DashPayPaymentAddressSupportFFI::default();
        let result = unsafe {
            platform_wallet_dashpay_profile_payment_address_support(
                wallet_handle as Handle,
                &mut support,
            )
        };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let flags = support_flags(&support);
        let array = match env.new_boolean_array(flags.len() as i32) {
            Ok(array) => array,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "allocating the support flags failed");
                return ptr::null_mut();
            }
        };
        if env.set_boolean_array_region(&array, 0, &flags).is_err() {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 99, "writing the support flags failed");
            return ptr::null_mut();
        }
        array.into_raw()
    })
}

/// `DashpayNative.paymentAddressFromString`: the bytes a profile stores for
/// the address `text` of `kind` (a `DASHPAY_PROFILE_PAYMENT_ADDRESS_*` value)
/// on `network` (an FFINetwork ordinal), via
/// `dashpay_profile_payment_address_from_string`. An address of another
/// network or kind, or text that is not an address, throws with the wallet's
/// readable reason and returns null.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_paymentAddressFromString(
    mut env: JNIEnv,
    _class: JClass,
    kind: jint,
    text: JString,
    network: jint,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        if text.is_null() {
            throw_sdk_exception(env, 1, "text must not be null");
            return ptr::null_mut();
        }
        let text: String = match read_jstring(env, &text) {
            Ok(value) => value,
            Err(e) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, &format!("text {e}"));
                return ptr::null_mut();
            }
        };
        let Ok(text) = CString::new(text) else {
            throw_sdk_exception(env, 1, "text must not contain a NUL character");
            return ptr::null_mut();
        };

        let mut bytes: *mut u8 = ptr::null_mut();
        let mut len: usize = 0;
        let result = unsafe {
            dashpay_profile_payment_address_from_string(
                kind as u32,
                text.as_ptr(),
                net_from_ord(network),
                &mut bytes,
                &mut len,
            )
        };
        if take_pwffi_error(env, result) {
            // The FFI leaves `*out_bytes` null on every error path.
            return ptr::null_mut();
        }
        let copied = if bytes.is_null() {
            Vec::new()
        } else {
            let copied = unsafe { std::slice::from_raw_parts(bytes, len) }.to_vec();
            unsafe { platform_wallet_ffi::platform_wallet_bytes_free(bytes, len) };
            copied
        };
        match env.byte_array_from_slice(&copied) {
            Ok(array) => array.into_raw(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "allocating the address bytes failed");
                ptr::null_mut()
            }
        }
    })
}

/// `DashpayNative.paymentAddressToString`: the address string of `network`
/// (an FFINetwork ordinal) for the stored `bytes` of a `kind` address, via
/// `dashpay_profile_payment_address_to_string`. Returns null when the wallet
/// does not read the bytes as an address of that kind; an unknown `kind`
/// throws.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_DashpayNative_paymentAddressToString(
    mut env: JNIEnv,
    _class: JClass,
    kind: jint,
    bytes: JByteArray,
    network: jint,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        if bytes.is_null() {
            throw_sdk_exception(env, 1, "bytes must not be null");
            return ptr::null_mut();
        }
        let bytes = match env.convert_byte_array(&bytes) {
            Ok(bytes) => bytes,
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "bytes byte[] was invalid");
                return ptr::null_mut();
            }
        };
        let mut out: *mut c_char = ptr::null_mut();
        let result = unsafe {
            dashpay_profile_payment_address_to_string(
                kind as u32,
                bytes.as_ptr(),
                bytes.len(),
                net_from_ord(network),
                &mut out,
            )
        };
        if take_pwffi_error(env, result) || out.is_null() {
            return ptr::null_mut();
        }
        let text = unsafe { CStr::from_ptr(out) }
            .to_string_lossy()
            .into_owned();
        unsafe { platform_wallet_ffi::platform_wallet_string_free(out) };
        env.new_string(text)
            .map(|s| s.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_order_the_support_flags_core_platform_shielded() {
        let support = DashPayPaymentAddressSupportFFI {
            core: true,
            platform: false,
            shielded: true,
        };
        assert_eq!(support_flags(&support), [JNI_TRUE, JNI_FALSE, JNI_TRUE]);
        assert_eq!(
            support_flags(&DashPayPaymentAddressSupportFFI::default()),
            [JNI_FALSE; 3]
        );
    }
}
