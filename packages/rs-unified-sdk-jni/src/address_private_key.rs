//! JNI export revealing the private key of one of a wallet's core (Layer-1)
//! addresses. It marshals ONE C function of `platform-wallet-ffi`:
//!
//! * `revealCoreAddressPrivateKey` -> `platform_wallet_address_private_key`
//!   (Swift `ManagedPlatformWallet.coreAddressPrivateKey(address:)`).
//!
//! Rust finds the address in the wallet's account pools, reads the mnemonic
//! through the resolver, and derives the key; this shim only copies the hex
//! and WIF it returns into one JVM `byte[]`. Every Rust copy is zeroed: the
//! FFI strings by `platform_wallet_address_private_key_free`, and the blob
//! built from them on drop. The returned `byte[]` is the Kotlin caller's to
//! zero.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.AddressPrivateKeyNative`.

use crate::decision_proposals::read_cstring;
use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JClass, JString};
use jni::sys::{jbyteArray, jlong};
use jni::JNIEnv;
use platform_wallet_ffi::address_private_key::{
    platform_wallet_address_private_key, platform_wallet_address_private_key_free,
    AddressPrivateKeyFFI,
};
use platform_wallet_ffi::handle::Handle;
use rs_sdk_ffi::MnemonicResolverHandle;
use std::ffi::CStr;
use std::ptr;
use zeroize::Zeroizing;

/// The private key of `address`, one of the wallet's tracked core addresses,
/// as one `byte[]`:
///
/// ```text
/// u8               hex_len
/// u8[hex_len]      lowercase hex of the 32-byte secret (ASCII)
/// u8               wif_len
/// u8[wif_len]      compressed, network-aware WIF (ASCII)
/// ```
///
/// `resolverHandle` is the manager's `MnemonicResolverHandle`; Rust consults
/// it only when the wallet holds no resident private keys. Null after
/// throwing the platform-wallet error: an unknown handle, an address the
/// wallet does not track, an account that holds no key for it (a DashPay
/// contact's external address), or a failed derive.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_AddressPrivateKeyNative_revealCoreAddressPrivateKey(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    resolver_handle: jlong,
    address: JString,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(address) = read_cstring(env, &address, "address") else {
            return ptr::null_mut();
        };
        let mut out = AddressPrivateKeyFFI {
            private_key_hex: ptr::null_mut(),
            private_key_wif: ptr::null_mut(),
        };
        let result = unsafe {
            platform_wallet_address_private_key(
                wallet_handle as Handle,
                resolver_handle as *mut MnemonicResolverHandle,
                address.as_ptr(),
                &mut out,
            )
        };
        // Copy the two strings out, then zero and free the Rust-owned ones
        // whether or not the call succeeded (the free no-ops on null fields).
        let blob = unsafe { encode_key(&out) };
        unsafe { platform_wallet_address_private_key_free(&mut out) };
        if take_pwffi_error(env, result) {
            return ptr::null_mut();
        }
        let Some(blob) = blob else {
            throw_sdk_exception(env, 99, "address private key came back malformed");
            return ptr::null_mut();
        };
        match env.byte_array_from_slice(&blob) {
            Ok(array) => array.into_raw(),
            Err(_) => {
                // Keeps a pending `OutOfMemoryError` from `NewByteArray`:
                // `throw_sdk_exception` throws only when none is pending.
                throw_sdk_exception(env, 99, "address private key allocation failed");
                ptr::null_mut()
            }
        }
    })
}

/// The blob documented on the export, in a buffer zeroed on drop. `None` when
/// a field is null or longer than a `u8` length can carry, which a successful
/// call never produces.
///
/// # Safety
/// Each non-null field of `out` must be a NUL-terminated string from
/// `platform_wallet_address_private_key`.
unsafe fn encode_key(out: &AddressPrivateKeyFFI) -> Option<Zeroizing<Vec<u8>>> {
    if out.private_key_hex.is_null() || out.private_key_wif.is_null() {
        return None;
    }
    let hex = CStr::from_ptr(out.private_key_hex).to_bytes();
    let wif = CStr::from_ptr(out.private_key_wif).to_bytes();
    let hex_len = u8::try_from(hex.len()).ok()?;
    let wif_len = u8::try_from(wif.len()).ok()?;
    let mut blob = Zeroizing::new(Vec::with_capacity(2 + hex.len() + wif.len()));
    blob.push(hex_len);
    blob.extend_from_slice(hex);
    blob.push(wif_len);
    blob.extend_from_slice(wif);
    Some(blob)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn should_encode_hex_and_wif_with_length_prefixes() {
        let hex = CString::new("ab".repeat(32)).unwrap().into_raw();
        let wif = CString::new("cWIF").unwrap().into_raw();
        let mut out = AddressPrivateKeyFFI {
            private_key_hex: hex,
            private_key_wif: wif,
        };
        let blob = unsafe { encode_key(&out) }.unwrap();
        assert_eq!(blob[0], 64);
        assert_eq!(&blob[1..65], "ab".repeat(32).as_bytes());
        assert_eq!(blob[65], 4);
        assert_eq!(&blob[66..], b"cWIF");
        unsafe { platform_wallet_address_private_key_free(&mut out) };
        assert!(out.private_key_hex.is_null() && out.private_key_wif.is_null());
    }

    #[test]
    fn should_refuse_to_encode_an_empty_result() {
        let out = AddressPrivateKeyFFI {
            private_key_hex: ptr::null_mut(),
            private_key_wif: ptr::null_mut(),
        };
        assert!(unsafe { encode_key(&out) }.is_none());
    }
}
