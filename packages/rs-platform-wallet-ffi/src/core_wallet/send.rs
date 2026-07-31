//! FFI binding for the single-account "build a signed payment" primitive.
//!
//! Like the step-by-step `core_wallet_tx_builder_*` builder, this funds from a
//! single caller-chosen account — but as a one-shot call that also signs, and
//! it names the account by BIP32 derivation path (so a DIP-9 CoinJoin or
//! DashPay-receiving account can be selected, not just BIP44/BIP32). It returns
//! the **signed serialized transaction bytes** plus the computed fee and change
//! amount. It does NOT broadcast and does NOT persist a debit — the caller
//! commits/broadcasts the returned bytes itself (dashj during the Android
//! transition; a later SDK-broadcast mode afterwards).
//!
//! Coin selection never unions funding accounts: `funding_path` names exactly
//! one, defaulting to the unmixed BIP44 account. See
//! `platform_wallet::wallet::funding_privacy` for the invariant and
//! `platform_wallet::wallet::core::send` for the semantics.

use crate::error::*;
use crate::handle::{Handle, CORE_WALLET_STORAGE};
use crate::runtime::runtime;
use crate::utils::parse_optional_derivation_path;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};
use dashcore::Address as DashAddress;
use platform_wallet::PlatformWalletError;
use rs_sdk_ffi::{MnemonicResolverCoreSigner, MnemonicResolverHandle};
use std::str::FromStr;

/// Smallest number of bytes one encoded output row can occupy: `u32 addr_len`
/// (4) + at least one address byte + `u64 amount` (8). Used to reject an
/// impossible `count` before any allocation.
const MIN_ENCODED_OUTPUT_LEN: usize = 4 + 1 + 8;

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
    // Checked cursor arithmetic throughout: `cursor + n` on a 32-bit target
    // (Android armeabi-v7a) can overflow and panic inside this `extern "C"`
    // frame, where the JNI guard cannot safely recover it.
    let read_u32 = |buf: &[u8], at: &mut usize| -> Result<u32, PlatformWalletError> {
        let end = at
            .checked_add(4)
            .filter(|e| *e <= buf.len())
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild("truncated recipients blob (u32)".to_string())
            })?;
        let v = u32::from_be_bytes([buf[*at], buf[*at + 1], buf[*at + 2], buf[*at + 3]]);
        *at = end;
        Ok(v)
    };
    let read_u64 = |buf: &[u8], at: &mut usize| -> Result<u64, PlatformWalletError> {
        let end = at
            .checked_add(8)
            .filter(|e| *e <= buf.len())
            .ok_or_else(|| {
                PlatformWalletError::TransactionBuild("truncated recipients blob (u64)".to_string())
            })?;
        let mut b = [0u8; 8];
        b.copy_from_slice(&buf[*at..end]);
        *at = end;
        Ok(u64::from_be_bytes(b))
    };

    // Bound `count` by what the blob could actually contain BEFORE reserving.
    // `count` is a caller-controlled `u32`: passing `u32::MAX` in a four-byte
    // blob would otherwise ask `Vec::with_capacity` for ~64 GiB and take the
    // process-aborting allocation-failure path instead of returning this
    // decode error.
    let count = read_u32(blob, &mut cursor)? as usize;
    let max_possible = blob.len().saturating_sub(cursor) / MIN_ENCODED_OUTPUT_LEN;
    if count > max_possible {
        return Err(err(format!(
            "recipients blob declares {count} outputs but holds at most {max_possible}"
        )));
    }
    let mut outputs = Vec::new();
    outputs
        .try_reserve_exact(count)
        .map_err(|e| err(format!("cannot allocate {count} recipient outputs: {e}")))?;
    for _ in 0..count {
        let addr_len = read_u32(blob, &mut cursor)? as usize;
        let end = cursor
            .checked_add(addr_len)
            .filter(|e| *e <= blob.len())
            .ok_or_else(|| err("truncated recipients blob (address)".to_string()))?;
        let addr_str = std::str::from_utf8(&blob[cursor..end])
            .map_err(|e| err(format!("recipient address is not valid UTF-8: {e}")))?;
        cursor = end;
        let amount = read_u64(blob, &mut cursor)?;

        let parsed = DashAddress::from_str(addr_str)
            .map_err(|e| err(format!("invalid recipient address {addr_str:?}: {e}")))?;
        let address = parsed.require_network(network).map_err(|e| {
            err(format!(
                "recipient address {addr_str:?} network mismatch: {e}"
            ))
        })?;
        outputs.push((address, amount));
    }
    Ok(outputs)
}

/// Build and sign a standard L1 payment from ONE of the wallet's signable funds
/// accounts and return the signed bytes + fee + change.
///
/// * `handle` — a core-wallet handle (`platform_wallet_get_core`).
/// * `outputs_blob`/`outputs_blob_len` — the recipients, encoded as documented
///   on [`decode_payment_outputs`].
/// * `fee_per_kb` — fee rate in duffs/kB, or `0` for the default (1000).
/// * `core_signer_handle` — the caller's `MnemonicResolverHandle`; ownership is
///   retained by the caller (this function does NOT destroy it).
/// * `funding_path_ptr`/`funding_path_len` — an optional UTF-8 BIP32
///   derivation-path string (e.g. `"m/44'/5'/0'"`) naming the SINGLE funds
///   account whose UTXOs fund the payment (dashpay/platform#4184). Pass
///   `null` / `0` for the default — the unmixed BIP44 account. Pass an explicit
///   account-level path (e.g. the DIP-9 CoinJoin account path) to spend
///   previously-mixed coins deliberately. There is no union across accounts and
///   no consent gate: exactly one funding source participates, and if it cannot
///   cover the payment the call fails with the typed insufficient-funds code.
/// * `out_tx_bytes`/`out_tx_len` — receive the consensus-serialized signed
///   transaction. Free with [`core_wallet_free_payment_bytes`].
/// * `out_fee` — receives the fee paid, in duffs.
/// * `out_change` — receives the change returned to the wallet, in duffs (0 if
///   the build produced no change output).
///
/// # Safety
/// All pointers must be valid; `outputs_blob` must be readable for
/// `outputs_blob_len` bytes; `funding_path_ptr`, when non-null, must point to
/// `funding_path_len` readable bytes for the duration of the call; the
/// out-pointers must be writable.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn core_wallet_build_signed_payment(
    handle: Handle,
    outputs_blob: *const u8,
    outputs_blob_len: usize,
    fee_per_kb: u64,
    core_signer_handle: *mut MnemonicResolverHandle,
    funding_path_ptr: *const u8,
    funding_path_len: usize,
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

    let funding_path = match parse_optional_derivation_path(funding_path_ptr, funding_path_len) {
        Ok(p) => p,
        Err(result) => return result,
    };

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
        let funding_path = funding_path.clone();
        let wallet_id = wallet.wallet_id();
        // SAFETY: `signer_addr` came from `core_signer_handle`, which the caller
        // pinned alive for this call; the `MnemonicResolverCoreSigner` lives
        // only on this stack frame and is dropped before returning.
        let signer = MnemonicResolverCoreSigner::new(
            signer_addr as *mut MnemonicResolverHandle,
            wallet_id,
            network,
        );
        runtime().block_on(wallet.build_signed_payment(outputs, fee, &signer, funding_path))
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

/// Decoder hardening tests.
///
/// `decode_payment_outputs` parses a caller-controlled blob inside an
/// `extern "C"` frame, where a panic or an allocation abort cannot be recovered
/// by the JNI guard. Every bound below was a blocking review finding on
/// dashpay/platform#4247 and shipped without coverage; these pin them so a
/// later cleanup cannot quietly reintroduce `Vec::with_capacity(count)` or
/// unchecked cursor arithmetic.
#[cfg(test)]
mod tests {
    use super::*;
    use dashcore::Network;

    /// Encode one recipient row in the wire layout `decode_payment_outputs`
    /// documents: `u32 addr_len`, the UTF-8 address, `u64 amount`.
    fn row(address: &str, amount: u64) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&(address.len() as u32).to_be_bytes());
        v.extend_from_slice(address.as_bytes());
        v.extend_from_slice(&amount.to_be_bytes());
        v
    }

    fn blob(rows: &[(&str, u64)]) -> Vec<u8> {
        let mut v = (rows.len() as u32).to_be_bytes().to_vec();
        for (a, amt) in rows {
            v.extend_from_slice(&row(a, *amt));
        }
        v
    }

    fn testnet_address(id: usize) -> String {
        DashAddress::dummy(Network::Testnet, id).to_string()
    }

    #[test]
    fn decodes_a_well_formed_blob() {
        let (a, b) = (testnet_address(1), testnet_address(2));
        let decoded = decode_payment_outputs(
            &blob(&[(a.as_str(), 1_000_000), (b.as_str(), 546)]),
            Network::Testnet,
        )
        .expect("a well-formed blob decodes");

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].0.to_string(), a);
        assert_eq!(decoded[0].1, 1_000_000);
        assert_eq!(decoded[1].0.to_string(), b);
        assert_eq!(decoded[1].1, 546);
    }

    #[test]
    fn zero_outputs_decode_to_an_empty_vec() {
        let decoded = decode_payment_outputs(&0u32.to_be_bytes(), Network::Testnet)
            .expect("an empty list is a decode success");
        assert!(
            decoded.is_empty(),
            "emptiness is rejected upstream, not here"
        );
    }

    /// THE allocation blocker: a four-byte blob declaring `u32::MAX` outputs
    /// must produce a decode error, never a ~64 GiB `Vec::with_capacity` that
    /// takes Rust's process-aborting allocation-failure path inside
    /// `extern "C"`.
    #[test]
    fn an_impossible_count_is_rejected_without_allocating() {
        for count in [u32::MAX, u32::MAX / 2, 1_000_000, 1] {
            let err = decode_payment_outputs(&count.to_be_bytes(), Network::Testnet)
                .expect_err("a header-only blob cannot hold any output");
            let PlatformWalletError::TransactionBuild(m) = err else {
                panic!("expected a decode error for count {count}");
            };
            assert!(
                m.contains("holds at most 0"),
                "count {count} must be bounded by the blob length, got {m:?}"
            );
        }
    }

    /// The bound is computed from the remaining bytes, so a count that merely
    /// overstates a non-empty blob is refused too.
    #[test]
    fn a_count_exceeding_the_rows_present_is_rejected() {
        let a = testnet_address(1);
        let mut b = blob(&[(a.as_str(), 1_000)]);
        b[0..4].copy_from_slice(&9u32.to_be_bytes());

        let err = decode_payment_outputs(&b, Network::Testnet).expect_err("9 rows are not present");
        assert!(
            matches!(err, PlatformWalletError::TransactionBuild(ref m) if m.contains("declares 9")),
            "got {err:?}"
        );
    }

    /// Checked cursor arithmetic: a truncated blob is a clean error at every
    /// field boundary, never an out-of-bounds slice or a `cursor + len`
    /// overflow panic (reachable on 32-bit Android targets).
    #[test]
    fn truncation_at_any_boundary_is_a_clean_error() {
        let a = testnet_address(1);
        let full = blob(&[(a.as_str(), 1_000)]);

        for cut in 1..full.len() {
            let err = decode_payment_outputs(&full[..cut], Network::Testnet)
                .expect_err("a truncated blob must not decode");
            assert!(
                matches!(err, PlatformWalletError::TransactionBuild(_)),
                "truncation at {cut} must be a decode error, got {err:?}"
            );
        }

        // The full blob still decodes, so the loop above proved truncation is
        // the cause rather than the fixture being malformed.
        assert!(decode_payment_outputs(&full, Network::Testnet).is_ok());
    }

    /// A declared address length far beyond the blob must not panic on
    /// `cursor + addr_len`.
    #[test]
    fn an_absurd_address_length_is_rejected() {
        let mut b = 1u32.to_be_bytes().to_vec();
        b.extend_from_slice(&u32::MAX.to_be_bytes());
        b.extend_from_slice(&[0u8; 16]);

        let err = decode_payment_outputs(&b, Network::Testnet)
            .expect_err("an address longer than the blob must not decode");
        assert!(
            matches!(err, PlatformWalletError::TransactionBuild(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn a_non_utf8_address_is_rejected() {
        let mut b = 1u32.to_be_bytes().to_vec();
        b.extend_from_slice(&4u32.to_be_bytes());
        b.extend_from_slice(&[0xff, 0xfe, 0xfd, 0xfc]);
        b.extend_from_slice(&1_000u64.to_be_bytes());

        let err = decode_payment_outputs(&b, Network::Testnet).expect_err("invalid UTF-8");
        assert!(
            matches!(err, PlatformWalletError::TransactionBuild(ref m) if m.contains("UTF-8")),
            "got {err:?}"
        );
    }

    #[test]
    fn an_unparseable_address_is_rejected() {
        let err = decode_payment_outputs(&blob(&[("not-an-address", 1_000)]), Network::Testnet)
            .expect_err("garbage is not an address");
        assert!(
            matches!(err, PlatformWalletError::TransactionBuild(ref m) if m.contains("invalid recipient address")),
            "got {err:?}"
        );
    }

    /// Network confusion is a funds-loss shape: a mainnet address accepted on a
    /// testnet wallet (or the reverse) sends real coins to an address the user
    /// did not intend.
    #[test]
    fn a_wrong_network_address_is_rejected() {
        let mainnet = DashAddress::dummy(Network::Mainnet, 1).to_string();
        let err = decode_payment_outputs(&blob(&[(mainnet.as_str(), 1_000)]), Network::Testnet)
            .expect_err("a mainnet address must not decode for a testnet wallet");
        assert!(
            matches!(err, PlatformWalletError::TransactionBuild(ref m) if m.contains("network mismatch")),
            "got {err:?}"
        );

        // …and it decodes fine against its own network, proving the rejection
        // is the network check rather than the address being malformed.
        assert!(
            decode_payment_outputs(&blob(&[(mainnet.as_str(), 1_000)]), Network::Mainnet).is_ok()
        );
    }

    /// `core_wallet_free_payment_bytes` tolerates the null/zero pair its own
    /// documented contract permits.
    #[test]
    fn freeing_null_payment_bytes_is_a_no_op() {
        unsafe {
            core_wallet_free_payment_bytes(std::ptr::null_mut(), 0);
            core_wallet_free_payment_bytes(std::ptr::null_mut(), 32);
        }
    }

    /// Round-trip through the real allocation path: what
    /// `core_wallet_build_signed_payment` hands out is what
    /// `core_wallet_free_payment_bytes` takes back.
    #[test]
    fn payment_bytes_round_trip_through_the_free_function() {
        let payload = vec![7u8; 128];
        let len = payload.len();
        let ptr = Box::into_raw(payload.into_boxed_slice()) as *mut u8;
        unsafe {
            assert_eq!(std::slice::from_raw_parts(ptr, len), [7u8; 128]);
            core_wallet_free_payment_bytes(ptr, len);
        }
    }
}
