use dpp::platform_value::string_encoding::Encoding;
use platform_wallet::PlatformWalletError;
use std::ffi::CString;
use std::os::raw::c_char;

/// Compile-time drift guard for the signer key-unavailable machine prefix.
///
/// `platform-wallet` cannot depend on this FFI crate, so it mirrors the
/// reserved prefix locally (`platform_wallet::error::SIGNER_KEY_UNAVAILABLE_PREFIX`)
/// to promote a structured signer failure to [`PlatformWalletError::Sdk`]
/// *before* an operation wrapper stringifies it. This crate — which sees both
/// definitions — pins the mirror byte-identical to the canonical
/// [`rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX`], so any drift is a
/// build failure rather than a silent code-31 regression
/// (dashpay/platform#4183 review).
const fn const_str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
const _: () = assert!(
    const_str_eq(
        platform_wallet::error::SIGNER_KEY_UNAVAILABLE_PREFIX,
        rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX,
    ),
    "platform-wallet's mirrored SIGNER_KEY_UNAVAILABLE_PREFIX drifted from \
     rs-sdk-ffi's canonical DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX"
);

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
    /// A quiesce/drain barrier did not complete within its budget: an
    /// in-flight sync pass was still running when a Clear / reset /
    /// sync-stop needed it provably drained. The operation failed closed
    /// (no state was wiped) and the host should retry once sync is idle.
    /// NOT returned by `platform_wallet_manager_destroy` — with owned
    /// callback contexts (`release_fn`) a straggling worker keeps its
    /// context alive and releases it on exit, so destroy logs a non-clean
    /// join instead of erroring. Swift mirror:
    /// `PlatformWalletResultCode.errorShutdownIncomplete`.
    ErrorShutdownIncomplete = 27,
    /// Asset-lock coin selection came up short over the *permitted* funding
    /// set (dashpay/platform#4073). Carries the structured
    /// `available`/`required` duff amounts in the message string — the
    /// by-value `PlatformWalletFFIResult` is ABI-frozen (code + message only),
    /// so the figures ride the typed `Display` rendering or not at all.
    ///
    /// Distinct from [`Self::ErrorCoreInsufficientFunds`] (22), which is the
    /// atomic Core-send selector rather than the asset-lock builder. Asset-lock
    /// funding never unions across accounts, so this names a shortfall on the
    /// ONE account the caller selected; a host offering another source must
    /// name it explicitly.
    ///
    /// Reached by the CoinJoin → shielded migration when the mixed account
    /// cannot cover the lock, which is why the Android binding needs it typed.
    ErrorAssetLockInsufficientFunds = 29,
    /// A state transition could not be signed because the signer has no
    /// usable private key for the requested public key — the stored blob is
    /// missing, stranded, or written under a different Keystore/Keychain
    /// alias — rather than the operation itself failing. Restored from the
    /// typed signer completion code
    /// ([`rs_sdk_ffi::DashSDKSignerErrorCode::SigningKeyUnavailable`]) via
    /// the stable machine prefix
    /// [`rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX`] riding the
    /// `ProtocolError::Generic` segment (dashpay/platform#4060 finding 7).
    /// Hosts route this to key repair instead of treating it as an opaque
    /// wallet-operation failure. Not retryable as-is — the key must be
    /// (re-)derived first.
    ///
    /// Also produced by
    /// [`CoreWallet::sign_message`](platform_wallet::CoreWallet::sign_message),
    /// both without a signer round-trip (a message-signing address that
    /// belongs to no signable funds account of this wallet, so no key can
    /// exist for it) and from the signer itself (the resolver reported no
    /// mnemonic stored — `MnemonicResolverCoreSigner::NotFound`, whose
    /// rendering stamps the same machine prefix at position 0 and which
    /// `sign_message` promotes to the typed
    /// `MessageSigningKeyUnavailable` before adding context). Hosts route
    /// all of these to key repair / address correction rather than to an
    /// opaque wallet-operation failure.
    ErrorSigningKeyUnavailable = 31,

    // Codes 27-33 are claimed outside this PR and MUST NOT be reused here.
    // The deferred-token trio below therefore occupies the contiguous block
    // 34-36. Current owners (see ERROR_CODE_REGISTRY.md, dashpay/platform#4261):
    //
    //   27  ErrorShutdownIncomplete         MERGED on v4.2-dev (dashpay/platform#4268)
    //   28  (free — vacated by this PR)
    //   29  ErrorAssetLockInsufficientFunds ALLOCATED above. Claimed by
    //       dashpay/platform#4184, which was closed unmerged along with its
    //       successor #4316; this PR salvages the code at its reserved number
    //       so the ABI matches what every host mirror already documents.
    //   30  (free — vacated by this PR)
    //   31  ErrorSigningKeyUnavailable      dashpay/platform#4183, #4259
    //   32  ErrorTransactionBuild           dashpay/platform#4247, #4256
    //   33  ErrorTransactionSigning         dashpay/platform#4256
    //
    // This trio previously sat at 26-28, then 27/28/30. It moved to 34-36 after
    // #4268 merged `ErrorShutdownIncomplete = 27` into the v4.2-dev ABI; the
    // contiguous block above every current claim ends the renumbering churn.
    //
    // Claimed after the trio, same rule (fresh block above every claim):
    //
    //   37  ErrorDocumentNotForSale         DPNS username marketplace
    //   38  ErrorDocumentPriceChanged       DPNS username marketplace
    //   39  ErrorInsufficientIdentityCredits DPNS username marketplace
    //   40  ErrorContestedNameNotTradable   DPNS username marketplace
    //
    // 38/39/40 carry a STABLE JSON detail object in the result `message`
    // instead of the typed `Display` rendering — see each variant's doc for
    // the exact object. `PlatformWalletFFIResult` is ABI-frozen (code +
    // message only), so structured values ride the message or not at all.
    /// Maps `SignedPaymentError::StaleReservationToken` from the deferred
    /// build → broadcast/release core-send lifecycle (`core_wallet_signed_payment_*`):
    /// the token has outlived the registry's `RESERVATION_MAX_AGE_BLOCKS` bound
    /// and its funding reservation may already have been swept and re-selected by
    /// key-wallet's TTL, so acting on it could touch a newer, unrelated
    /// reservation. The operation did NOT touch the network. NOT retryable in
    /// place — the host must rebuild the payment.
    ///
    /// Sibling codes split out the other two deferred-token failures that this
    /// code used to conflate: [`Self::ErrorReservationTokenConsumed`] (35,
    /// unknown / already broadcast / already released) and
    /// [`Self::ErrorReservationWalletMismatch`] (36, minted against a different
    /// wallet generation). All three are non-retryable-in-place and none touched
    /// the network; they are distinct codes so a host can message each precisely.
    ErrorStaleReservationToken = 34,

    /// Maps `SignedPaymentError::StaleToken`. The deferred reservation token is
    /// unknown, already broadcast, or already released — the guard that turns a
    /// double-broadcast (or a broadcast after release) into a typed error
    /// instead of a second send. Did NOT touch the network; NOT retryable
    /// (rebuild the payment). Release is idempotent and never surfaces this.
    ErrorReservationTokenConsumed = 35,

    /// Maps `SignedPaymentError::WalletMismatch`. The deferred reservation token
    /// was minted against a different wallet *generation* than the one it is
    /// being broadcast through (e.g. a wallet re-created under the same id); its
    /// reservation lives in that other generation's `ReservationSet`. Did NOT
    /// touch the network and did NOT consume the rightful owner's token; NOT
    /// retryable through this handle (rebuild the payment).
    ///
    ErrorReservationWalletMismatch = 36,

    // -----------------------------------------------------------------
    // DPNS username-marketplace trade rejections (37-40).
    //
    // A fresh contiguous block ABOVE every current claim, for the same
    // reason the 34-36 trio moved there: 28 and 30 are nominally free but
    // reusing a vacated slot re-opens the renumbering churn the registry
    // note above exists to end.
    // -----------------------------------------------------------------
    /// Maps `PlatformWalletError::DocumentNotForSale`. The document
    /// carries no `$price`, so it cannot be purchased (and a DPNS delist
    /// has nothing to clear). Raised by the wallet's pre-flight read and
    /// by the downcast of the consensus `DocumentNotForSaleError` (DPP
    /// code 40108). The transition did NOT execute.
    ///
    /// Message: the typed `Display` rendering (no structured detail —
    /// the only value is the document id, which the caller already has).
    ErrorDocumentNotForSale = 37,

    /// Maps `PlatformWalletError::DocumentPriceChanged`. The listing no
    /// longer matches the price the user confirmed — either the wallet's
    /// pre-flight read disagreed, or consensus rejected the broadcast
    /// with `DocumentIncorrectPurchasePriceError` (DPP code 40109)
    /// because the listing changed between read and broadcast. The
    /// purchase did NOT execute in either case; re-confirm at the new
    /// price and retry.
    ///
    /// Message: a STABLE JSON detail object so hosts recover the typed
    /// values without parsing prose —
    /// `{"documentId":"<base58>","expected":<u64>,"actual":<u64>}`
    /// (credits). Swift mirror: `PlatformWalletError.priceChanged`.
    ErrorDocumentPriceChanged = 38,

    /// Maps `PlatformWalletError::InsufficientIdentityCredits`. The
    /// identity's credit balance cannot cover the operation — the
    /// wallet's purchase pre-flight (price + fee reserve against the
    /// local balance snapshot) or the downcast of the consensus
    /// `IdentityInsufficientBalanceError`. Nothing executed; top the
    /// identity up and retry.
    ///
    /// Message: a STABLE JSON detail object —
    /// `{"identityId":"<base58>","required":<u64>,"available":<u64>}`
    /// (credits). Swift mirror:
    /// `PlatformWalletError.insufficientIdentityCredits`.
    ErrorInsufficientIdentityCredits = 39,

    /// Maps `PlatformWalletError::ContestedNameNotTradable`. The DPNS
    /// name is inside an active contested-name vote, so its domain
    /// document is not in the documents tree and no trade transition can
    /// reference it. Without this typed code the network's bare
    /// `DocumentNotFoundError` (40101) would read as "no such name".
    /// Retry after the contest resolves.
    ///
    /// Message: a STABLE JSON detail object —
    /// `{"label":"<string>","endsAtMs":<u64>}`, where `endsAtMs == 0`
    /// means the vote's end time was unavailable. Swift mirror:
    /// `PlatformWalletError.contestedNameNotTradable`.
    ErrorContestedNameNotTradable = 40,

    /// The named thing does not exist.
    ///
    /// Originally (and still mostly) the code for every `Option` returned as an
    /// error — a handle that resolves to nothing, a lookup that came back empty.
    ///
    /// The deferred build → broadcast/release lifecycle also reports its
    /// wallet-was-REMOVED case here rather than minting a fourth
    /// deferred-token code, because it *is* that same "does not exist" case:
    /// `core_wallet_signed_payment_broadcast` maps
    /// `SignedPaymentError::WalletRemoved` (the token's wallet is no longer
    /// registered in the manager), and `core_wallet_signed_payment_finalize`
    /// refuses to register a payment whose wallet was removed while it was being
    /// signed — reconciling that build's reservation before returning. Neither
    /// touched the network. Contrast [`Self::ErrorReservationWalletMismatch`]
    /// (36), where a DIFFERENT live generation answers to the same wallet id;
    /// here there is no live generation at all, so there is nothing to retry
    /// against (`dashpay/platform#4185`).
    NotFound = 98,
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

/// The value-carrying DPNS-marketplace rejections, rendered as
/// `(code, JSON detail)` instead of `(code, Display)`.
///
/// `PlatformWalletFFIResult` is ABI-frozen at `{ code, message }`, so a
/// host that needs the *values* — not prose naming them — can only get
/// them through the message. These three therefore put a stable JSON
/// object there; the exact shape is documented on each
/// [`PlatformWalletFFIResultCode`] variant and parsed back by the Swift
/// mirror. Returns `None` for every other error, leaving the `Display`
/// rendering in charge.
///
/// `DocumentNotForSale` (37) is deliberately absent: its only value is
/// the document id the caller supplied, so its `Display` is enough.
fn trade_error_json_detail(
    error: &PlatformWalletError,
) -> Option<(PlatformWalletFFIResultCode, String)> {
    match error {
        PlatformWalletError::DocumentPriceChanged {
            document_id,
            expected,
            actual,
        } => Some((
            PlatformWalletFFIResultCode::ErrorDocumentPriceChanged,
            serde_json::json!({
                "documentId": document_id.to_string(Encoding::Base58),
                "expected": expected,
                "actual": actual,
            })
            .to_string(),
        )),
        PlatformWalletError::InsufficientIdentityCredits {
            identity_id,
            required,
            available,
        } => Some((
            PlatformWalletFFIResultCode::ErrorInsufficientIdentityCredits,
            serde_json::json!({
                "identityId": identity_id.to_string(Encoding::Base58),
                "required": required,
                "available": available,
            })
            .to_string(),
        )),
        PlatformWalletError::ContestedNameNotTradable { label, ends_at_ms } => Some((
            PlatformWalletFFIResultCode::ErrorContestedNameNotTradable,
            serde_json::json!({
                "label": label,
                "endsAtMs": ends_at_ms,
            })
            .to_string(),
        )),
        _ => None,
    }
}

impl From<PlatformWalletError> for PlatformWalletFFIResult {
    fn from(error: PlatformWalletError) -> Self {
        // The three value-carrying marketplace rejections replace the
        // Display rendering with a stable JSON detail object; everything
        // else keeps Display as the message.
        if let Some((code, detail)) = trade_error_json_detail(&error) {
            return PlatformWalletFFIResult::err(code, detail);
        }
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
            // Both shapes are "the wallet cannot cover this payment"; hosts
            // classify and retry them identically, so the pooled variant rides
            // the same code rather than forcing every host to learn a second
            // insufficient-funds value.
            PlatformWalletError::CoreInsufficientFunds { .. }
            | PlatformWalletError::CorePooledInsufficientFunds { .. } => {
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
            // The asset-lock coin-selection shortfall (dashpay/platform#4073).
            // Without this arm it flattens to `ErrorUnknown` (99), hiding a
            // typed shortfall behind the catch-all and forcing hosts to
            // string-match the Display text. The structured
            // `available`/`required` duff amounts still travel in the message
            // (there are no out-params for them), but the code now lets a host
            // branch on the shortfall without parsing text.
            PlatformWalletError::AssetLockInsufficientFunds { .. } => {
                PlatformWalletFFIResultCode::ErrorAssetLockInsufficientFunds
            }
            // A quiesce/drain barrier that did not complete within budget
            // (clear/reset paths). The host must fail closed: keep its
            // callback context alive and skip any paired persistence wipe.
            PlatformWalletError::ShutdownIncomplete(..) => {
                PlatformWalletFFIResultCode::ErrorShutdownIncomplete
            }
            // A signer failure can also reach this blanket impl wrapped as
            // `PlatformWalletError::Sdk(dash_sdk::Error::Protocol(..))` (any
            // wallet operation that propagates the SDK error via `?`). The
            // typed discriminator rides the stable machine prefix at the
            // START of the `ProtocolError::Generic` payload — restore it here
            // too, but ONLY on the catch-all: the dedicated retry-semantics
            // codes above are never overridden (dashpay/platform#4060 finding
            // 7). Inspect the payload STRUCTURALLY and require the marker at
            // position 0 rather than sniffing it as a substring of the fully
            // rendered error: a foreign signer can emit a generic (code-0)
            // failure whose human-readable text merely mentions the token, or
            // it can hide inside another variant's Display, and neither must
            // be routed into key repair (dashpay/platform#4183 review).
            PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(s)))
                if s.starts_with(rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX) =>
            {
                PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
            }
            // An unparseable / wrong-network / non-P2PKH message-signing
            // address: a caller-input error, so it is routed to the
            // already-mirrored ErrorInvalidParameter rather than spending a new
            // numeric code (and churning the Swift/Kotlin mirror enums) on a
            // case hosts handle by correcting the input. The typed Display
            // names which of the three it was.
            PlatformWalletError::MessageSigningAddressInvalid { .. } => {
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            }
            // Message bytes that are not valid UTF-8: the same kind of
            // caller-input error as the address arm above, so it gets the same
            // code. It previously fell through to ErrorUnknown, which told a
            // host "internal failure" about a malformed argument it could fix.
            PlatformWalletError::MessageSigningMessageInvalid { .. } => {
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            }
            // A caller-argument rejection raised below the FFI boundary — the
            // same class the boundary itself rejects with this code, so both
            // sides agree instead of one reporting a not-found or an internal
            // failure for a bad argument.
            PlatformWalletError::InvalidParameter(..) => {
                PlatformWalletFFIResultCode::ErrorInvalidParameter
            }
            // A second producer of code 31 (the arm above is the first),
            // reached without any marker inspection at this layer: message
            // signing found no signable account for the address, or the signer
            // reported its key missing (the reserved marker at position 0 of
            // its rendering, which `sign_message` promotes to this typed
            // variant BEFORE composing any context string). Either way no key
            // can sign as things stand, so hosts route it to key repair /
            // address correction instead of an opaque failure.
            PlatformWalletError::MessageSigningKeyUnavailable { .. } => {
                PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
            }
            // DPNS marketplace: the one trade rejection whose Display is
            // sufficient (the other three are handled by
            // `trade_error_json_detail` above and never reach this match).
            PlatformWalletError::DocumentNotForSale { .. } => {
                PlatformWalletFFIResultCode::ErrorDocumentNotForSale
            }
            // An exact-label DPNS lookup that came back empty IS the
            // "does not exist" case this code has always covered, so it
            // rides `NotFound` rather than spending a fifth marketplace
            // code hosts would handle identically.
            PlatformWalletError::DpnsNameNotFound { .. } => PlatformWalletFFIResultCode::NotFound,
            // NOTE: `MessageSigningFailed` is deliberately NOT matched, so it
            // falls to the `ErrorUnknown` catch-all below. Its causes are
            // internal invariant breaks (a public key that does not own the
            // address, no recovery id that recovers it) or an unclassifiable
            // signer failure, which should read as a bug rather than as a
            // key-repair prompt; it carries the signer's own `Display`, which
            // reaches the host in the message either way.
            //
            // The one signer failure with typed meaning never lands here:
            // key-wallet's `Signer::Error` is bounded only by `Display`, so a
            // key-unavailable backend stamps the reserved marker at position 0
            // of its rendering (`MnemonicResolverCoreSigner::NotFound`), and
            // `sign_message` recognizes exactly that — a position-0 check on
            // the UNWRAPPED rendering, never a substring sniff of a composed
            // reason (#4183 review) — and returns the typed
            // `MessageSigningKeyUnavailable` mapped above. By the time a
            // `MessageSigningFailed` reason exists, any marker in it sits
            // mid-string and is deliberately not matched.
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
        // The signer's typed SigningKeyUnavailable completion rides the
        // stable machine prefix at the START of the `ProtocolError::Generic`
        // payload (dashpay/platform#4060 finding 7) — restore the typed code
        // FIRST, before any of the loose keyword sniffs below can misroute
        // it. Match the Generic payload STRUCTURALLY and require the marker at
        // position 0: a foreign signer that only mentions the token somewhere
        // in a human-readable message (`contains`), or that nests it in
        // another variant's Display, must NOT be reclassified as the typed
        // key-unavailable code (dashpay/platform#4183 review).
        let is_key_unavailable = matches!(
            &e,
            dpp::ProtocolError::Generic(s)
                if s.starts_with(rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX)
        );
        let code = if is_key_unavailable {
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        } else if msg.contains("identifier") {
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

    /// A pooled shortfall is the same thing to a host as a single-account one —
    /// "this wallet cannot cover the payment" — so it deliberately rides the
    /// SAME code rather than making every host learn a second value. Pin that,
    /// since splitting it later would silently reclassify the most common send
    /// failure on the pooled (default) path.
    #[test]
    fn pooled_insufficient_funds_shares_the_single_account_code() {
        let result: PlatformWalletFFIResult = PlatformWalletError::CorePooledInsufficientFunds {
            sources: vec![
                AccountTypePreference::BIP44,
                AccountTypePreference::BIP32,
                AccountTypePreference::AllDashpayReceivingFunds,
            ],
            available: Some(1_000),
            required: Some(2_000),
        }
        .into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorCoreInsufficientFunds
        );
    }

    /// A caller-argument rejection raised BELOW the FFI boundary must reach the
    /// host as the same parameter error the boundary itself returns — not as a
    /// not-found, and not through the `ErrorUnknown` catch-all.
    #[test]
    fn invalid_parameter_maps_to_the_parameter_code() {
        let result: PlatformWalletFFIResult =
            PlatformWalletError::InvalidParameter("names a set of accounts".to_string()).into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
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

    /// The asset-lock coin-selection shortfall must cross the FFI boundary as
    /// the dedicated `ErrorAssetLockInsufficientFunds` (29) code — NOT
    /// `ErrorUnknown` (99) as it did before this arm existed
    /// (dashpay/platform#4073) — and its structured `available`/`required`
    /// duffs must survive verbatim in the message so hosts can parse the
    /// amounts.
    #[test]
    fn asset_lock_insufficient_funds_maps_to_dedicated_code() {
        let err = PlatformWalletError::AssetLockInsufficientFunds {
            available: 18_000_000,
            required: 100_000_000,
        };
        let rendered = err.to_string();
        // Guard the exact text hosts (dash-wallet) substring-match on.
        assert!(
            rendered.contains("asset lock coin selection is short"),
            "shortfall Display text changed — coordinate dash-wallet's matcher \
             (rendered: {rendered})"
        );
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorAssetLockInsufficientFunds,
            "must not flatten to ErrorUnknown(99) (rendered: {rendered})"
        );
        assert_ne!(
            result.code as i32,
            PlatformWalletFFIResultCode::ErrorUnknown as i32
        );
        assert!(!result.message.is_null());
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            msg, rendered,
            "structured available/required duffs must survive the FFI boundary verbatim"
        );
    }

    /// The numeric value of `ErrorAssetLockInsufficientFunds` is ABI, mirrored
    /// by hand in the Swift and Kotlin host enums. Pin it so a future
    /// renumbering of the surrounding block cannot silently re-point a host's
    /// shortfall branch at some other error.
    #[test]
    fn asset_lock_insufficient_funds_code_is_pinned_at_29() {
        assert_eq!(
            PlatformWalletFFIResultCode::ErrorAssetLockInsufficientFunds as i32,
            29,
            "code 29 is reserved for the asset-lock shortfall in the FFI \
             error-code registry; hosts mirror the number, not the name"
        );
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

    /// The typed SigningKeyUnavailable signer completion rides the stable
    /// machine prefix through `ProtocolError::Generic`; the conversion must
    /// restore code 31 (dashpay/platform#4060 finding 7) — and must do so
    /// BEFORE the loose keyword sniffs (the human message may well contain
    /// "identifier" or similar).
    #[test]
    fn signer_key_unavailable_prefix_maps_to_code_31() {
        let e = dpp::ProtocolError::Generic(format!(
            "{}no private key stored for identifier 02abcd",
            rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX
        ));
        let result: PlatformWalletFFIResult = e.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        );
    }

    /// The same prefix arriving wrapped in the SDK-error path (the blanket
    /// `From<PlatformWalletError>` catch-all) restores code 31 too — but
    /// only on the catch-all; typed variants keep their codes.
    #[test]
    fn signer_key_unavailable_prefix_maps_on_the_sdk_catch_all() {
        let err = PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(
            format!(
                "{}no private key stored for 02abcd",
                rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX
            ),
        )));
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        );
    }

    /// A second producer of code 31, reached with no marker inspection at
    /// this layer: message signing concluded no key can sign for the address
    /// (no signable account owns it, or the signer reported the key missing
    /// and `sign_message` promoted the position-0 marker to this typed
    /// variant). Hosts branch on it to correct the address or repair the key,
    /// so it must not flatten to ErrorUnknown.
    #[test]
    fn message_signing_key_unavailable_maps_to_code_31() {
        let err = PlatformWalletError::MessageSigningKeyUnavailable {
            address: "yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt".to_string(),
        };
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        );
    }

    /// A generic protocol error without the prefix keeps the historical
    /// mapping — no message sniffing beyond the machine prefix.
    #[test]
    fn generic_protocol_error_without_prefix_is_unchanged() {
        let e = dpp::ProtocolError::Generic("no private key stored for 02abcd".to_string());
        let result: PlatformWalletFFIResult = e.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
    }

    /// A foreign generic (code-0) signer error that merely MENTIONS the
    /// reserved marker somewhere after position 0 must NOT be reclassified as
    /// the typed key-unavailable code — only a marker at the payload start
    /// counts (dashpay/platform#4183 review).
    #[test]
    fn generic_error_with_prefix_as_substring_is_not_code_31() {
        let e = dpp::ProtocolError::Generic(format!(
            "remote signer reported: {}oops",
            rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX
        ));
        let result: PlatformWalletFFIResult = e.into();
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        );
    }

    /// Same guard on the SDK-error catch-all path: the marker mid-message
    /// (not at position 0) must not restore code 31.
    #[test]
    fn sdk_catch_all_with_prefix_as_substring_is_not_code_31() {
        let err = PlatformWalletError::Sdk(dash_sdk::Error::Protocol(dpp::ProtocolError::Generic(
            format!(
                "remote signer reported: {}oops",
                rs_sdk_ffi::DASH_SDK_SIGNER_ERR_KEY_UNAVAILABLE_PREFIX
            ),
        )));
        let result: PlatformWalletFFIResult = err.into();
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorSigningKeyUnavailable
        );
    }

    /// Malformed MESSAGE bytes are caller input just like a malformed address,
    /// so they map to the same ErrorInvalidParameter — not ErrorUnknown, which
    /// would report an internal failure for an argument the caller can fix.
    /// Only reachable across the FFI, where the message arrives as raw bytes.
    #[test]
    fn message_signing_message_invalid_maps_to_invalid_parameter() {
        let err = PlatformWalletError::MessageSigningMessageInvalid {
            address: "yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt".to_string(),
            reason: "invalid utf-8 sequence of 1 bytes from index 2".to_string(),
        };
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        // The rendering must blame the message, not the address.
        let msg = unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned();
        assert!(
            msg.contains("message to sign") && !msg.contains("address is not valid"),
            "the Display must name the message as the malformed argument: {msg}"
        );
    }

    /// A bad message-signing address is caller input, so it maps to the
    /// already-mirrored ErrorInvalidParameter rather than ErrorUnknown.
    #[test]
    fn message_signing_address_invalid_maps_to_invalid_parameter() {
        let err = PlatformWalletError::MessageSigningAddressInvalid {
            address: "not-an-address".to_string(),
            reason: "not a valid Dash address".to_string(),
        };
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
    }

    /// The four DPNS-marketplace trade rejections each map to their own
    /// dedicated code rather than flattening to `ErrorUnknown`, and the
    /// not-found case rides the existing `NotFound`. Hosts branch on these
    /// to distinguish "re-confirm the price" from "top up credits" from
    /// "wait for the contest".
    #[test]
    fn dpns_marketplace_errors_map_to_dedicated_codes() {
        let document_id = dpp::prelude::Identifier::from([9u8; 32]);
        let identity_id = dpp::prelude::Identifier::from([8u8; 32]);
        let cases: Vec<(PlatformWalletError, PlatformWalletFFIResultCode)> = vec![
            (
                PlatformWalletError::DocumentNotForSale { document_id },
                PlatformWalletFFIResultCode::ErrorDocumentNotForSale,
            ),
            (
                PlatformWalletError::DocumentPriceChanged {
                    document_id,
                    expected: 1_000,
                    actual: 2_000,
                },
                PlatformWalletFFIResultCode::ErrorDocumentPriceChanged,
            ),
            (
                PlatformWalletError::InsufficientIdentityCredits {
                    identity_id,
                    required: 100_001_000,
                    available: 7,
                },
                PlatformWalletFFIResultCode::ErrorInsufficientIdentityCredits,
            ),
            (
                PlatformWalletError::ContestedNameNotTradable {
                    label: "alice".to_string(),
                    ends_at_ms: 1_800_000_000_000,
                },
                PlatformWalletFFIResultCode::ErrorContestedNameNotTradable,
            ),
            (
                PlatformWalletError::DpnsNameNotFound {
                    name: "nobody".to_string(),
                },
                PlatformWalletFFIResultCode::NotFound,
            ),
        ];
        for (error, expected_code) in cases {
            let rendered = error.to_string();
            let result: PlatformWalletFFIResult = error.into();
            assert_eq!(
                result.code, expected_code,
                "variant should map to {expected_code:?} (rendered: {rendered})"
            );
        }
    }

    /// Code 37 keeps the typed `Display` rendering as its message — it
    /// carries no value the caller doesn't already have, so it is NOT in
    /// the JSON-detail set.
    #[test]
    fn document_not_for_sale_message_is_the_display_rendering() {
        let err = PlatformWalletError::DocumentNotForSale {
            document_id: dpp::prelude::Identifier::from([9u8; 32]),
        };
        let rendered = err.to_string();
        let result: PlatformWalletFFIResult = err.into();
        assert_eq!(message_of(&result), rendered);
    }

    /// Codes 38/39/40 put a STABLE JSON detail object in the message so
    /// the Swift mirror can rebuild typed cases. Pin the exact keys and
    /// values — a rename or a transposed pair silently degrades every host
    /// to `.unknown`, which no compiler catches across the ABI.
    #[test]
    fn price_changed_message_is_the_documented_json_detail() {
        let document_id = dpp::prelude::Identifier::from([9u8; 32]);
        let result: PlatformWalletFFIResult = PlatformWalletError::DocumentPriceChanged {
            document_id,
            expected: 1_000,
            actual: 2_000,
        }
        .into();
        let parsed: serde_json::Value =
            serde_json::from_str(&message_of(&result)).expect("code 38 message must parse as JSON");
        assert_eq!(
            parsed["documentId"],
            document_id.to_string(Encoding::Base58)
        );
        assert_eq!(parsed["expected"], 1_000u64);
        assert_eq!(parsed["actual"], 2_000u64);
    }

    #[test]
    fn insufficient_credits_message_is_the_documented_json_detail() {
        let identity_id = dpp::prelude::Identifier::from([8u8; 32]);
        let result: PlatformWalletFFIResult = PlatformWalletError::InsufficientIdentityCredits {
            identity_id,
            required: 100_001_000,
            available: 7,
        }
        .into();
        let parsed: serde_json::Value =
            serde_json::from_str(&message_of(&result)).expect("code 39 message must parse as JSON");
        assert_eq!(
            parsed["identityId"],
            identity_id.to_string(Encoding::Base58)
        );
        assert_eq!(parsed["required"], 100_001_000u64);
        assert_eq!(parsed["available"], 7u64);
    }

    #[test]
    fn contested_name_message_is_the_documented_json_detail() {
        let result: PlatformWalletFFIResult = PlatformWalletError::ContestedNameNotTradable {
            label: "alice".to_string(),
            ends_at_ms: 1_800_000_000_000,
        }
        .into();
        let parsed: serde_json::Value =
            serde_json::from_str(&message_of(&result)).expect("code 40 message must parse as JSON");
        assert_eq!(parsed["label"], "alice");
        assert_eq!(parsed["endsAtMs"], 1_800_000_000_000u64);
    }

    /// The numeric values are the ABI contract with the Swift/Kotlin
    /// mirrors (there is no compile-time check across the boundary), so
    /// pin them explicitly rather than trusting declaration order.
    #[test]
    fn dpns_marketplace_codes_are_pinned_at_37_through_40() {
        assert_eq!(
            PlatformWalletFFIResultCode::ErrorDocumentNotForSale as i32,
            37
        );
        assert_eq!(
            PlatformWalletFFIResultCode::ErrorDocumentPriceChanged as i32,
            38
        );
        assert_eq!(
            PlatformWalletFFIResultCode::ErrorInsufficientIdentityCredits as i32,
            39
        );
        assert_eq!(
            PlatformWalletFFIResultCode::ErrorContestedNameNotTradable as i32,
            40
        );
    }

    /// `MessageSigningFailed` is intentionally unmapped: its causes are
    /// internal invariant breaks, which should read as a bug rather than as a
    /// key-repair prompt, so it falls through to ErrorUnknown carrying the
    /// signer's own rendering. Pinned so a future arm cannot silently claim it.
    ///
    /// Note this variant no longer carries malformed-message-bytes, which used
    /// to land here and therefore on ErrorUnknown; they now have their own
    /// `MessageSigningMessageInvalid` mapping to ErrorInvalidParameter. What
    /// remains here is genuinely internal.
    ///
    /// A signer-reported key-unavailable failure never lands on this variant:
    /// `sign_message` checks the reserved marker at position 0 of the signer's
    /// UNWRAPPED rendering and returns the typed
    /// `MessageSigningKeyUnavailable` before composing `reason` as
    /// "signer rejected the digest at {path}: {e}". Once a reason exists, any
    /// marker in it sits mid-string, and matching it there would be the
    /// substring sniff #4183's review rejected. See the NOTE on the mapping
    /// arm.
    #[test]
    fn message_signing_failed_falls_through_to_unknown() {
        let internal = PlatformWalletError::MessageSigningFailed {
            address: "yRd4FhXfVGHXpsuZXPNkMrfD9GVj46pnjt".to_string(),
            reason: "no recovery id in 0..=3 recovers the signing public key".to_string(),
        };
        let result: PlatformWalletFFIResult = internal.into();
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorUnknown);
    }

    /// Read a result's message back as an owned `String`. Every
    /// marketplace assertion below inspects the message, and the raw
    /// `CStr::from_ptr` dance is noise at each site.
    fn message_of(result: &PlatformWalletFFIResult) -> String {
        assert!(!result.message.is_null(), "result carries no message");
        unsafe { std::ffi::CStr::from_ptr(result.message) }
            .to_string_lossy()
            .into_owned()
    }
}
