//! FFI for the state-aware address funding fee quote
//! (`getAddressFundingFeeQuote`).

use dashcore::hashes::Hash;
use dpp::address_funds::PlatformAddress;

use crate::check_ptr;
use crate::core_wallet_types::OutPointFFI;
use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::{unwrap_option_or_return, unwrap_result_or_return};

/// A state-aware fee quote for a 0-input / 1-output address funding from a
/// fresh asset lock, as computed by a node.
///
/// Plain data — nothing to free. The quote is planning data from a single
/// node (the response carries no proof): sizing the funding lock stays
/// governed by `minimum_required_lock_credits` plus the application's own
/// margin policy.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AddressFundingFeeQuoteFFI {
    /// State-aware estimate of the fee the network would charge for the
    /// funding executed near the quoted state (includes the requested user
    /// fee increase).
    pub estimated_fee_credits: u64,
    /// The consensus admission floor for the asset lock.
    pub minimum_required_lock_credits: u64,
    /// The protocol version the node quoted with.
    pub protocol_version: u32,
    /// The committed block height the quote was computed on.
    pub state_height: u64,
}

/// Fetch a state-aware fee quote for funding one platform address from a
/// fresh asset lock (0 address inputs, 1 remainder output).
///
/// Asynchronous network call executed on the wallet's worker runtime; the
/// node computes the quote read-only against its committed state. There is
/// no offline fallback — a network failure returns an error, never a stale
/// constant.
///
/// - `recipient_address` / `recipient_address_len`: the serialized platform
///   address of the funding recipient (as produced by the wallet's address
///   APIs).
/// - `prepared_outpoint`: the exact asset-lock outpoint when the wallet has
///   already built and signed the lock transaction; NULL lets the node use a
///   deterministic placeholder with the same expected search depth. An
///   outpoint that is already spent on Platform is rejected by the node.
/// - `user_fee_increase`: the fee increase the quote should include; the
///   SDK's chain-lock retry loop can raise a funding by up to 14 units.
///
/// # Safety
/// - `recipient_address` must be valid for `recipient_address_len` bytes.
/// - `prepared_outpoint` must be NULL or a valid pointer.
/// - `out_quote` must be a valid, writable pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_address_wallet_quote_funding_fee(
    handle: Handle,
    recipient_address: *const u8,
    recipient_address_len: usize,
    prepared_outpoint: *const OutPointFFI,
    user_fee_increase: u16,
    out_quote: *mut AddressFundingFeeQuoteFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_quote);
    *out_quote = AddressFundingFeeQuoteFFI::default();
    check_ptr!(recipient_address);

    let address_bytes = std::slice::from_raw_parts(recipient_address, recipient_address_len);
    let recipient = match PlatformAddress::from_bytes(address_bytes) {
        Ok(recipient) => recipient,
        Err(e) => {
            return PlatformWalletFFIResult::err(
                PlatformWalletFFIResultCode::ErrorInvalidParameter,
                format!("recipient_address is not a serialized platform address: {e}"),
            );
        }
    };

    let outpoint: Option<[u8; 36]> = if prepared_outpoint.is_null() {
        None
    } else {
        let outpoint_ffi = *prepared_outpoint;
        // The same byte layout the chain uses for outpoint keys:
        // raw txid bytes followed by the vout in little endian.
        let out_point = dashcore::OutPoint {
            txid: dashcore::Txid::from_byte_array(outpoint_ffi.txid),
            vout: outpoint_ffi.vout,
        };
        Some(out_point.into())
    };

    let option = PLATFORM_ADDRESS_WALLET_STORAGE.with_item(handle, |wallet| {
        let wallet_clone = wallet.clone();
        block_on_worker(async move {
            wallet_clone
                .quote_funding_fee(recipient, outpoint, user_fee_increase)
                .await
        })
    });
    let result = unwrap_option_or_return!(option);
    let quote = unwrap_result_or_return!(result);

    *out_quote = AddressFundingFeeQuoteFFI {
        estimated_fee_credits: quote.estimated_fee_credits,
        minimum_required_lock_credits: quote.minimum_required_lock_credits,
        protocol_version: quote.protocol_version,
        state_height: quote.state_height,
    };
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Null-pointer and invalid-handle guards fail closed with the right
    /// codes and never touch the out parameter's success path.
    #[test]
    fn test_guards_fail_closed() {
        let mut out = AddressFundingFeeQuoteFFI {
            estimated_fee_credits: 0xDEAD,
            ..Default::default()
        };

        // Null out pointer.
        let mut result = unsafe {
            platform_address_wallet_quote_funding_fee(
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        unsafe { platform_wallet_ffi_result_free(&mut result) };

        // Null address pointer: out must be reset to the zero sentinel.
        let mut result = unsafe {
            platform_address_wallet_quote_funding_fee(
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                0,
                &mut out,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(out.estimated_fee_credits, 0, "sentinel must be written");
        unsafe { platform_wallet_ffi_result_free(&mut result) };

        // Malformed address bytes.
        let bad_address = [0xFFu8; 3];
        let mut result = unsafe {
            platform_address_wallet_quote_funding_fee(
                0,
                bad_address.as_ptr(),
                bad_address.len(),
                std::ptr::null(),
                0,
                &mut out,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        unsafe { platform_wallet_ffi_result_free(&mut result) };

        // Unknown handle with a well-formed address.
        let address = dpp::address_funds::PlatformAddress::P2pkh([7; 20]).to_bytes();
        let mut result = unsafe {
            platform_address_wallet_quote_funding_fee(
                0,
                address.as_ptr(),
                address.len(),
                std::ptr::null(),
                0,
                &mut out,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::NotFound,
            "an unknown handle maps to NotFound"
        );
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }
}
