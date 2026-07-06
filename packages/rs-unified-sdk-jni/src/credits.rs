//! JNI bridge for identity credit movements — transfer, withdraw, and
//! address-input top-up on the platform-wallet `IdentityWallet`.
//!
//! Kotlin counterpart: `org.dashfoundation.dashsdk.ffi.CreditsNative`,
//! driven by `org.dashfoundation.dashsdk.credits.IdentityCredits`.
//!
//! ## What lives here
//!
//! Each export is a thin marshaler over a SINGLE `platform-wallet-ffi`
//! entry point (no stitching, per `packages/kotlin-sdk/CLAUDE.md`):
//! - [`platform_wallet_transfer_credits_with_signer`] — identity → identity.
//! - [`platform_wallet_withdraw_credits_with_signer`] — identity → L1 address.
//! - [`platform_wallet_top_up_from_addresses_with_signer`] — top up an
//!   existing identity from Platform-address inputs (ID-06).
//! - [`platform_wallet_top_up_identity_with_funding_signer`] — top up an
//!   existing identity from a **new Core asset lock** (ID-05); this one
//!   takes a `MnemonicResolverHandle` (the Core asset-lock signer) rather
//!   than a plain `SignerHandle`.
//!
//! ## Result convention
//!
//! These entry points return `PlatformWalletFFIResult`, so errors go
//! through the shared [`crate::support::take_pwffi_error`] — the same
//! mapping `identity.rs` / `wallet_manager.rs` use.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jint, jlong};
use jni::JNIEnv;
use platform_wallet_ffi::handle::Handle;
use platform_wallet_ffi::identity_registration::IdentityFundingInputFFI;
use platform_wallet_ffi::identity_transfer::PlatformAddressCreditOutputFFI;
use rs_sdk_ffi::{MnemonicResolverHandle, SignerHandle};
use std::ffi::CString;

/// Read a required 32-byte id from a Java `byte[]`; throws + returns None
/// on the wrong length or a JNI error.
fn read_id32(env: &mut JNIEnv, arr: &JByteArray, field: &str) -> Option<[u8; 32]> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} byte[] was null/invalid"));
            return None;
        }
    };
    if bytes.len() != 32 {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} must be 32 bytes, got {}", bytes.len()),
        );
        return None;
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Some(id)
}

// ── Transfer (identity → identity) ─────────────────────────────────────

/// Transfer `amount` credits from `fromIdentityId` to `toIdentityId`,
/// signed by the identity's transfer-purpose key via `signerHandle`.
///
/// Wraps `platform_wallet_transfer_credits_with_signer`. No balance is
/// returned by the FFI (the sender's balance refreshes through the
/// persistence changeset); this returns void and throws on error.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_transferCredits(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    from_identity_id: JByteArray,
    to_identity_id: JByteArray,
    amount: jlong,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64 (and a clamped 0 would
        // post a meaningless 0-credit transition).
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        let Some(from_id) = read_id32(env, &from_identity_id, "fromIdentityId") else {
            return;
        };
        let Some(to_id) = read_id32(env, &to_identity_id, "toIdentityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_transfer_credits_with_signer(
                wallet_handle as Handle,
                from_id.as_ptr(),
                to_id.as_ptr(),
                amount as u64,
                signer_handle as *mut SignerHandle,
            )
        };
        take_pwffi_error(env, result);
    })
}

// ── Withdraw (identity → L1 Dash address) ──────────────────────────────

/// Withdraw `amount` credits from `identityId` to the Base58Check Dash
/// address `toAddress`, signed via `signerHandle`. The Rust side
/// validates the address against the wallet's network. The L1 payout is
/// pooled and broadcast asynchronously (no txid is returned); this
/// returns void and throws on error.
///
/// Wraps `platform_wallet_withdraw_credits_with_signer`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_withdrawCredits(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    amount: jlong,
    to_address: JString,
    signer_handle: jlong,
) {
    guard(&mut env, (), |env| {
        // Reject a non-positive amount at the boundary — a negative jlong
        // would otherwise bit-cast to a huge u64 (and a clamped 0 would
        // post a meaningless 0-credit withdrawal).
        if amount <= 0 {
            throw_sdk_exception(env, 1, "amount must be positive");
            return;
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let addr_str: String = match env.get_string(&to_address) {
            Ok(s) => s.into(),
            Err(_) => {
                let _ = env.exception_clear();
                throw_sdk_exception(env, 1, "toAddress string was null/invalid");
                return;
            }
        };
        let c_addr = match CString::new(addr_str) {
            Ok(c) => c,
            Err(_) => {
                throw_sdk_exception(env, 1, "toAddress contained an interior NUL");
                return;
            }
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_withdraw_credits_with_signer(
                wallet_handle as Handle,
                id.as_ptr(),
                amount as u64,
                c_addr.as_ptr(),
                signer_handle as *mut SignerHandle,
            )
        };
        take_pwffi_error(env, result);
    })
}

// ── Top up (existing identity ← Platform-address inputs) ───────────────

/// Top up an existing identity's credit balance from one or more
/// Platform-address inputs, signed per-address via `signerHandle` (the
/// platform-address signer). Returns the post-transition credit balance.
///
/// `inputsBlob` layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`.
///
/// Wraps `platform_wallet_top_up_from_addresses_with_signer`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_topUpFromAddresses(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    inputs_blob: JByteArray,
    signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };
        let Some(inputs) = decode_inputs_blob(env, &inputs_blob) else {
            return 0;
        };
        if inputs.is_empty() {
            throw_sdk_exception(env, 1, "inputsBlob contained no funding inputs");
            return 0;
        }

        let mut out_balance: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_top_up_from_addresses_with_signer(
                wallet_handle as Handle,
                &id as *const [u8; 32],
                inputs.as_ptr(),
                inputs.len(),
                signer_handle as *mut SignerHandle,
                &mut out_balance as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

// ── Transfer (identity → Platform addresses) ───────────────────────────

/// Transfer credits from `fromIdentityId` (32 bytes) to one or more
/// Platform-address recipients, signed by the identity's transfer-purpose
/// key via `signerHandle`. Returns the sender's post-transfer credit
/// balance.
///
/// `outputsBlob` layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits` — the same
/// row shape `topUpFromAddresses` consumes.
///
/// Wraps `platform_wallet_transfer_credits_to_addresses_with_signer`, which
/// also reconciles any wallet-owned recipient addresses' platform balances
/// from the transfer proof (third-party recipients are skipped).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_transferCreditsToAddresses(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    from_identity_id: JByteArray,
    outputs_blob: JByteArray,
    signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        let Some(from_id) = read_id32(env, &from_identity_id, "fromIdentityId") else {
            return 0;
        };
        let Some(rows) = decode_credit_rows(env, &outputs_blob, "outputsBlob") else {
            return 0;
        };
        if rows.is_empty() {
            throw_sdk_exception(env, 1, "outputsBlob contained no recipients");
            return 0;
        }
        let outputs: Vec<PlatformAddressCreditOutputFFI> = rows
            .into_iter()
            .map(
                |(address_type, hash, credits)| PlatformAddressCreditOutputFFI {
                    address_type,
                    hash,
                    credits,
                },
            )
            .collect();

        let mut out_balance: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_transfer_credits_to_addresses_with_signer(
                wallet_handle as Handle,
                from_id.as_ptr(),
                outputs.as_ptr(),
                outputs.len(),
                signer_handle as *mut SignerHandle,
                &mut out_balance as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

// ── Top up (existing identity ← new Core asset lock) ───────────────────

/// Top up an existing identity's credit balance by building and
/// broadcasting a **new Core asset lock** — the same funding mechanism as
/// identity registration (ID-05), distinct from `topUpFromAddresses`
/// (ID-06) which spends already-funded Platform-payment addresses.
///
/// `amountDuffs` is the Dash amount (in duffs) to lock; `accountIndex`
/// selects which BIP44 standard account funds the asset lock.
/// `coreSignerHandle` is the manager's `MnemonicResolverHandle` — the
/// `IdentityTopUp` transition is signed entirely by the asset lock's
/// Core key, so no identity signer is required. Returns the
/// post-transition credit balance.
///
/// Wraps `platform_wallet_top_up_identity_with_funding_signer`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_topUpIdentityFromCore(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    amount_duffs: jlong,
    account_index: jint,
    core_signer_handle: jlong,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        // Reject sign errors at the boundary — a negative amount / index
        // would otherwise bit-cast to a huge unsigned value (and a clamped
        // 0 amount would post a meaningless 0-duff top-up).
        if amount_duffs <= 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be positive");
            return 0;
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return 0;
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };

        let mut out_balance: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_top_up_identity_with_funding_signer(
                wallet_handle as Handle,
                &id as *const [u8; 32],
                amount_duffs as u64,
                account_index as u32,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_balance as *mut u64,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

/// Decode a credit-rows BLOB into owned `(address_type, hash, credits)`
/// tuples. Layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`. Throws +
/// returns None on a malformed blob or an out-of-range (sign-bit-set)
/// credit amount. [`field`] names the blob in error messages
/// (`"inputsBlob"` / `"outputsBlob"`).
///
/// Shared by the top-up (ID-06) and transfer-to-addresses (ID-11) credit
/// exports here and the register-from-addresses (ID-08) export in
/// `identity.rs` — every one marshals the same flat row shape that
/// [`IdentityFundingInputFFI`] / [`PlatformAddressCreditOutputFFI`] expose.
pub(crate) fn decode_credit_rows(
    env: &mut JNIEnv,
    arr: &JByteArray,
    field: &str,
) -> Option<Vec<(u8, [u8; 20], u64)>> {
    let bytes = match env.convert_byte_array(arr) {
        Ok(b) => b,
        Err(_) => {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, &format!("{field} was null/invalid"));
            return None;
        }
    };
    let mut cursor = 0usize;
    let read = |cursor: &mut usize, n: usize| -> Option<&[u8]> {
        if *cursor + n > bytes.len() {
            return None;
        }
        let s = &bytes[*cursor..*cursor + n];
        *cursor += n;
        Some(s)
    };
    let Some(count_bytes) = read(&mut cursor, 4) else {
        throw_sdk_exception(env, 1, &format!("{field} truncated (row count)"));
        return None;
    };
    let count = u32::from_be_bytes(count_bytes.try_into().ok()?) as usize;
    // Reject an impossible row count before allocating: rows are a fixed
    // 29 bytes, so a header claiming more than the remaining payload can
    // hold is malformed. Guards the raw-JNI boundary against a huge
    // `with_capacity` abort on a short/hostile blob (the per-row reads
    // below still throw precise truncation errors for valid-sized claims).
    if count
        .checked_mul(29)
        .is_none_or(|need| bytes.len() - cursor < need)
    {
        throw_sdk_exception(
            env,
            1,
            &format!("{field} claims {count} rows but body is too short"),
        );
        return None;
    }
    let mut rows = Vec::with_capacity(count);
    for i in 0..count {
        let Some(type_byte) = read(&mut cursor, 1) else {
            throw_sdk_exception(env, 1, &format!("{field} truncated at row {i} type"));
            return None;
        };
        let address_type = type_byte[0];
        let Some(hash_bytes) = read(&mut cursor, 20) else {
            throw_sdk_exception(env, 1, &format!("{field} truncated at row {i} hash"));
            return None;
        };
        let mut hash = [0u8; 20];
        hash.copy_from_slice(hash_bytes);
        let Some(credits_bytes) = read(&mut cursor, 8) else {
            throw_sdk_exception(env, 1, &format!("{field} truncated at row {i} credits"));
            return None;
        };
        let credits = u64::from_be_bytes(credits_bytes.try_into().ok()?);
        // The Kotlin encoder writes this field from a signed long; a set
        // sign bit means a negative amount crossed the boundary — reject
        // rather than treat it as a huge unsigned credit value.
        if credits & (1 << 63) != 0 {
            throw_sdk_exception(
                env,
                1,
                &format!("{field} row {i} credits amount out of range"),
            );
            return None;
        }
        rows.push((address_type, hash, credits));
    }
    Some(rows)
}

/// Decode the top-up inputs BLOB into owned [`IdentityFundingInputFFI`]
/// rows (the ID-06 shape). Thin mapper over [`decode_credit_rows`]; the
/// struct is `Copy`, so the returned `Vec` fully owns its rows.
fn decode_inputs_blob(env: &mut JNIEnv, arr: &JByteArray) -> Option<Vec<IdentityFundingInputFFI>> {
    decode_credit_rows(env, arr, "inputsBlob").map(|rows| {
        rows.into_iter()
            .map(|(address_type, hash, credits)| IdentityFundingInputFFI {
                address_type,
                hash,
                credits,
            })
            .collect()
    })
}
