//! Shared serialization of proof-verified token balances for the
//! transfer / burn FFI return path.
//!
//! Token state transitions return the post-action balances of the
//! affected identities as part of the broadcast proof result. Rather
//! than have the host issue a separate (racy) balance query after a
//! transfer / burn succeeds, the FFI marshals those already-verified
//! balances straight out as a JSON object.
//!
//! Shape: `{"<identityIdBase58>": "<balanceDecimalString>", ...}`.
//!
//! The balances are `u64` (`dpp::balances::credits::TokenAmount`).
//! JSON numbers only safely represent integers up to 2^53, so a large
//! balance would silently lose precision if emitted as a JSON number.
//! They are therefore encoded as decimal **strings**; the host parses
//! them back to `UInt64`.

use std::collections::BTreeMap;
use std::ffi::CString;
use std::os::raw::c_char;

use dpp::balances::credits::TokenAmount;
use dpp::platform_value::string_encoding::Encoding;
use dpp::prelude::Identifier;

use crate::error::{PlatformWalletFFIResult, PlatformWalletFFIResultCode};

/// Serialize a `BTreeMap<Identifier, TokenAmount>` into a heap-allocated
/// C string holding the `{"<base58Id>": "<decimal>"}` JSON object.
///
/// Returns the `*mut c_char` on success (caller owns it, frees via
/// `platform_wallet_string_free`), or a populated
/// `PlatformWalletFFIResult` error on a serialization / NUL failure.
pub(crate) fn token_balances_to_json_cstring(
    balances: &BTreeMap<Identifier, TokenAmount>,
) -> Result<*mut c_char, PlatformWalletFFIResult> {
    // Encode balances as strings to preserve full u64 range.
    let map: BTreeMap<String, String> = balances
        .iter()
        .map(|(id, amount)| (id.to_string(Encoding::Base58), amount.to_string()))
        .collect();

    json_map_to_cstring(&map)
}

/// Serialize a single `(owner, balance)` pair (the burn / mint
/// proof-result shape) into the same `{"<base58Id>": "<decimal>"}`
/// JSON C string.
pub(crate) fn single_token_balance_to_json_cstring(
    owner: &Identifier,
    balance: TokenAmount,
) -> Result<*mut c_char, PlatformWalletFFIResult> {
    let mut map: BTreeMap<String, String> = BTreeMap::new();
    map.insert(owner.to_string(Encoding::Base58), balance.to_string());
    json_map_to_cstring(&map)
}

/// Write an empty balances object (`{}`) — used for the
/// history-tracking / group-action proof variants that carry no
/// per-identity balance to persist.
pub(crate) fn write_empty_balances_json() -> Result<*mut c_char, PlatformWalletFFIResult> {
    let empty: BTreeMap<String, String> = BTreeMap::new();
    json_map_to_cstring(&empty)
}

fn json_map_to_cstring(
    map: &BTreeMap<String, String>,
) -> Result<*mut c_char, PlatformWalletFFIResult> {
    let json = serde_json::to_string(map).map_err(|e| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorSerialization,
            format!("failed to serialize token balances: {e}"),
        )
    })?;

    let c_str = CString::new(json).map_err(|e| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            format!("token balances JSON contained an interior NUL byte: {e}"),
        )
    })?;

    Ok(c_str.into_raw())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    /// Free a `*mut c_char` produced by the helpers under test, mirroring
    /// `platform_wallet_string_free`.
    unsafe fn free_cstring(ptr: *mut c_char) {
        if !ptr.is_null() {
            let _ = CString::from_raw(ptr);
        }
    }

    unsafe fn read_and_free(ptr: *mut c_char) -> String {
        let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
        free_cstring(ptr);
        s
    }

    #[test]
    fn empty_map_serializes_to_empty_object() {
        let ptr = write_empty_balances_json().expect("serialize");
        let json = unsafe { read_and_free(ptr) };
        assert_eq!(json, "{}");
    }

    #[test]
    fn balances_are_string_encoded_and_keyed_by_base58() {
        let id_a = Identifier::from([1u8; 32]);
        let id_b = Identifier::from([2u8; 32]);
        let mut balances: BTreeMap<Identifier, TokenAmount> = BTreeMap::new();
        balances.insert(id_a, 42);
        // A value above 2^53 to prove we don't lose precision via a
        // JSON number.
        balances.insert(id_b, 18_000_000_000_000_000_000);

        let ptr = token_balances_to_json_cstring(&balances).expect("serialize");
        let json = unsafe { read_and_free(ptr) };

        let parsed: BTreeMap<String, String> = serde_json::from_str(&json).expect("parse back");

        assert_eq!(
            parsed
                .get(&id_a.to_string(Encoding::Base58))
                .map(String::as_str),
            Some("42")
        );
        assert_eq!(
            parsed
                .get(&id_b.to_string(Encoding::Base58))
                .map(String::as_str),
            Some("18000000000000000000")
        );
    }

    #[test]
    fn single_balance_round_trips() {
        let owner = Identifier::from([7u8; 32]);
        let ptr = single_token_balance_to_json_cstring(&owner, 1_000).expect("serialize");
        let json = unsafe { read_and_free(ptr) };
        let parsed: BTreeMap<String, String> = serde_json::from_str(&json).expect("parse back");
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed
                .get(&owner.to_string(Encoding::Base58))
                .map(String::as_str),
            Some("1000")
        );
    }
}
