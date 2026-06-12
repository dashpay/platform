use platform_wallet::PlatformWalletError;
use std::ffi::CString;
use std::os::raw::c_char;

#[macro_export]
macro_rules! deref_ptr {
    ($ptr:expr) => {{
        if $ptr.is_null() {
            return $crate::error::PlatformWalletFFIResult::err(
                $crate::error::PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("{} ptr is null", stringify!($ptr)),
            );
        }
        unsafe { &*$ptr }
    }};
}

#[macro_export]
macro_rules! deref_ptr_mut {
    ($ptr:expr) => {{
        if $ptr.is_null() {
            return $crate::error::PlatformWalletFFIResult::err(
                $crate::error::PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("{} ptr is null", stringify!($ptr)),
            );
        }
        unsafe { &mut *$ptr }
    }};
}

#[macro_export]
macro_rules! check_ptr {
    ($ptr:expr) => {{
        if $ptr.is_null() {
            return $crate::error::PlatformWalletFFIResult::err(
                $crate::error::PlatformWalletFFIResultCode::ErrorNullPointer,
                format!("{} ptr is null", stringify!($ptr)),
            );
        }
    }};
}

#[macro_export]
macro_rules! unwrap_result_or_return {
    ($expr:expr) => {{
        match $expr {
            Ok(v) => v,
            Err(e) => return e.into(),
        }
    }};
}

#[macro_export]
macro_rules! unwrap_option_or_return {
    ($expr:expr) => {{
        let Some(v) = $expr else {
            return $expr.into();
        };
        v
    }};
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformWalletFFIResultCode {
    Success = 0,
    ErrorInvalidHandle = 1,
    ErrorInvalidParameter = 2,
    ErrorNullPointer = 3,
    ErrorSerialization = 4,
    ErrorDeserialization = 5,
    ErrorWalletOperation = 6,
    ErrorIdentityNotFound = 7,
    ErrorContactNotFound = 8,
    ErrorInvalidNetwork = 9,
    ErrorInvalidIdentifier = 10,
    ErrorMemoryAllocation = 11,
    ErrorUtf8Conversion = 12,
    /// Reserved slot for the arithmetic-overflow mapping arriving via #3549 —
    /// no in-tree producer today. Holding the slot here keeps language-mirror
    /// enums (Swift, Kotlin) numerically aligned with the eventual producer.
    ErrorArithmeticOverflow = 13,
    /// Auto-select had no candidate inputs. Covers all three "can't-select-inputs"
    /// wallet variants: `NoSpendableInputs` (account has nothing spendable),
    /// `OnlyOutputAddressesFunded` (every funded address is also a destination),
    /// and `OnlyDustInputs` (every funded address is below `min_input_amount`).
    /// The typed Display rendering survives via the result message so callers
    /// can distinguish the underlying cause. Caller must rotate to a fresh
    /// receive address, consolidate sub-min balances, or fall back to
    /// `InputSelection::Explicit`.
    ErrorNoSelectableInputs = 14,
    /// Maps `PlatformWalletError::WalletAlreadyExists`. Callers that create
    /// a wallet across multiple networks (or enable an additional network on
    /// an existing wallet) treat this as a benign "already present" no-op
    /// rather than a hard failure — the wallet's mnemonic/metadata were
    /// stored under its scoped id at original creation, so there is nothing
    /// to re-persist. The typed Display rendering still survives as the
    /// result message for logging/detail.
    ErrorWalletAlreadyExists = 15,
    /// Maps `PlatformWalletError::ShieldedBroadcastFailed`. The shielded
    /// identity-create transition was DEFINITIVELY not executed — either the
    /// relay/CheckTx rejected the broadcast, or Platform reported the
    /// transition's own execution error. The new identity does NOT exist, the
    /// spent notes' reservations were released, and the caller is free to
    /// retry. `out_identity_id` is left untouched (still zeroed).
    ErrorShieldedBroadcastFailed = 16,
    /// Maps `PlatformWalletError::ShieldedBroadcastUnconfirmed`. The broadcast
    /// was ACCEPTED by the relay but the SDK could not confirm its execution
    /// result (a transient result-proof fetch/verify failure, not a platform
    /// rejection), and a direct fetch of the derived id also came back empty.
    /// The identity may already exist on chain, so the caller must NOT treat
    /// it as unregistered or re-submit. UNLIKE every other error code,
    /// `out_identity_id` IS written (the 32-byte derived id) on this code so
    /// the caller can hold the slot and surface the pending identity.
    ErrorShieldedBroadcastUnconfirmed = 17,

    NotFound = 98, // Used exclusively for all the Option that are retuned as errors
    ErrorUnknown = 99,
}

/// Must be freed with ['platform_wallet_ffi_result_free']
#[repr(C)]
#[derive(Debug)]
pub struct PlatformWalletFFIResult {
    pub code: PlatformWalletFFIResultCode,
    pub message: *mut c_char,
}

impl Drop for PlatformWalletFFIResult {
    fn drop(&mut self) {
        if !self.message.is_null() {
            unsafe {
                let _ = CString::from_raw(self.message);
            }
            self.message = std::ptr::null_mut();
        }
    }
}

impl PlatformWalletFFIResult {
    pub const fn ok() -> Self {
        Self {
            code: PlatformWalletFFIResultCode::Success,
            message: std::ptr::null_mut(),
        }
    }

    pub fn err(code: PlatformWalletFFIResultCode, message: impl Into<String>) -> Self {
        let msg = message.into();
        let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("<invalid UTF-8>").unwrap());
        Self {
            code,
            message: c_msg.into_raw(),
        }
    }
}

/// Free the Rust-owned message held by an error result.
///
/// Idempotent — the message pointer is nulled after free so a
/// second call is a no-op. Safe to pass a `Success` result
/// (message is already NULL).
///
/// # Safety
/// `result` must point to a valid `PlatformWalletFFIResult`
/// produced by this crate. Mutates the struct through the pointer.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_ffi_result_free(result: *mut PlatformWalletFFIResult) {
    if result.is_null() {
        return;
    }
    let result = &mut *result;

    // Same logic we have in the Drop implementation, we can't rely on the Drop implementation
    // bcs the struct is always in the stack, and we cannot take ownership of it
    if !result.message.is_null() {
        let _ = CString::from_raw(result.message);
        result.message = std::ptr::null_mut();
    }
}

impl<T> From<Option<T>> for PlatformWalletFFIResult {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(_) => Self::ok(),
            None => Self::err(
                PlatformWalletFFIResultCode::NotFound,
                format!("requested {} not found", std::any::type_name::<T>()),
            ),
        }
    }
}

impl From<PlatformWalletError> for PlatformWalletFFIResult {
    fn from(error: PlatformWalletError) -> Self {
        // Map the typed wallet error variants explicitly so they
        // don't flatten to ErrorUnknown at the FFI boundary. The
        // catch-all ErrorUnknown remains for variants the FFI hasn't
        // assigned a dedicated code yet — those still carry the
        // typed Display rendering as the message.
        let code = match &error {
            PlatformWalletError::NoSpendableInputs { .. }
            | PlatformWalletError::OnlyOutputAddressesFunded { .. }
            | PlatformWalletError::OnlyDustInputs { .. } => {
                PlatformWalletFFIResultCode::ErrorNoSelectableInputs
            }
            PlatformWalletError::WalletAlreadyExists(..) => {
                PlatformWalletFFIResultCode::ErrorWalletAlreadyExists
            }
            // The two shielded broadcast/wait variants. Today nothing routes
            // them through this blanket impl — the dedicated match in
            // `platform_wallet_manager_shielded_identity_create_from_pool`
            // (`shielded_send.rs`) owns them so it can also write
            // `out_identity_id` on the unconfirmed code. But any *future* FFI
            // entry point that propagates these via `?` / `.into()` would
            // otherwise silently flatten them to `ErrorUnknown` and defeat the
            // slot-holding contract. A blanket conversion can't write
            // `out_identity_id` (it has no out-param), so the most it can do is
            // keep the typed code alive — which is what these arms guarantee.
            PlatformWalletError::ShieldedBroadcastFailed(..) => {
                PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed
            }
            PlatformWalletError::ShieldedBroadcastUnconfirmed { .. } => {
                PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed
            }
            _ => PlatformWalletFFIResultCode::ErrorUnknown,
        };
        PlatformWalletFFIResult::err(code, error.to_string())
    }
}

impl From<dashcore::consensus::encode::Error> for PlatformWalletFFIResult {
    fn from(error: dashcore::consensus::encode::Error) -> Self {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorDeserialization,
            error.to_string(),
        )
    }
}

impl From<std::ffi::NulError> for PlatformWalletFFIResult {
    fn from(e: std::ffi::NulError) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            format!("string contained an interior NUL byte: {e}"),
        )
    }
}

impl From<std::str::Utf8Error> for PlatformWalletFFIResult {
    fn from(e: std::str::Utf8Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            format!("invalid UTF-8: {e}"),
        )
    }
}

impl From<std::string::FromUtf8Error> for PlatformWalletFFIResult {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            format!("invalid UTF-8: {e}"),
        )
    }
}

impl From<bs58::decode::Error> for PlatformWalletFFIResult {
    fn from(e: bs58::decode::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorInvalidIdentifier,
            format!("base58 decode failed: {e}"),
        )
    }
}

impl From<hex::FromHexError> for PlatformWalletFFIResult {
    fn from(e: hex::FromHexError) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorInvalidIdentifier,
            format!("hex decode failed: {e}"),
        )
    }
}

impl From<serde_json::Error> for PlatformWalletFFIResult {
    fn from(e: serde_json::Error) -> Self {
        let code = if e.is_data() || e.is_syntax() {
            PlatformWalletFFIResultCode::ErrorDeserialization
        } else {
            PlatformWalletFFIResultCode::ErrorSerialization
        };
        Self::err(code, format!("JSON error: {e}"))
    }
}

impl From<bincode::error::EncodeError> for PlatformWalletFFIResult {
    fn from(e: bincode::error::EncodeError) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorSerialization,
            format!("bincode encode failed: {e}"),
        )
    }
}

impl From<bincode::error::DecodeError> for PlatformWalletFFIResult {
    fn from(e: bincode::error::DecodeError) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorDeserialization,
            format!("bincode decode failed: {e}"),
        )
    }
}

impl From<dpp::ProtocolError> for PlatformWalletFFIResult {
    fn from(e: dpp::ProtocolError) -> Self {
        let msg = e.to_string();
        let code = if msg.contains("identifier") {
            PlatformWalletFFIResultCode::ErrorInvalidIdentifier
        } else if msg.contains("deserialization") || msg.contains("decode") {
            PlatformWalletFFIResultCode::ErrorDeserialization
        } else if msg.contains("serialization") || msg.contains("encode") {
            PlatformWalletFFIResultCode::ErrorSerialization
        } else {
            PlatformWalletFFIResultCode::ErrorWalletOperation
        };
        Self::err(code, format!("DPP protocol error: {msg}"))
    }
}

impl From<&str> for PlatformWalletFFIResult {
    fn from(e: &str) -> Self {
        Self::err(PlatformWalletFFIResultCode::ErrorInvalidParameter, e)
    }
}

impl From<String> for PlatformWalletFFIResult {
    fn from(e: String) -> Self {
        Self::err(PlatformWalletFFIResultCode::ErrorInvalidParameter, e)
    }
}

impl From<key_wallet::bip32::Error> for PlatformWalletFFIResult {
    fn from(e: key_wallet::bip32::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("bip32 derivation failed: {e}"),
        )
    }
}

impl From<key_wallet::Error> for PlatformWalletFFIResult {
    fn from(e: key_wallet::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("key-wallet error: {e}"),
        )
    }
}

impl From<dashcore::address::Error> for PlatformWalletFFIResult {
    fn from(e: dashcore::address::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("address parse failed: {e}"),
        )
    }
}

impl From<dashcore::key::Error> for PlatformWalletFFIResult {
    fn from(e: dashcore::key::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            format!("dashcore key error: {e}"),
        )
    }
}

impl From<dpp::platform_value::Error> for PlatformWalletFFIResult {
    fn from(e: dpp::platform_value::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorSerialization,
            format!("platform_value error: {e}"),
        )
    }
}

impl From<platform_wallet::changeset::PersistenceError> for PlatformWalletFFIResult {
    fn from(e: platform_wallet::changeset::PersistenceError) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorWalletOperation,
            format!("persistence error: {e}"),
        )
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for PlatformWalletFFIResult {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorUnknown,
            format!("unclassified error: {e}"),
        )
    }
}

impl From<anyhow::Error> for PlatformWalletFFIResult {
    fn from(e: anyhow::Error) -> Self {
        Self::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            e.to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_has_null_message() {
        let r = PlatformWalletFFIResult::ok();
        assert_eq!(r.code, PlatformWalletFFIResultCode::Success);
        assert!(r.message.is_null());
    }

    #[test]
    fn err_carries_message() {
        let mut r =
            PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::ErrorDeserialization, "boom");
        assert_ne!(r.code, PlatformWalletFFIResultCode::Success);
        assert!(!r.message.is_null());
        unsafe { platform_wallet_ffi_result_free(&mut r) };
        assert!(r.message.is_null());
    }

    #[test]
    fn free_is_idempotent() {
        let mut r = PlatformWalletFFIResult::err(PlatformWalletFFIResultCode::ErrorUnknown, "x");
        unsafe {
            platform_wallet_ffi_result_free(&mut r);
            platform_wallet_ffi_result_free(&mut r);
        }
        assert!(r.message.is_null());
    }

    #[test]
    fn nul_in_message_is_replaced() {
        let r = PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorUnknown,
            "before\0after",
        );
        assert!(!r.message.is_null());
    }

    /// The three "can't-select-inputs" wallet variants (`NoSpendableInputs`,
    /// `OnlyOutputAddressesFunded`, `OnlyDustInputs`) all map to the dedicated
    /// `ErrorNoSelectableInputs` FFI code rather than flattening to
    /// `ErrorUnknown`, and the typed Display rendering survives across the
    /// boundary so callers can distinguish the underlying cause from the
    /// message string.
    #[test]
    fn no_selectable_inputs_maps_to_dedicated_code() {
        use dpp::address_funds::PlatformAddress;
        use key_wallet::account::StandardAccountType;

        let cases: Vec<PlatformWalletError> = vec![
            PlatformWalletError::NoSpendableInputs {
                account_type: StandardAccountType::BIP44Account,
                account_index: 0,
                context: "wallet empty in test".to_string(),
            },
            PlatformWalletError::OnlyOutputAddressesFunded {
                funded_outputs: Vec::<PlatformAddress>::new(),
                sub_min_count: 0,
                sub_min_aggregate: 0,
                min_input_amount: 1_000,
            },
            PlatformWalletError::OnlyDustInputs {
                sub_min_count: 3,
                sub_min_aggregate: 500,
                min_input_amount: 1_000,
            },
        ];

        for err in cases {
            let rendered = err.to_string();
            let result: PlatformWalletFFIResult = err.into();
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorNoSelectableInputs,
                "variant should map to ErrorNoSelectableInputs (rendered: {rendered})"
            );
            assert!(!result.message.is_null());
            let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
                .to_string_lossy()
                .into_owned();
            assert_eq!(
                msg, rendered,
                "Display payload must survive the FFI boundary verbatim"
            );
        }
    }

    /// `WalletAlreadyExists` maps to the dedicated
    /// `ErrorWalletAlreadyExists` FFI code rather than flattening to
    /// `ErrorUnknown`, so multi-network wallet create/enable callers can
    /// branch on the typed code instead of substring-matching the Display
    /// text. The typed Display rendering still survives as the message.
    #[test]
    fn wallet_already_exists_maps_to_dedicated_code() {
        let err = PlatformWalletError::WalletAlreadyExists("wallet 0xdeadbeef".to_string());
        let rendered = err.to_string();
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletAlreadyExists,
            "WalletAlreadyExists should map to ErrorWalletAlreadyExists (rendered: {rendered})"
        );
        assert!(!result.message.is_null());
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            msg, rendered,
            "Display payload must survive the FFI boundary verbatim"
        );
    }

    /// The two shielded broadcast/wait variants map to their dedicated FFI
    /// codes through the blanket `From` impl rather than flattening to
    /// `ErrorUnknown`. The dedicated `shielded_send.rs` match owns the live
    /// path (it also writes `out_identity_id` on the unconfirmed code), but
    /// any future entry point propagating these via `?` / `.into()` must keep
    /// the typed code — these arms guarantee that. The typed Display rendering
    /// still survives as the message.
    #[test]
    fn shielded_broadcast_variants_map_to_dedicated_codes() {
        let failed = PlatformWalletError::ShieldedBroadcastFailed("relay rejected".to_string());
        let rendered = failed.to_string();
        let result: PlatformWalletFFIResult = failed.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastFailed,
            "ShieldedBroadcastFailed should map to ErrorShieldedBroadcastFailed (rendered: {rendered})"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(msg, rendered, "Display payload must survive verbatim");

        let unconfirmed = PlatformWalletError::ShieldedBroadcastUnconfirmed {
            identity_id: dpp::prelude::Identifier::from([7u8; 32]),
            reason: "result proof fetch failed".to_string(),
        };
        let rendered = unconfirmed.to_string();
        let result: PlatformWalletFFIResult = unconfirmed.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBroadcastUnconfirmed,
            "ShieldedBroadcastUnconfirmed should map to ErrorShieldedBroadcastUnconfirmed (rendered: {rendered})"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(msg, rendered, "Display payload must survive verbatim");
    }

    /// Other wallet-error variants without a dedicated FFI arm still
    /// fall through to `ErrorUnknown` while carrying the typed
    /// Display rendering as the message. Pin this so the catch-all
    /// stays the only `ErrorUnknown` source.
    #[test]
    fn unmapped_variants_fall_through_to_unknown() {
        let err = PlatformWalletError::AddressOperation("explicit fallthrough".to_string());
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorUnknown);
    }
}
