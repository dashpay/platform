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
    /// transition (identity-create, unshield, transfer, or withdrawal) was
    /// DEFINITIVELY not executed — either the relay/CheckTx rejected the
    /// broadcast, or Platform reported the transition's own execution error.
    /// Any note reservations were released and the caller is free to retry.
    /// For identity-create, the new identity does NOT exist and
    /// `out_identity_id` is left untouched (still zeroed).
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
    /// Maps `PlatformWalletError::ShieldedSpendUnconfirmed` (unshield /
    /// shielded transfer / shielded withdrawal). The spend transition was
    /// ACCEPTED by the relay but its execution result could not be confirmed
    /// (DAPI wait timeout, result-proof fetch/verify failure, …). The spend
    /// may have executed on chain, so the wallet intentionally KEEPS the
    /// notes reserved: the next nullifier sync promotes them to spent if the
    /// spend landed, and an app restart frees them if it never did. The host
    /// must NOT auto-retry — a retry would select different unreserved notes
    /// and could double-send if the original spend landed.
    ErrorShieldedSpendUnconfirmed = 18,
    /// Maps `PlatformWalletError::ShieldedNoRecordedAnchor`. The wallet could
    /// not build the spend against any Platform-recorded anchor yet: its local
    /// commitment tree is mid-block (an index-chunk sync routinely stops
    /// between block boundaries) while Platform records an anchor only at each
    /// block boundary. The transition was NOT broadcast and any note
    /// reservations were released, so this is RETRYABLE — the host should wait
    /// for the next shielded sync (which advances the tree onto a recorded
    /// boundary) and try again. Distinct from `ErrorShieldedSpendUnconfirmed`,
    /// where a spend WAS broadcast and must NOT be retried.
    ErrorShieldedNoRecordedAnchor = 19,
    /// Maps `PlatformWalletError::TransactionBroadcastUnconfirmed`. A core
    /// transaction broadcast (send-to-addresses, DashPay payment, or
    /// asset-lock funding) failed with an AMBIGUOUS outcome — the transaction
    /// may already be on the network (transport timeout after delivery,
    /// partial peer send, or an internal multi-node retry whose earlier
    /// attempt may have delivered). The wallet intentionally KEEPS the spent
    /// inputs' UTXO reservation, so an immediate retry fails at input
    /// selection instead of double-spending; the reservation TTL or a sync
    /// observing the transaction reconciles the outcome. The host must NOT
    /// auto-retry. Shielded sibling: [`Self::ErrorShieldedSpendUnconfirmed`].
    ErrorTransactionBroadcastUnconfirmed = 20,
    /// Maps `PlatformWalletError::AddressNonceMismatch`. Platform rejected an
    /// address-funds transition (shield, or identity top-up-from-addresses)
    /// because the submitted address nonce raced Platform's expected next
    /// value (a lagging DAPI replica stale read; consensus code 40603). Same
    /// definitively-failed / notes-released / safe-to-retry contract as
    /// [`Self::ErrorShieldedBroadcastFailed`] — the transition did NOT execute
    /// and any note reservations were released (a shield reserves none) — but
    /// as its OWN code so hosts can recognize this specific, self-healing
    /// failure and retry: the retry re-fetches the address nonce, resolving
    /// the mismatch without host intervention. The submitted and Platform-
    /// expected nonce values travel in the result `message` (the typed
    /// `Display`); they are not exposed as structured out-fields (that would
    /// require an ABI-breaking change to `PlatformWalletFFIResult`).
    ErrorAddressNonceMismatch = 21,
    /// Atomic Core selection found no or insufficient unreserved UTXOs.
    ErrorCoreInsufficientFunds = 22,
    /// Existing-lock recovery referenced an outpoint not owned/tracked by the wallet.
    ErrorAssetLockNotTracked = 23,
    /// Existing-lock recovery referenced a one-shot output already consumed.
    ErrorAssetLockAlreadyConsumed = 24,
    /// Existing-lock recovery attempted to use a lock for the wrong funding
    /// family or bound identity index.
    ErrorAssetLockFundingMismatch = 25,
    /// Maps `PlatformWalletError::TransactionBroadcast`. Core definitively
    /// rejected the transaction, so its UTXO reservation was released and the
    /// host may safely retry after addressing the rejection reason.
    ErrorTransactionBroadcastRejected = 26,
    /// `platform_wallet_manager_destroy` could not join every background
    /// coordinator thread cleanly, even after a retry: a loop panicked,
    /// exceeded its join budget, or stayed detached. The manager handle is
    /// still freed, but a worker may outlive `destroy` and fire a host
    /// callback through the about-to-be-freed context, so the host should
    /// treat this as a real teardown fault (log / surface) rather than a
    /// silent success. Swift mirror: `PlatformWalletResultCode.errorShutdownIncomplete`.
    ErrorShutdownIncomplete = 27,

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

    /// A `Success`-coded result that still carries an advisory `message`.
    ///
    /// Used by non-error outcomes that want to convey a human-readable
    /// explanation alongside an out-parameter — e.g. the withdrawal preflight's
    /// "can't fund" case, where `can_withdraw = false` is the authoritative
    /// signal and the message is the planner's typed reason. The `Success` code
    /// keeps it off the error path (`.check()` on language bindings only
    /// inspects the code); the message is freed like any other via
    /// [`platform_wallet_ffi_result_free`] / `Drop`.
    pub fn success_with_message(message: impl Into<String>) -> Self {
        let msg = message.into();
        let c_msg = CString::new(msg).unwrap_or_else(|_| CString::new("<invalid UTF-8>").unwrap());
        Self {
            code: PlatformWalletFFIResultCode::Success,
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
            PlatformWalletError::ShieldedSpendUnconfirmed { .. } => {
                PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed
            }
            PlatformWalletError::ShieldedNoRecordedAnchor(..) => {
                PlatformWalletFFIResultCode::ErrorShieldedNoRecordedAnchor
            }
            // The core-transaction sibling of the shielded pair above: the
            // do-not-retry signal must survive the boundary as a typed code
            // so hosts can distinguish it from a definitive rejection.
            PlatformWalletError::TransactionBroadcastUnconfirmed(..) => {
                PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed
            }
            PlatformWalletError::TransactionBroadcast(..) => {
                PlatformWalletFFIResultCode::ErrorTransactionBroadcastRejected
            }
            // A definitively-failed address-nonce race (reaches the blanket impl
            // via identity `top_up_from_addresses` → `?`/`.into()`). Exposing
            // provided/expected nonce as structured out-fields is INTENTIONALLY
            // out of scope: `PlatformWalletFFIResult` is by-value / ABI-frozen, so
            // the values travel in the message string and an FFI retry re-fetches
            // the nonce.
            PlatformWalletError::AddressNonceMismatch { .. } => {
                PlatformWalletFFIResultCode::ErrorAddressNonceMismatch
            }
            PlatformWalletError::CoreInsufficientFunds { .. } => {
                PlatformWalletFFIResultCode::ErrorCoreInsufficientFunds
            }
            PlatformWalletError::AssetLockNotTracked(..) => {
                PlatformWalletFFIResultCode::ErrorAssetLockNotTracked
            }
            PlatformWalletError::AssetLockAlreadyConsumed(..) => {
                PlatformWalletFFIResultCode::ErrorAssetLockAlreadyConsumed
            }
            PlatformWalletError::AssetLockFundingMismatch { .. } => {
                PlatformWalletFFIResultCode::ErrorAssetLockFundingMismatch
            }
            // A quiesce/drain barrier that did not complete within budget
            // (clear/reset paths). The host must fail closed: keep its
            // callback context alive and skip any paired persistence wipe.
            PlatformWalletError::ShutdownIncomplete(..) => {
                PlatformWalletFFIResultCode::ErrorShutdownIncomplete
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
    use key_wallet::account::StandardAccountType;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;

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

    #[test]
    fn atomic_core_insufficient_funds_maps_to_dedicated_code() {
        for account_type in [
            AccountTypePreference::BIP44,
            AccountTypePreference::BIP32,
            AccountTypePreference::CoinJoin,
        ] {
            let result: PlatformWalletFFIResult = PlatformWalletError::CoreInsufficientFunds {
                account_type,
                account_index: 0,
                available: Some(0),
                required: None,
            }
            .into();
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorCoreInsufficientFunds
            );
        }
    }

    #[test]
    fn asset_lock_recovery_failures_map_to_stable_codes() {
        use dashcore::OutPoint;
        use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;

        let out_point = OutPoint::null();
        let cases = [
            (
                PlatformWalletError::AssetLockNotTracked(out_point),
                PlatformWalletFFIResultCode::ErrorAssetLockNotTracked,
            ),
            (
                PlatformWalletError::AssetLockAlreadyConsumed(out_point),
                PlatformWalletFFIResultCode::ErrorAssetLockAlreadyConsumed,
            ),
            (
                PlatformWalletError::AssetLockFundingMismatch {
                    out_point,
                    expected_funding_type: AssetLockFundingType::IdentityRegistration,
                    expected_identity_index: 1,
                    actual_funding_type: AssetLockFundingType::IdentityTopUp,
                    actual_identity_index: 1,
                },
                PlatformWalletFFIResultCode::ErrorAssetLockFundingMismatch,
            ),
        ];
        for (error, expected) in cases {
            let result: PlatformWalletFFIResult = error.into();
            assert_eq!(result.code, expected);
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

        let spend_unconfirmed = PlatformWalletError::ShieldedSpendUnconfirmed {
            operation: "unshield",
            reason: "wait timed out".to_string(),
        };
        let rendered = spend_unconfirmed.to_string();
        let result: PlatformWalletFFIResult = spend_unconfirmed.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed,
            "ShieldedSpendUnconfirmed should map to ErrorShieldedSpendUnconfirmed (rendered: {rendered})"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(msg, rendered, "Display payload must survive verbatim");
    }

    /// The ambiguous core-broadcast outcome keeps its typed code across the
    /// boundary — flattening it to `ErrorUnknown` would erase the
    /// do-not-retry signal the variant exists to carry.
    #[test]
    fn transaction_broadcast_unconfirmed_maps_to_dedicated_code() {
        let unconfirmed = PlatformWalletError::TransactionBroadcastUnconfirmed(
            "gRPC deadline exceeded after delivery".to_string(),
        );
        let rendered = unconfirmed.to_string();
        let result: PlatformWalletFFIResult = unconfirmed.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastUnconfirmed,
            "TransactionBroadcastUnconfirmed should map to its dedicated code (rendered: {rendered})"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(msg, rendered, "Display payload must survive verbatim");
    }

    #[test]
    fn transaction_broadcast_rejected_maps_to_dedicated_code() {
        let rejected = PlatformWalletError::TransactionBroadcast(
            "mandatory-script-verify-flag-failed".to_string(),
        );
        let rendered = rejected.to_string();
        let result: PlatformWalletFFIResult = rejected.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorTransactionBroadcastRejected,
            "TransactionBroadcast should map to its dedicated rejection code"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(msg, rendered, "Display payload must survive verbatim");
    }

    /// `AddressNonceMismatch` maps to the dedicated `ErrorAddressNonceMismatch`
    /// FFI code through the blanket `From` impl (the path identity
    /// `top_up_from_addresses` takes via `?`/`.into()`) rather than flattening
    /// to `ErrorUnknown`. The typed Display rendering — carrying the submitted
    /// and expected nonce values — survives across the boundary as the message.
    #[test]
    fn address_nonce_mismatch_maps_to_dedicated_code() {
        let err = PlatformWalletError::AddressNonceMismatch {
            address: dpp::address_funds::PlatformAddress::P2pkh([7u8; 20]),
            provided_nonce: 1,
            expected_nonce: 2,
        };
        let rendered = err.to_string();
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAddressNonceMismatch,
            "AddressNonceMismatch should map to ErrorAddressNonceMismatch (rendered: {rendered})"
        );
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            msg, rendered,
            "Display payload must survive the FFI boundary verbatim"
        );
        // Pin the EXACT rendered substrings, not bare digits, so a
        // provided/expected transposition would fail the test.
        assert!(
            msg.contains("submitted nonce 1"),
            "submitted (provided) nonce must render exactly: {msg}"
        );
        assert!(
            msg.contains("Platform expected 2"),
            "expected nonce must render exactly: {msg}"
        );
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
