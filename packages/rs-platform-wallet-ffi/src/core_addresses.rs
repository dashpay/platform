//! Dash Core (on-chain) address validation.

use dash_network::ffi::FFINetwork;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::str::FromStr;

/// Validate that `address` is a well-formed Dash address on `network`.
/// Any null, non-UTF-8, malformed, or wrong-network input returns `false`.
///
/// # Safety
/// `address` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_address_validate(
    address: *const c_char,
    network: FFINetwork,
) -> bool {
    if address.is_null() {
        return false;
    }
    let Ok(address_str) = CStr::from_ptr(address).to_str() else {
        return false;
    };
    let Ok(parsed) = dashcore::Address::from_str(address_str) else {
        return false;
    };
    let net: dashcore::Network = network.into();
    parsed.require_network(net).is_ok()
}
