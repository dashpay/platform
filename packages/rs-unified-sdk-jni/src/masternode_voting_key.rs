//! JNI exports for the masternode voting private key a host stores: forming its voting
//! address, and reading a key a person typed. Each export marshals ONE C function of
//! `rs-sdk-ffi`:
//!
//! * `votingAddressFromPrivateKey` -> `dash_sdk_masternode_voting_address_from_private_key`
//!   (Swift `KeychainSigner.masternodeVotingAddress`),
//! * `privateKeyBytesFromText` -> `dash_sdk_private_key_bytes_from_text`
//!   (Swift `KeyValidation.privateKeyBytes(fromText:)`).
//!
//! Every private key, and the text that may hold one, is held in a buffer zeroed on drop;
//! the JVM copies they come from and go to are the Kotlin caller's to zero. A refusal throws
//! `DashSDKException` with the C function's code and reason, which never contains the key
//! or the text.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative`.

use crate::decision_proposals::read_key32_zeroizing;
use crate::results::take_error;
use crate::support::{guard, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::jbyteArray;
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_masternode_voting_address_from_private_key, dash_sdk_private_key_bytes_from_text,
};
use std::os::raw::c_char;
use std::ptr;
use zeroize::Zeroizing;

/// The 20-byte voting address of a 32-byte masternode voting private key: the hash160 of
/// its compressed public key, the `votingAddress` the `castVoteWithSigner` exports take.
/// Null after throwing (a key that is not 32 bytes, or not a secp256k1 secret key).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_votingAddressFromPrivateKey(
    mut env: JNIEnv,
    _class: JClass,
    private_key: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(key) = read_key32_zeroizing(env, &private_key, "privateKey") else {
            return ptr::null_mut();
        };
        let mut address = [0u8; 20];
        let result = unsafe {
            dash_sdk_masternode_voting_address_from_private_key(key.as_ptr(), address.as_mut_ptr())
        };
        drop(key);
        if unsafe { take_error(env, &result) } {
            return ptr::null_mut();
        }
        env.byte_array_from_slice(&address)
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// The 32 secret bytes of a private key typed as 64 hex characters or in compressed WIF.
/// `simple_signer::private_key_from_text` decides which forms are accepted.
///
/// `textUtf8` is the text as UTF-8 bytes, never a `java.lang.String` (which cannot be
/// scrubbed); the caller zeroes it after the call. It is read into a buffer zeroed on drop
/// and NUL-terminated there. Null after throwing: a refusal carries the C function's reason
/// (`InvalidParameter`), which never contains the text, and text holding a NUL byte is
/// refused here, since a C string would end at it.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_privateKeyBytesFromText(
    mut env: JNIEnv,
    _class: JClass,
    text_utf8: JByteArray,
) -> jbyteArray {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(text) = read_text_zeroizing(env, &text_utf8) else {
            return ptr::null_mut();
        };
        let mut secret = Zeroizing::new([0u8; 32]);
        let result = unsafe {
            dash_sdk_private_key_bytes_from_text(
                text.as_ptr().cast::<c_char>(),
                secret.as_mut_ptr(),
            )
        };
        drop(text);
        if unsafe { take_error(env, &result) } {
            return ptr::null_mut();
        }
        env.byte_array_from_slice(secret.as_slice())
            .map(|a| a.into_raw())
            .unwrap_or(ptr::null_mut())
    })
}

/// The bytes of `text_utf8` followed by a NUL, in a buffer zeroed on drop. Throws and
/// returns `None` for a null array, or for text holding a NUL byte.
///
/// The buffer is allocated once with room for the terminator and filled straight from the
/// JVM array (`get_byte_array_region`), so no reallocation leaves an unscrubbed copy behind.
fn read_text_zeroizing(env: &mut JNIEnv, text_utf8: &JByteArray) -> Option<Zeroizing<Vec<u8>>> {
    let len = match env.get_array_length(text_utf8) {
        Ok(len) if len >= 0 => len as usize,
        _ => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "textUtf8 byte[] was null/invalid");
            return None;
        }
    };
    let mut text: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::with_capacity(len + 1));
    text.resize(len, 0);
    {
        // jbyte is i8 and the buffer u8: the same layout.
        let dst = unsafe { std::slice::from_raw_parts_mut(text.as_mut_ptr().cast::<i8>(), len) };
        if env.get_byte_array_region(text_utf8, 0, dst).is_err() {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "textUtf8 byte[] was null/invalid");
            return None;
        }
    }
    if text.contains(&0) {
        throw_sdk_exception(
            env,
            1,
            "the private key contains a NUL character, which a C string cannot carry",
        );
        return None;
    }
    // Capacity is len + 1, so this push cannot reallocate.
    text.push(0);
    Some(text)
}

#[cfg(test)]
mod tests {
    use crate::decision_proposals::tests::kotlin_signature_in;
    use dashcore::{Network, PrivateKey};
    use rs_sdk_ffi::{
        dash_sdk_error_free, dash_sdk_masternode_voting_address_from_private_key,
        dash_sdk_private_key_bytes_from_text,
    };
    use std::ffi::CString;

    const MASTERNODE_VOTING_KEY_NATIVE_KT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../kotlin-sdk/sdk/src/main/kotlin/org/dashfoundation/dashsdk/ffi/MasternodeVotingKeyNative.kt"
    ));

    /// The secret 1: the smallest secp256k1 key, whose compressed public key and hash160 are
    /// published vectors. `MasternodeVotingKeyFfiTest` (sdk/src/androidTest) uses the same
    /// three constants, so a change here must change them too.
    const KEY_ONE_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000001";
    /// hash160 of `0279be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798`.
    const KEY_ONE_VOTING_ADDRESS_HEX: &str = "751e76e8199196d454941c45d1b3a323f1433bd6";
    /// The secret 1 as a compressed testnet WIF.
    const KEY_ONE_TESTNET_WIF: &str = "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA";

    fn to_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn key_one() -> [u8; 32] {
        let mut key = [0u8; 32];
        key[31] = 1;
        key
    }

    /// The secret the C function reads from `text`, or `None` when it refuses it.
    fn secret_from_text(text: &str) -> Option<[u8; 32]> {
        let text = CString::new(text).expect("no NUL");
        let mut out = [0u8; 32];
        unsafe {
            let result = dash_sdk_private_key_bytes_from_text(text.as_ptr(), out.as_mut_ptr());
            if result.error.is_null() {
                Some(out)
            } else {
                dash_sdk_error_free(result.error);
                None
            }
        }
    }

    /// The vectors the instrumented Kotlin test relies on hold for the C functions this
    /// module marshals.
    #[test]
    fn should_hold_the_key_one_vectors_the_kotlin_test_uses() {
        assert_eq!(to_hex(&key_one()), KEY_ONE_HEX);

        let mut address = [0u8; 20];
        let result = unsafe {
            dash_sdk_masternode_voting_address_from_private_key(
                key_one().as_ptr(),
                address.as_mut_ptr(),
            )
        };
        assert!(result.error.is_null());
        assert_eq!(to_hex(&address), KEY_ONE_VOTING_ADDRESS_HEX);

        let wif = PrivateKey::from_byte_array(&key_one(), Network::Testnet)
            .expect("a secret key")
            .to_wif();
        assert_eq!(wif, KEY_ONE_TESTNET_WIF);

        assert_eq!(secret_from_text(KEY_ONE_HEX), Some(key_one()));
        assert_eq!(secret_from_text(KEY_ONE_TESTNET_WIF), Some(key_one()));
        assert_eq!(secret_from_text("not a key"), None);
    }

    #[test]
    fn should_declare_the_key_helpers_as_the_exports_take_them() {
        assert_eq!(
            kotlin_signature_in(
                MASTERNODE_VOTING_KEY_NATIVE_KT,
                "votingAddressFromPrivateKey"
            ),
            (vec!["ByteArray".to_string()], "ByteArray".to_string())
        );
        assert_eq!(
            kotlin_signature_in(MASTERNODE_VOTING_KEY_NATIVE_KT, "privateKeyBytesFromText"),
            (vec!["ByteArray".to_string()], "ByteArray".to_string())
        );
    }
}
