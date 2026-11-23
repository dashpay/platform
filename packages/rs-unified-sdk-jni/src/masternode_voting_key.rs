//! JNI exports for the masternode voting private key a host stores: forming its voting
//! address, reading a key a person typed, and checking a key against Platform with the rules
//! for what a host then does with it. Each export marshals ONE C function of `rs-sdk-ffi`:
//!
//! * `votingAddressFromPrivateKey` -> `dash_sdk_masternode_voting_address_from_private_key`
//!   (Swift `KeychainSigner.masternodeVotingAddress`),
//! * `privateKeyBytesFromText` -> `dash_sdk_private_key_bytes_from_text`
//!   (Swift `KeyValidation.privateKeyBytes(fromText:)`),
//! * `checkVotingKey` -> `dash_sdk_masternode_check_voting_key`
//!   (Swift `SDK.checkMasternodeVotingKey(proTxHash:votingAddress:)`),
//! * `votingKeyCheckOutcome` -> `dash_sdk_masternode_voting_key_check_outcome`
//!   (Swift `MasternodeVotingKeyCheck.outcome(votingAddress:slot:)`),
//! * `recordedVotingKeyUse` -> `dash_sdk_masternode_recorded_voting_key_use`
//!   (Swift `MasternodeRecordedVotingKeyUse.init(recorded:storedVotingAddress:)`),
//! * `loadForVoting` -> `dash_sdk_masternode_load_for_voting`
//!   (Swift `SDK.loadMasternodeForVoting(proTxHash:votingAddress:)`).
//!
//! rs-sdk decides every verdict and rule (`masternode_vote_keys.rs`); the last four exports
//! only read JVM values into the C types, call the function and write its result as compact
//! JSON. An enum value the C side does not declare is refused (code 1), never cast.
//!
//! Every private key, and the text that may hold one, is held in a buffer zeroed on drop;
//! the JVM copies they come from and go to are the Kotlin caller's to zero. A refusal throws
//! `DashSDKException` with the C function's code and reason, which never contains the key
//! or the text.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative`.

use crate::decision_proposals::{read_fixed_bytes, read_key32_zeroizing};
use crate::dpns_marketplace::new_jstring;
use crate::results::take_error;
use crate::support::{guard, throw_sdk_exception};
use jni::objects::{JByteArray, JClass};
use jni::sys::{jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;
use rs_sdk_ffi::{
    dash_sdk_masternode_check_voting_key, dash_sdk_masternode_load_for_voting,
    dash_sdk_masternode_recorded_voting_key_use,
    dash_sdk_masternode_voting_address_from_private_key,
    dash_sdk_masternode_voting_key_check_outcome, dash_sdk_private_key_bytes_from_text,
    dash_sdk_string_free, DashSDKMasternodeVotingLoad, DashSDKMasternodeVotingLoadKind,
    DashSDKRecordedVotingKey, DashSDKRecordedVotingKeyCheck, DashSDKVotingAddress,
    DashSDKVotingKeySlot, DashSDKVotingKeyVerdict, SDKHandle,
};
use serde_json::json;
use std::ffi::CStr;
use std::mem::MaybeUninit;
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

/// Whether Platform's state lets the voting key with `votingAddress` (20 bytes) vote for the
/// masternode `wireOrderProTxHash` (32 bytes, wire order, as every vote export takes it), by
/// one proof-verified fetch of its voter identity: `dash_sdk_masternode_check_voting_key`.
///
/// Returns `{"verdict":0|1|2,"message":string|null}` with the C verdict (`Valid`, `Invalid`,
/// `Undetermined`) and, for the last two, the sentence Rust wrote for the user. A failed fetch
/// is `Undetermined`, never a throw. Null after throwing: a null handle or a value of the
/// wrong length (code 1, the C function's `InvalidParameter` for the handle).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_checkVotingKey(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    wire_order_pro_tx_hash: JByteArray,
    voting_address: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(pro_tx_hash) =
            read_fixed_bytes::<32>(env, &wire_order_pro_tx_hash, "wireOrderProTxHash")
        else {
            return ptr::null_mut();
        };
        let Some(address) = read_fixed_bytes::<20>(env, &voting_address, "votingAddress") else {
            return ptr::null_mut();
        };
        let mut verdict = DashSDKVotingKeyVerdict::Undetermined;
        let result = unsafe {
            dash_sdk_masternode_check_voting_key(
                sdk as *const SDKHandle,
                pro_tx_hash.as_ptr(),
                address.as_ptr(),
                &mut verdict,
            )
        };
        if unsafe { take_error(env, &result) } {
            return ptr::null_mut();
        }
        // On success `data` is null (`Valid`) or the verdict's C string, freed here only.
        let message = if result.data.is_null() {
            None
        } else {
            let pointer = result.data as *mut c_char;
            let message = unsafe { CStr::from_ptr(pointer) }
                .to_string_lossy()
                .into_owned();
            unsafe { dash_sdk_string_free(pointer) };
            Some(message)
        };
        new_jstring(env, check_json(verdict, message))
    })
}

fn check_json(verdict: DashSDKVotingKeyVerdict, message: Option<String>) -> String {
    json!({ "verdict": verdict as i32, "message": message }).to_string()
}

/// What a host does with a voting key once checked, given what its voting key slot holds:
/// `dash_sdk_masternode_voting_key_check_outcome`, which calls rs-sdk's
/// `voting_key_check_outcome`.
///
/// `verdict` is the C verdict (0..2), `slot` the C slot (`Empty` 0, `SameKey` 1,
/// `DifferentKey` 2), `recordedCheck` the C recorded check (`None` 0 .. `Undetermined` 3) of
/// the record the host holds, with `recordedVotingAddress` the address it was made of: null
/// when `recordedCheck` is 0. The record is read by Rust only for `SameKey`.
///
/// Returns `{"storeKey":bool,"record":null|{"check":1|2|3,"votingAddress":hex}}`: a null
/// record leaves what is recorded unchanged. Null after throwing code 1 for an undeclared
/// enum value or an address of the wrong length.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_votingKeyCheckOutcome(
    mut env: JNIEnv,
    _class: JClass,
    verdict: jint,
    voting_address: JByteArray,
    slot: jint,
    recorded_check: jint,
    recorded_voting_address: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(verdict) = verdict_from(env, verdict) else {
            return ptr::null_mut();
        };
        let Some(address) = read_fixed_bytes::<20>(env, &voting_address, "votingAddress") else {
            return ptr::null_mut();
        };
        let Some(slot) = slot_from(env, slot) else {
            return ptr::null_mut();
        };
        let Some(recorded) =
            recorded_from(env, recorded_check, &recorded_voting_address, "recorded")
        else {
            return ptr::null_mut();
        };
        let outcome = dash_sdk_masternode_voting_key_check_outcome(
            verdict,
            DashSDKVotingAddress { bytes: address },
            slot,
            recorded,
        );
        new_jstring(env, outcome_json(outcome.store_key, outcome.record))
    })
}

fn outcome_json(store_key: bool, record: DashSDKRecordedVotingKey) -> String {
    let record = match record.check {
        DashSDKRecordedVotingKeyCheck::None => serde_json::Value::Null,
        check => json!({
            "check": check as i32,
            "votingAddress": to_hex(&record.voting_address.bytes),
        }),
    };
    json!({ "storeKey": store_key, "record": record }).to_string()
}

/// How a host may use a stored voting key, from the record it holds:
/// `dash_sdk_masternode_recorded_voting_key_use`, which calls rs-sdk's
/// `recorded_voting_key_use`.
///
/// `recordedCheck` and `recordedVotingAddress` as for `votingKeyCheckOutcome`;
/// `storedVotingAddress` is the 20-byte address of the stored key, or null when the host did
/// not read it.
///
/// Returns `{"canVote":bool,"checkBeforeUse":bool}`. Null after throwing code 1 for an
/// undeclared enum value or an address of the wrong length.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_recordedVotingKeyUse(
    mut env: JNIEnv,
    _class: JClass,
    recorded_check: jint,
    recorded_voting_address: JByteArray,
    stored_voting_address: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(recorded) =
            recorded_from(env, recorded_check, &recorded_voting_address, "recorded")
        else {
            return ptr::null_mut();
        };
        let stored = if stored_voting_address.is_null() {
            None
        } else {
            let Some(stored) =
                read_fixed_bytes::<20>(env, &stored_voting_address, "storedVotingAddress")
            else {
                return ptr::null_mut();
            };
            Some(stored)
        };
        let key_use = dash_sdk_masternode_recorded_voting_key_use(
            recorded,
            stored.is_some(),
            DashSDKVotingAddress {
                bytes: stored.unwrap_or([0u8; 20]),
            },
        );
        new_jstring(
            env,
            json!({ "canVote": key_use.can_vote, "checkBeforeUse": key_use.check_before_use })
                .to_string(),
        )
    })
}

/// Loads the masternode `wireOrderProTxHash` (32 bytes, wire order) for voting, with the
/// voting key of `votingAddress` (20 bytes) when one is given (null otherwise):
/// `dash_sdk_masternode_load_for_voting`, which calls rs-sdk's `load_masternode_for_voting`
/// and holds the rule (the owner identity when Platform holds one, else the voter identity
/// when Platform proves the owner absent and the voter holding the enabled voting key).
///
/// Returns `{"kind":0|1|2,"identityId":hex|null,"identityJson":string|null,
/// "votingKeyVerdict":0|1|2|null,"message":string|null}`: kind `OwnerIdentity`,
/// `VoterIdentity` or `Refused`, the loaded identity's id and JSON (the shape
/// `dash_sdk_identity_fetch` returns), the voting key's verdict when it was checked, and the
/// sentence Rust wrote for a refusal or an `Invalid` or `Undetermined` verdict. A `Refused`
/// load carries an id only with a verdict (always `Invalid`): the voter identity the voting key
/// was refused for. Null after
/// throwing: a value of the wrong length (code 1), or the C function's error when nothing is
/// proved (the owner identity, or the voter identity after a proved-absent owner, could not
/// be fetched).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_MasternodeVotingKeyNative_loadForVoting(
    mut env: JNIEnv,
    _class: JClass,
    sdk: jlong,
    wire_order_pro_tx_hash: JByteArray,
    voting_address: JByteArray,
) -> jstring {
    guard(&mut env, ptr::null_mut(), |env| {
        let Some(pro_tx_hash) =
            read_fixed_bytes::<32>(env, &wire_order_pro_tx_hash, "wireOrderProTxHash")
        else {
            return ptr::null_mut();
        };
        let address = if voting_address.is_null() {
            None
        } else {
            let Some(address) = read_fixed_bytes::<20>(env, &voting_address, "votingAddress")
            else {
                return ptr::null_mut();
            };
            Some(address)
        };
        let mut out = MaybeUninit::<DashSDKMasternodeVotingLoad>::uninit();
        let result = unsafe {
            dash_sdk_masternode_load_for_voting(
                sdk as *const SDKHandle,
                pro_tx_hash.as_ptr(),
                address.is_some(),
                DashSDKVotingAddress {
                    bytes: address.unwrap_or([0u8; 20]),
                },
                out.as_mut_ptr(),
            )
        };
        // On an error nothing was written to `out`.
        if unsafe { take_error(env, &result) } {
            return ptr::null_mut();
        }
        // SAFETY: on success the C function wrote the whole struct, whose two strings are
        // ours to free, and freed here only.
        let load = unsafe { out.assume_init() };
        let identity_json = unsafe { take_c_string(load.identity_json) };
        let message = unsafe { take_c_string(load.message) };
        new_jstring(
            env,
            load_json(
                load.kind,
                load.identity_id,
                identity_json,
                load.has_voting_key_verdict
                    .then_some(load.voting_key_verdict),
                message,
            ),
        )
    })
}

/// The Rust string at `pointer`, freed; `None` for null.
///
/// # Safety
/// `pointer` must be null or a C string the FFI allocated and nothing else frees.
unsafe fn take_c_string(pointer: *mut c_char) -> Option<String> {
    if pointer.is_null() {
        return None;
    }
    let text = CStr::from_ptr(pointer).to_string_lossy().into_owned();
    dash_sdk_string_free(pointer);
    Some(text)
}

fn load_json(
    kind: DashSDKMasternodeVotingLoadKind,
    identity_id: [u8; 32],
    identity_json: Option<String>,
    verdict: Option<DashSDKVotingKeyVerdict>,
    message: Option<String>,
) -> String {
    // A refusal names an identity only when the voting key was refused for one.
    let names_identity = kind != DashSDKMasternodeVotingLoadKind::Refused || verdict.is_some();
    json!({
        "kind": kind as i32,
        "identityId": names_identity.then(|| to_hex(&identity_id)),
        "identityJson": identity_json,
        "votingKeyVerdict": verdict.map(|verdict| verdict as i32),
        "message": message,
    })
    .to_string()
}

/// The C verdict for `value`; throws code 1 for a value the C enum does not declare.
fn verdict_from(env: &mut JNIEnv, value: jint) -> Option<DashSDKVotingKeyVerdict> {
    match value {
        0 => Some(DashSDKVotingKeyVerdict::Valid),
        1 => Some(DashSDKVotingKeyVerdict::Invalid),
        2 => Some(DashSDKVotingKeyVerdict::Undetermined),
        other => {
            throw_sdk_exception(
                env,
                1,
                &format!("verdict {other} is not a voting key verdict"),
            );
            None
        }
    }
}

/// The C slot for `value`; throws code 1 for a value the C enum does not declare.
fn slot_from(env: &mut JNIEnv, value: jint) -> Option<DashSDKVotingKeySlot> {
    match value {
        0 => Some(DashSDKVotingKeySlot::Empty),
        1 => Some(DashSDKVotingKeySlot::SameKey),
        2 => Some(DashSDKVotingKeySlot::DifferentKey),
        other => {
            throw_sdk_exception(env, 1, &format!("slot {other} is not a voting key slot"));
            None
        }
    }
}

/// The C record for `check` and `address`: `{0}` for no record (check 0, whose address must
/// be null), else the check with its 20-byte address. Throws code 1 for an undeclared check, a
/// record without an address, or no record with one.
fn recorded_from(
    env: &mut JNIEnv,
    check: jint,
    address: &JByteArray,
    field: &str,
) -> Option<DashSDKRecordedVotingKey> {
    let check = match check {
        0 => DashSDKRecordedVotingKeyCheck::None,
        1 => DashSDKRecordedVotingKeyCheck::Valid,
        2 => DashSDKRecordedVotingKeyCheck::Invalid,
        3 => DashSDKRecordedVotingKeyCheck::Undetermined,
        other => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field}Check {other} is not a recorded voting key check"),
            );
            return None;
        }
    };
    let bytes = match (check, address.is_null()) {
        (DashSDKRecordedVotingKeyCheck::None, true) => [0u8; 20],
        (DashSDKRecordedVotingKeyCheck::None, false) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field}VotingAddress must be null when nothing is recorded"),
            );
            return None;
        }
        (_, true) => {
            throw_sdk_exception(
                env,
                1,
                &format!("{field}VotingAddress must be set for a recorded check"),
            );
            return None;
        }
        (_, false) => read_fixed_bytes::<20>(env, address, &format!("{field}VotingAddress"))?,
    };
    Some(DashSDKRecordedVotingKey {
        check,
        voting_address: DashSDKVotingAddress { bytes },
    })
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{check_json, load_json, outcome_json};
    use crate::decision_proposals::tests::kotlin_signature_in;
    use dashcore::{Network, PrivateKey};
    use rs_sdk_ffi::{
        dash_sdk_error_free, dash_sdk_masternode_voting_address_from_private_key,
        dash_sdk_private_key_bytes_from_text, DashSDKMasternodeVotingLoadKind,
        DashSDKRecordedVotingKey, DashSDKRecordedVotingKeyCheck, DashSDKVotingAddress,
        DashSDKVotingKeyVerdict,
    };
    use serde_json::{json, Value};
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

    #[test]
    fn should_declare_the_voting_key_check_as_the_exports_take_it() {
        assert_eq!(
            kotlin_signature_in(MASTERNODE_VOTING_KEY_NATIVE_KT, "checkVotingKey"),
            (
                vec![
                    "Long".to_string(),      // sdk
                    "ByteArray".to_string(), // wire_order_pro_tx_hash
                    "ByteArray".to_string(), // voting_address
                ],
                "String?".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(MASTERNODE_VOTING_KEY_NATIVE_KT, "votingKeyCheckOutcome"),
            (
                vec![
                    "Int".to_string(),        // verdict
                    "ByteArray".to_string(),  // voting_address
                    "Int".to_string(),        // slot
                    "Int".to_string(),        // recorded_check
                    "ByteArray?".to_string(), // recorded_voting_address
                ],
                "String?".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(MASTERNODE_VOTING_KEY_NATIVE_KT, "loadForVoting"),
            (
                vec![
                    "Long".to_string(),       // sdk
                    "ByteArray".to_string(),  // wire_order_pro_tx_hash
                    "ByteArray?".to_string(), // voting_address
                ],
                "String?".to_string()
            )
        );
        assert_eq!(
            kotlin_signature_in(MASTERNODE_VOTING_KEY_NATIVE_KT, "recordedVotingKeyUse"),
            (
                vec![
                    "Int".to_string(),        // recorded_check
                    "ByteArray?".to_string(), // recorded_voting_address
                    "ByteArray?".to_string(), // stored_voting_address
                ],
                "String?".to_string()
            )
        );
    }

    /// The JSON the Kotlin decoder (`MasternodeVotingKeyChecks`) reads: the C enum values, a
    /// null message for a valid key and a null record for "leave it unchanged". Compared as
    /// values, since key order depends on serde_json's features in the build.
    #[test]
    fn should_write_the_json_the_kotlin_decoder_reads() {
        let parsed = |text: String| -> Value { serde_json::from_str(&text).expect("JSON") };
        assert_eq!(
            parsed(check_json(DashSDKVotingKeyVerdict::Valid, None)),
            json!({ "verdict": 0, "message": null })
        );
        assert_eq!(
            parsed(check_json(
                DashSDKVotingKeyVerdict::Invalid,
                Some("no \"key\"".to_string())
            )),
            json!({ "verdict": 1, "message": "no \"key\"" })
        );
        assert_eq!(
            parsed(check_json(
                DashSDKVotingKeyVerdict::Undetermined,
                Some("timeout".to_string())
            )),
            json!({ "verdict": 2, "message": "timeout" })
        );
        let none = DashSDKRecordedVotingKey {
            check: DashSDKRecordedVotingKeyCheck::None,
            voting_address: DashSDKVotingAddress { bytes: [0u8; 20] },
        };
        assert_eq!(
            parsed(outcome_json(true, none)),
            json!({ "storeKey": true, "record": null })
        );
        let invalid = DashSDKRecordedVotingKey {
            check: DashSDKRecordedVotingKeyCheck::Invalid,
            voting_address: DashSDKVotingAddress { bytes: [0xab; 20] },
        };
        assert_eq!(
            parsed(outcome_json(false, invalid)),
            json!({
                "storeKey": false,
                "record": { "check": 2, "votingAddress": "ab".repeat(20) },
            })
        );
    }

    /// The load JSON `MasternodeVotingKeyCheckCodec.decodeLoad` reads: no id for a refusal,
    /// and no verdict when no key was checked.
    #[test]
    fn should_write_the_load_json_the_kotlin_decoder_reads() {
        let parsed = |text: String| -> Value { serde_json::from_str(&text).expect("JSON") };
        assert_eq!(
            parsed(load_json(
                DashSDKMasternodeVotingLoadKind::VoterIdentity,
                [0xcd; 32],
                Some("{\"id\":\"x\"}".to_string()),
                Some(DashSDKVotingKeyVerdict::Valid),
                None,
            )),
            json!({
                "kind": 1,
                "identityId": "cd".repeat(32),
                "identityJson": "{\"id\":\"x\"}",
                "votingKeyVerdict": 0,
                "message": null,
            })
        );
        assert_eq!(
            parsed(load_json(
                DashSDKMasternodeVotingLoadKind::Refused,
                [0u8; 32],
                None,
                None,
                Some("no owner".to_string()),
            )),
            json!({
                "kind": 2,
                "identityId": null,
                "identityJson": null,
                "votingKeyVerdict": null,
                "message": "no owner",
            })
        );
        // A voting key refused for a voter identity names that identity, as invalid.
        assert_eq!(
            parsed(load_json(
                DashSDKMasternodeVotingLoadKind::Refused,
                [0xef; 32],
                None,
                Some(DashSDKVotingKeyVerdict::Invalid),
                Some("rotated".to_string()),
            )),
            json!({
                "kind": 2,
                "identityId": "ef".repeat(32),
                "identityJson": null,
                "votingKeyVerdict": 1,
                "message": "rotated",
            })
        );
    }
}
