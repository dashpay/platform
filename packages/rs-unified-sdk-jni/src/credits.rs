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
//! - [`platform_wallet_top_up_external_identity_with_funding_signer`] — top
//!   up an identity this wallet does not manage from a new Core asset lock;
//!   the lock records the identity it pays.
//! - [`platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`]
//!   — resume a tracked identity top-up lock to the identity it pays (no
//!   identity argument).
//! - [`platform_wallet_assign_identity_top_up_recipient`] — assign the
//!   identity an unassigned unbound top-up lock pays.
//!
//! ## Result convention
//!
//! These entry points return `PlatformWalletFFIResult`, so errors go
//! through the shared [`crate::support::take_pwffi_error`] — the same
//! mapping `identity.rs` / `wallet_manager.rs` use.

#![allow(clippy::missing_safety_doc)]

use crate::support::{guard, take_pwffi_error, throw_sdk_exception};
use jni::objects::{JByteArray, JClass, JString, JValue};
use jni::sys::{jboolean, jint, jlong, jobject};
use jni::JNIEnv;
use platform_wallet_ffi::core_wallet_types::OutPointFFI;
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
        // Zero decoded rows go through: the wallet refuses them as an
        // invalid parameter after its handle and generation checks.
        let Some(inputs) = decode_inputs_blob(env, &inputs_blob) else {
            return 0;
        };

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

/// Resume an interrupted top-up from a tracked Core asset-lock outpoint —
/// or, with `consumeInvitationVoucher = true`, reclaim an unclaimed DIP-13
/// invitation voucher as credits on an existing identity. The flag is
/// forwarded verbatim: Rust core's funding resolver refuses
/// invitation-typed locks on every generic path and consumes them only
/// when the flag is set, so authorization lives below this binding.
/// Generic recovery call sites always pass `false`.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_topUpIdentityWithExistingAssetLock(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    outpoint_txid: JByteArray,
    outpoint_vout: jint,
    identity_id: JByteArray,
    core_signer_handle: jlong,
    consume_invitation_voucher: jboolean,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        let Some(outpoint) = read_outpoint(env, &outpoint_txid, outpoint_vout) else {
            return 0;
        };
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };
        if core_signer_handle == 0 {
            throw_sdk_exception(env, 1, "coreSignerHandle must be non-zero");
            return 0;
        }
        let mut out_balance = 0u64;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_topup_identity_with_existing_asset_lock_signer(
                wallet_handle as Handle,
                &outpoint,
                &id,
                core_signer_handle as *mut MnemonicResolverHandle,
                consume_invitation_voucher != 0,
                &mut out_balance,
            )
        };
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

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

/// Top up an identity this wallet does NOT manage by building and
/// broadcasting a new Core asset lock from the wallet's balance. The lock's
/// row records `identityId` before the broadcast, so an interrupted top-up
/// resumes to that identity only (see `resumeTopUpToRecordedRecipient`).
/// Returns the identity's proved post-transition balance.
///
/// `out_outpoint` (a `byte[36]`) receives the built lock's outpoint
/// (`txid[32] ‖ vout_le[4]`, the wire order `TrackedAssetLock` carries) on
/// every outcome, success, error or panic, before any exception is thrown;
/// all zero means nothing was built and a retry is safe. It is checked before
/// the call, so a malformed one builds nothing.
///
/// Wraps `platform_wallet_top_up_external_identity_with_funding_signer`. Its
/// errors cross unchanged: `ErrorInvalidParameter` below the top-up minimum,
/// `ErrorIdentityNotFound` (7) for an identity Platform does not know, a
/// persistence error when the host does not keep identity recipients, and
/// `ErrorTransactionBroadcastUnconfirmed` (20) when the lock may have been
/// sent or its row may survive: resume that outpoint, never re-send.
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_topUpExternalIdentityFromCore(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    identity_id: JByteArray,
    amount_duffs: jlong,
    account_index: jint,
    core_signer_handle: jlong,
    out_outpoint: JByteArray,
) -> jlong {
    guard(&mut env, 0i64, |env| {
        // Sign errors only: a negative value would bit-cast to a huge
        // unsigned one. The amount's minimum is the wallet's to enforce.
        if amount_duffs < 0 {
            throw_sdk_exception(env, 1, "amountDuffs must be non-negative");
            return 0;
        }
        if account_index < 0 {
            throw_sdk_exception(env, 1, "accountIndex must be non-negative");
            return 0;
        }
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return 0;
        };
        // Checked before the call so the region write after it cannot fail:
        // a built lock's outpoint must reach Kotlin.
        if out_outpoint.is_null()
            || env
                .get_array_length(&out_outpoint)
                .map_or(true, |len| len != OUTPOINT_LEN as i32)
        {
            let _ = env.exception_clear();
            throw_sdk_exception(env, 1, "outOutpoint must be a non-null byte[36]");
            return 0;
        }
        let mut out_balance: u64 = 0;
        let mut built = OutPointFFI {
            txid: [0; 32],
            vout: 0,
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_top_up_external_identity_with_funding_signer(
                wallet_handle as Handle,
                &id,
                amount_duffs as u64,
                account_index as u32,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_balance,
                &mut built,
            )
        };
        // Before the error is raised: no JNI call may run with an exception
        // pending.
        let mut raw = [0i8; OUTPOINT_LEN];
        for (dst, src) in raw
            .iter_mut()
            .zip(built.txid.iter().chain(&built.vout.to_le_bytes()))
        {
            *dst = *src as i8;
        }
        if env.set_byte_array_region(&out_outpoint, 0, &raw).is_err() {
            // Unreachable after the length check; say a lock may exist so the
            // caller does not re-send.
            let _ = env.exception_clear();
            throw_sdk_exception(
                env,
                20,
                "the top-up's asset lock outpoint could not be returned; do not re-send",
            );
            return 0;
        }
        if take_pwffi_error(env, result) {
            return 0;
        }
        out_balance as i64
    })
}

/// Resume an interrupted identity top-up from a tracked Core outpoint to the
/// identity the lock pays: a bound lock pays the wallet's identity at its
/// registration index, an unbound one the identity its row recorded. The
/// caller names no identity. Returns an `IdentityTopUpNativeResult`
/// (`([BJ)V`) carrying the identity paid and its proved new balance.
///
/// Wraps `platform_wallet_resume_identity_top_up_to_recorded_recipient_signer`;
/// `ErrorAssetLockRecipientUnknown` (81) means the lock records no identity
/// (assign one with `assignTopUpRecipient` first).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_resumeTopUpToRecordedRecipient(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    outpoint_txid: JByteArray,
    outpoint_vout: jint,
    core_signer_handle: jlong,
) -> jobject {
    guard(&mut env, std::ptr::null_mut(), |env| {
        let Some(outpoint) = read_outpoint(env, &outpoint_txid, outpoint_vout) else {
            return std::ptr::null_mut();
        };
        let mut out_identity_id = [0u8; 32];
        let mut out_balance: u64 = 0;
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_resume_identity_top_up_to_recorded_recipient_signer(
                wallet_handle as Handle,
                &outpoint,
                core_signer_handle as *mut MnemonicResolverHandle,
                &mut out_identity_id,
                &mut out_balance,
            )
        };
        if take_pwffi_error(env, result) {
            return std::ptr::null_mut();
        }
        let built = env.byte_array_from_slice(&out_identity_id).and_then(|id| {
            env.new_object(
                "org/dashfoundation/dashsdk/ffi/IdentityTopUpNativeResult",
                "([BJ)V",
                &[
                    JValue::Object(id.as_ref()),
                    JValue::Long(out_balance as i64),
                ],
            )
        });
        match built {
            Ok(object) => object.into_raw(),
            Err(_) => {
                // The top-up already happened; only the result could not be
                // handed back. The balance refreshes through persistence.
                let _ = env.exception_clear();
                throw_sdk_exception(env, 99, "identity top-up result allocation failed");
                std::ptr::null_mut()
            }
        }
    })
}

/// Assign the identity an unassigned unbound identity top-up lock pays, as
/// the user chose it. Wraps `platform_wallet_assign_identity_top_up_recipient`:
/// `ErrorIdentityNotFound` (7) for an identity Platform does not know (nothing
/// assigned), `ErrorAssetLockRecipientMismatch` (61) when the lock already
/// records another identity (the same one again is a no-op).
#[no_mangle]
pub extern "system" fn Java_org_dashfoundation_dashsdk_ffi_CreditsNative_assignTopUpRecipient(
    mut env: JNIEnv,
    _class: JClass,
    wallet_handle: jlong,
    outpoint_txid: JByteArray,
    outpoint_vout: jint,
    identity_id: JByteArray,
) {
    guard(&mut env, (), |env| {
        let Some(outpoint) = read_outpoint(env, &outpoint_txid, outpoint_vout) else {
            return;
        };
        let Some(id) = read_id32(env, &identity_id, "identityId") else {
            return;
        };
        let result = unsafe {
            platform_wallet_ffi::platform_wallet_assign_identity_top_up_recipient(
                wallet_handle as Handle,
                &outpoint,
                &id,
            )
        };
        take_pwffi_error(env, result);
    })
}

/// Length of an outpoint as Kotlin carries it: `txid[32] ‖ vout_le[4]`.
const OUTPOINT_LEN: usize = 36;

/// Read a tracked-lock outpoint (32-byte wire-order txid + non-negative vout);
/// throws and returns `None` on a malformed one.
fn read_outpoint(env: &mut JNIEnv, txid: &JByteArray, vout: jint) -> Option<OutPointFFI> {
    let txid = read_id32(env, txid, "outpointTxid")?;
    if vout < 0 {
        throw_sdk_exception(env, 1, "outpointVout must be non-negative");
        return None;
    }
    Some(OutPointFFI {
        txid,
        vout: vout as u32,
    })
}

/// Size of one credit row: `u8 addressType, u8[20] hash, u64 credits`.
const CREDIT_ROW_LEN: usize = 1 + 20 + 8;

/// Parse a credit-rows BLOB into owned `(address_type, hash, credits)`
/// tuples. Layout (big-endian): `u32 rowCount` then per row
/// `u8 addressType (0 P2PKH / 1 P2SH), u8[20] hash, u64 credits`. Bytes
/// after the last row are ignored. Fails on a malformed blob or an
/// out-of-range (sign-bit-set) credit amount; `field` names the blob in the
/// error message, which never echoes a hash or amount.
///
/// The one parser for this wire format: [`decode_credit_rows`] wraps it for
/// the JNI exports here and in `identity.rs`, and `wallet_manager.rs` maps its
/// rows into `AddressBalanceEntryFFI` (wallet-signed transfer outputs) and
/// `ExplicitInputFFI` (explicit withdrawal inputs).
pub(crate) fn parse_credit_rows(
    bytes: &[u8],
    field: &str,
) -> Result<Vec<(u8, [u8; 20], u64)>, String> {
    let (count_bytes, body) = bytes
        .split_first_chunk::<4>()
        .ok_or_else(|| format!("{field} truncated (row count)"))?;
    let count = u32::from_be_bytes(*count_bytes) as usize;
    // Reject an impossible row count before allocating: rows are a fixed
    // 29 bytes, so a header claiming more than the remaining payload can
    // hold is malformed. Guards the raw-JNI boundary against a huge
    // `with_capacity` abort on a short/hostile blob.
    if count
        .checked_mul(CREDIT_ROW_LEN)
        .is_none_or(|need| body.len() < need)
    {
        return Err(format!("{field} claims {count} rows but body is too short"));
    }
    let (rows, _) = body.as_chunks::<CREDIT_ROW_LEN>();
    let mut out = Vec::with_capacity(count);
    for (i, row) in rows.iter().take(count).enumerate() {
        // The row is a `[u8; 29]`, so this pattern is irrefutable and binds
        // `hash` as `[u8; 20]`.
        let &[address_type, ref hash @ .., c0, c1, c2, c3, c4, c5, c6, c7] = row;
        let credits = u64::from_be_bytes([c0, c1, c2, c3, c4, c5, c6, c7]);
        // The Kotlin encoder writes this field from a signed long; a set
        // sign bit means a negative amount crossed the boundary — reject
        // rather than treat it as a huge unsigned credit value.
        if credits & (1 << 63) != 0 {
            return Err(format!("{field} row {i} credits amount out of range"));
        }
        out.push((address_type, *hash, credits));
    }
    Ok(out)
}

/// Decode a credit-rows BLOB (see [`parse_credit_rows`]) from a Java
/// `byte[]`. Throws + returns None on a null array, a malformed blob or an
/// out-of-range credit amount. [`field`] names the blob in error messages
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
    match parse_credit_rows(&bytes, field) {
        Ok(rows) => Some(rows),
        Err(msg) => {
            throw_sdk_exception(env, 1, &msg);
            None
        }
    }
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
