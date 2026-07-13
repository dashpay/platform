//! FFI binding for the union-funding "build a signed payment" primitive.
//!
//! Unlike the step-by-step `core_wallet_tx_builder_*` builder (which funds from
//! a single caller-chosen account), this is a one-shot call that funds a
//! standard L1 payment from the UNION of every signable funds account
//! (BIP44 + BIP32 + CoinJoin + DashPay receiving; watch-only DashPay external
//! accounts are excluded) and returns the **signed serialized transaction
//! bytes** plus the computed fee and change amount. It does NOT broadcast and
//! does NOT persist a debit — the caller commits/broadcasts the returned bytes
//! itself (dashj during the Android transition; a later SDK-broadcast mode
//! afterwards). See `platform_wallet::wallet::core::send` for the semantics.

use crate::error::*;
use crate::handle::{Handle, CORE_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::Address as DashAddress;
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use std::str::FromStr;

/// Decode the recipients blob the caller passes to
/// [`core_wallet_build_signed_payment`]. Layout (big-endian):
///
/// ```text
/// u32 count
/// count × ( u32 address_len, address_len bytes (UTF-8), u64 amount_duffs )
/// ```
///
/// Each address is parsed and checked against `network`; a malformed blob or a
/// wrong-network / unparseable address is a decode error.
fn decode_payment_outputs(
    blob: &[u8],
    network: dashcore::Network,
) -> Result<Vec<(DashAddress, u64)>, PlatformWalletError> {
    let err = |m: String| PlatformWalletError::TransactionBuild(m);
    let mut cursor = 0usize;
    let read_u32 = |buf: &[u8], at: &mut usize| -> Result<u32, PlatformWalletError> {
        let end = *at + 4;
        if end > buf.len() {
            return Err(PlatformWalletError::TransactionBuild(
                "truncated recipients blob (u32)".to_string(),
            ));
        }
        let v = u32::from_be_bytes([buf[*at], buf[*at + 1], buf[*at + 2], buf[*at + 3]]);
        *at = end;
        Ok(v)
    };
    let read_u64 = |buf: &[u8], at: &mut usize| -> Result<u64, PlatformWalletError> {
        let end = *at + 8;
        if end > buf.len() {
            return Err(PlatformWalletError::TransactionBuild(
                "truncated recipients blob (u64)".to_string(),
            ));
        }
        let mut b = [0u8; 8];
        b.copy_from_slice(&buf[*at..end]);
        *at = end;
        Ok(u64::from_be_bytes(b))
    };

    let count = read_u32(blob, &mut cursor)? as usize;
    let mut outputs = Vec::with_capacity(count);
    for _ in 0..count {
        let addr_len = read_u32(blob, &mut cursor)? as usize;
        let end = cursor + addr_len;
        if end > blob.len() {
            return Err(err("truncated recipients blob (address)".to_string()));
        }
        let addr_str = std::str::from_utf8(&blob[cursor..end])
            .map_err(|e| err(format!("recipient address is not valid UTF-8: {e}")))?;
        cursor = end;
        let amount = read_u64(blob, &mut cursor)?;

        let parsed = DashAddress::from_str(addr_str)
            .map_err(|e| err(format!("invalid recipient address {addr_str:?}: {e}")))?;
        let address = parsed
            .require_network(network)
            .map_err(|e| err(format!("recipient address {addr_str:?} network mismatch: {e}")))?;
        outputs.push((address, amount));
    }
    Ok(outputs)
}

/// Build and sign a standard L1 payment from the wallet's signable funds
/// accounts (union coin selection) and return the signed bytes + fee + change.
///
/// * `handle` — a core-wallet handle (`platform_wallet_get_core`).
/// * `outputs_blob`/`outputs_blob_len` — the recipients, encoded as documented
///   on [`decode_payment_outputs`].
/// * `fee_per_kb` — fee rate in duffs/kB, or `0` for the default (1000).
/// * `core_signer_handle` — the caller's `MnemonicResolverHandle`; ownership is
///   retained by the caller (this function does NOT destroy it).
/// * `out_tx_bytes`/`out_tx_len` — receive the consensus-serialized signed
///   transaction. Free with [`core_wallet_free_payment_bytes`].
/// * `out_fee` — receives the fee paid, in duffs.
/// * `out_change` — receives the change returned to the wallet, in duffs (0 if
///   the build produced no change output).
///
/// # Safety
/// All pointers must be valid; `outputs_blob` must be readable for
/// `outputs_blob_len` bytes; the out-pointers must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_build_signed_payment(
    handle: Handle,
    outputs_blob: *const u8,
    outputs_blob_len: usize,
    fee_per_kb: u64,
    core_signer_handle: *mut MnemonicResolverHandle,
    out_tx_bytes: *mut *mut u8,
    out_tx_len: *mut usize,
    out_fee: *mut u64,
    out_change: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(outputs_blob);
    check_ptr!(core_signer_handle);
    check_ptr!(out_tx_bytes);
    check_ptr!(out_tx_len);
    check_ptr!(out_fee);
    check_ptr!(out_change);

    let blob = std::slice::from_raw_parts(outputs_blob, outputs_blob_len);
    let signer_addr = core_signer_handle as usize;
    let fee = if fee_per_kb == 0 {
        None
    } else {
        Some(fee_per_kb)
    };

    let option = CORE_WALLET_STORAGE.with_item(handle, |wallet| {
        let network = wallet.network();
        let outputs = decode_payment_outputs(blob, network)?;
        let wallet_id = wallet.wallet_id();
        // SAFETY: `signer_addr` came from `core_signer_handle`, which the caller
        // pinned alive for this call; the `MnemonicResolverCoreSigner` lives
        // only on this stack frame and is dropped before returning.
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id,
            network,
        );
        runtime().block_on(wallet.build_signed_payment(outputs, fee, &signer))
    });

    let result = unwrap_option_or_return!(option);
    let payment = unwrap_result_or_return!(result);

    let serialized = dashcore::consensus::serialize(&payment.transaction);
    let len = serialized.len();
    *out_tx_bytes = Box::into_raw(serialized.into_boxed_slice()) as *mut u8;
    *out_tx_len = len;
    *out_fee = payment.fee;
    *out_change = payment.change_amount;

    PlatformWalletFFIResult::ok()
}

/// Free the signed-payment bytes returned by [`core_wallet_build_signed_payment`].
///
/// # Safety
/// `bytes`/`len` must be the exact pair written to `out_tx_bytes`/`out_tx_len`
/// by [`core_wallet_build_signed_payment`] (or null / 0).
#[no_mangle]
pub unsafe extern "C" fn core_wallet_free_payment_bytes(bytes: *mut u8, len: usize) {
    if !bytes.is_null() && len > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(bytes, len));
    }
}
