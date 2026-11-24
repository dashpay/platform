//! FFI bindings for shielded tips to a DashPay username: an ordinary
//! shielded transfer to the shielded address the name's profile publishes.
//!
//! Every decision lives in `platform-wallet`
//! (`platform_wallet::wallet::shielded::tip_payment`): what a typed amount
//! is, whom a name pays, whether that changed, whether an earlier payment to
//! the recipient is unresolved, which account pays, and whether an
//! identity's shielded balance is known. These exports marshal the arguments
//! in and the results out.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::time::Duration;

use dpp::prelude::Identifier;
use platform_wallet::wallet::shielded::CachedOrchardProver;
use platform_wallet::{
    parse_shielded_tip_amount, PlatformWalletError, ShieldedIdentityBalance, ShieldedTipRecipient,
    ShieldedTipRecipientChange, ShieldedTipReview, ShieldedTipSource,
};
use rs_sdk_ffi::MnemonicResolverHandle;

use crate::check_ptr;
use crate::error::*;
use crate::handle::*;
use crate::identity_keys_from_mnemonic::resolve_seed_from_resolver;
use crate::runtime::{block_on_worker, ensure_not_on_runtime, runtime};
use crate::shielded_send::{
    catch_spend_panic, encode_memo_text, map_spend_result, resolve_wallet_and_coordinator,
};
use crate::shielded_sync::{catch_query_panic, map_shielded_error};
use crate::types::read_identifier;

/// How long the identity balance read waits for the coordinator's locks
/// before it reports busy, as the local balance snapshot does.
const IDENTITY_BALANCE_WAIT_BUDGET: Duration = Duration::from_millis(100);

/// How a tip recipient compares with the one this wallet last confirmed for
/// the same name (`ShieldedTipRecipientChange`).
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedTipRecipientChangeFFI {
    /// No recipient was confirmed for the name from this wallet before. A
    /// recipient is confirmed when a send to it is admitted, so none of
    /// these cases says a tip was paid.
    #[default]
    New = 0,
    /// The same identity and address as last confirmed.
    Unchanged = 1,
    /// The name now resolves to another identity, or its profile publishes
    /// another address; `previous_*` hold the recipient last confirmed.
    Changed = 2,
}

/// What the user confirms before tipping a name (`ShieldedTipReview`).
/// Plain data with no heap ownership; nothing to free.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedTipReviewFFI {
    /// The identity the username resolves to.
    pub recipient_identity_id: [u8; 32],
    /// The raw Orchard address its profile publishes.
    pub recipient_address: [u8; 43],
    /// How the recipient compares with the one last confirmed for the name.
    pub change: ShieldedTipRecipientChangeFFI,
    /// The identity last confirmed for the name; all zero unless `change`
    /// is `Changed`.
    pub previous_identity_id: [u8; 32],
    /// The address last confirmed for the name; all zero unless `change` is
    /// `Changed`.
    pub previous_address: [u8; 43],
    /// An earlier shielded payment from this wallet to the recipient may
    /// still execute, so a send to it is refused until a shielded sync
    /// resolves that payment.
    pub unresolved: bool,
}

impl Default for ShieldedTipReviewFFI {
    fn default() -> Self {
        Self {
            recipient_identity_id: [0; 32],
            recipient_address: [0; 43],
            change: ShieldedTipRecipientChangeFFI::New,
            previous_identity_id: [0; 32],
            previous_address: [0; 43],
            unresolved: false,
        }
    }
}

impl From<ShieldedTipReview> for ShieldedTipReviewFFI {
    fn from(review: ShieldedTipReview) -> Self {
        let (change, previous) = match review.change {
            ShieldedTipRecipientChange::New => (ShieldedTipRecipientChangeFFI::New, None),
            ShieldedTipRecipientChange::Unchanged => {
                (ShieldedTipRecipientChangeFFI::Unchanged, None)
            }
            ShieldedTipRecipientChange::Changed { previous } => {
                (ShieldedTipRecipientChangeFFI::Changed, Some(previous))
            }
        };
        let (previous_identity_id, previous_address) = previous
            .map(|previous| (previous.identity_id.to_buffer(), previous.address))
            .unwrap_or(([0; 32], [0; 43]));
        Self {
            recipient_identity_id: review.recipient.identity_id.to_buffer(),
            recipient_address: review.recipient.address,
            change,
            previous_identity_id,
            previous_address,
            unresolved: review.unresolved,
        }
    }
}

/// What the wallet can vouch for about an identity's shielded balance
/// (`ShieldedIdentityBalance`).
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedIdentityBalanceStateFFI {
    /// The seed has not proved in this session which account is the
    /// identity's; preparing the identity's shielded address proves it.
    #[default]
    Unverified = 0,
    /// The account is proved, but its ledger is not established, so its
    /// balance is not known (which is not zero).
    Unavailable = 1,
    /// The account's spendable credits are known.
    Available = 2,
}

/// The shielded balance of a wallet identity's own account. Plain data with
/// no heap ownership; nothing to free.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedIdentityBalanceFFI {
    pub state: ShieldedIdentityBalanceStateFFI,
    /// The identity's shielded account; 0 when `state` is `Unverified`.
    pub account: u32,
    /// Spendable credits; 0 unless `state` is `Available`.
    pub credits: u64,
}

impl From<ShieldedIdentityBalance> for ShieldedIdentityBalanceFFI {
    fn from(balance: ShieldedIdentityBalance) -> Self {
        match balance {
            ShieldedIdentityBalance::Unverified => Self::default(),
            ShieldedIdentityBalance::Unavailable { account } => Self {
                state: ShieldedIdentityBalanceStateFFI::Unavailable,
                account,
                credits: 0,
            },
            ShieldedIdentityBalance::Available { account, credits } => Self {
                state: ShieldedIdentityBalanceStateFFI::Available,
                account,
                credits,
            },
        }
    }
}

/// Read a required NUL-terminated UTF-8 argument, naming it in the error.
///
/// # Safety
/// `ptr` must be non-null and a valid NUL-terminated C string for the
/// duration of the call.
unsafe fn read_utf8<'a>(
    ptr: *const c_char,
    name: &str,
) -> Result<&'a str, PlatformWalletFFIResult> {
    CStr::from_ptr(ptr).to_str().map_err(|e| {
        PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorUtf8Conversion,
            format!("{name} is not valid UTF-8: {e}"),
        )
    })
}

/// Parse a tip amount the user typed in DASH into exact credits, with
/// `platform_wallet::parse_shielded_tip_amount`. A host shows the parsed
/// amount in its confirmation, not the typed text.
///
/// No handle and no wallet: a pure parse. `out_credits` is zeroed on entry
/// and written only on success. Result codes:
/// - `ErrorUtf8Conversion`: `amount_text` is not valid UTF-8.
/// - `ErrorInvalidParameter`: the wallet refuses the amount (a sign, digit
///   grouping, an exponent, zero, more than `u64` credits, or digits finer
///   than one credit); the message carries the reason.
///
/// # Safety
/// - `amount_text` must be a valid NUL-terminated C string for the duration
///   of the call.
/// - `out_credits` must point to 8 writable bytes (a `u64`).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_parse_shielded_tip_amount(
    amount_text: *const c_char,
    out_credits: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(out_credits);
    *out_credits = 0;
    check_ptr!(amount_text);
    let text = match read_utf8(amount_text, "amount_text") {
        Ok(text) => text,
        Err(result) => return result,
    };
    match parse_shielded_tip_amount(text) {
        Ok(credits) => {
            *out_credits = credits;
            PlatformWalletFFIResult::ok()
        }
        Err(e) => e.into(),
    }
}

/// Resolve `username` for the user to confirm a shielded tip, with
/// `PlatformWallet::review_shielded_tip`: whom it pays now (the identity the
/// DPNS name resolves to and the shielded address its DashPay profile
/// publishes, both fetched with proof), how that compares with the recipient
/// this wallet last confirmed for the name, and whether an earlier shielded
/// payment to that recipient is still unresolved. Changes nothing and moves
/// no funds. Fetches from the network.
///
/// `out_review` is zeroed (`ShieldedTipReviewFFI`'s default) on entry and
/// written only on success. Show the recipient and the change, then pass
/// `recipient_identity_id` and `recipient_address` back to
/// [`platform_wallet_manager_send_shielded_tip`] exactly as returned.
///
/// `contact_identity_id` is the identity the host expects the name to belong
/// to, when the tip starts from a saved contact: pass the contact's identity
/// id, and a name that now resolves to another identity is refused with
/// `ErrorShieldedTipRecipientMismatch`. Pass null when the user typed the
/// name: the wallet then expects nothing.
///
/// Result codes:
/// - `ErrorUtf8Conversion`: `username` is not valid UTF-8.
/// - `ErrorShieldedTipRecipientMismatch` (86): `contact_identity_id` is
///   non-null and the name resolves to another identity; nothing changed.
/// - `ErrorShieldedRecoveryCorrupted` / `ErrorShieldedRecoveryKeysRequired`:
///   the wallet cannot tell whom a saved shielded spend pays.
/// - `ErrorInvalidHandle`: `handle` is not a live manager handle.
/// - `ErrorWalletOperation`: every other failure, including a name that does
///   not exist, a profile that publishes no valid shielded address, a wallet
///   whose shielded accounts are not bound, an unknown wallet under a valid
///   manager and a shielded coordinator that is not configured. The message
///   carries the wallet's reason.
///
/// `unresolved` reports a saved transfer to the recipient. It does not
/// report another tip being sent, so a send can still return
/// `ErrorShieldedTipPending` after a review that said `false`.
///
/// Blocks, so call it off the UI thread, and not from a callback this
/// library invokes (the shielded balance, sync progress and persistence
/// callbacks): from there it returns `ErrorWalletOperation` at once.
///
/// In a build that unwinds (Android and the host tests), a panic inside the
/// call is returned as `ErrorWalletOperation`; the iOS profiles build with
/// `panic = "abort"`.
///
/// # Safety
/// - `wallet_id_bytes` must point to 32 readable bytes.
/// - `username` must be a valid NUL-terminated C string for the duration of
///   the call.
/// - `contact_identity_id` must be null or point to 32 readable bytes.
/// - `out_review` must point to a writable `ShieldedTipReviewFFI`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_review_shielded_tip(
    handle: Handle,
    wallet_id_bytes: *const u8,
    username: *const c_char,
    contact_identity_id: *const u8,
    out_review: *mut ShieldedTipReviewFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_review);
    // Read before the output is zeroed: a caller may pass the recipient of
    // an earlier review, held in the same struct it passes as `out_review`.
    let contact_identity = read_optional_identifier(contact_identity_id);
    *out_review = ShieldedTipReviewFFI::default();
    let contact_identity = match contact_identity {
        Ok(id) => id,
        Err(result) => return result,
    };
    check_ptr!(wallet_id_bytes);
    check_ptr!(username);
    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let username = match read_utf8(username, "username") {
        Ok(username) => username.to_owned(),
        Err(result) => return result,
    };
    if let Err(result) = ensure_not_on_runtime("shielded tip review") {
        return result;
    }
    // Resolution blocks on the runtime, so it sits inside the boundary with
    // the call. The raw output write happens outside it and only on success.
    let mut review = ShieldedTipReviewFFI::default();
    let result = catch_query_panic("shielded tip review", || {
        let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
            Ok(value) => value,
            Err(result) => return result,
        };
        match block_on_worker(async move {
            wallet
                .review_shielded_tip(&coordinator, &username, contact_identity.as_ref())
                .await
        }) {
            Ok(value) => {
                review = value.into();
                PlatformWalletFFIResult::ok()
            }
            Err(e) => map_tip_review_error(e),
        }
    });
    if result.code == PlatformWalletFFIResultCode::Success {
        *out_review = review;
    }
    result
}

/// Map a failed tip review as the shielded reads map theirs
/// ([`map_shielded_error`]), keeping one more code typed: a name that belongs
/// to another identity than the contact the tip was started from.
fn map_tip_review_error(error: PlatformWalletError) -> PlatformWalletFFIResult {
    match error {
        e @ PlatformWalletError::ShieldedTipRecipientMismatch { .. } => e.into(),
        other => map_shielded_error(other, "review shielded tip"),
    }
}

/// Read an optional 32-byte identifier: null is `None`.
///
/// # Safety
/// `ptr` must be null or point to 32 readable bytes.
unsafe fn read_optional_identifier(
    ptr: *const u8,
) -> Result<Option<Identifier>, PlatformWalletFFIResult> {
    if ptr.is_null() {
        return Ok(None);
    }
    read_identifier(ptr).map(Some).map_err(Into::into)
}

/// Send a shielded tip of `amount` credits to `username`, with
/// `PlatformWallet::send_shielded_tip`.
///
/// The tip pays only the recipient the user confirmed: pass
/// `expected_identity_id` and `expected_address` back exactly as
/// [`platform_wallet_manager_review_shielded_tip`] returned them. The wallet
/// resolves the name again and refuses a recipient that differs.
///
/// `contact_identity_id` is a different thing from `expected_identity_id`:
/// the identity the host expects the name to belong to, when the tip starts
/// from a saved contact. Pass the contact's identity id, and a reviewed
/// recipient of another identity is refused with
/// `ErrorShieldedTipRecipientMismatch`, also when the review was not given
/// it. Pass null when the user typed the name: the wallet then expects
/// nothing.
///
/// The paying account: a non-null `source_identity_id` pays from that wallet
/// identity's own shielded account (`ShieldedTipSource::Identity`, proved
/// with the seed) and `source_account` is ignored; a null one pays from
/// account `source_account` (`ShieldedTipSource::Account`).
///
/// `mnemonic_resolver_handle` supplies the Orchard spend authority for this
/// call only, as for `platform_wallet_manager_shielded_transfer`. `memo_text`
/// is an optional UTF-8 memo on the recipient's note, encoded as that export
/// encodes it (null or empty: no memo; at most 32 UTF-8 bytes).
///
/// Result codes a host must tell apart:
/// - `ErrorShieldedSpendUnconfirmed` (18): the tip may have left the wallet
///   and may still execute. Never send it again; a shielded sync resolves it.
///   This is also what a panic inside the call returns in a build that
///   unwinds. `ErrorTransactionBroadcastUnconfirmed` (20), which the shielded
///   spend contract keeps for an SDK outcome it reports unconfirmed, means
///   the same.
/// - `ErrorShieldedTipPending` (85): refused, nothing sent. Another tip from
///   this wallet is being sent (whoever it pays; the review does not report
///   this), or an earlier shielded transfer to the recipient may still
///   execute. This call sent nothing, but the earlier payment may be this
///   same tip and may pay the recipient: a shielded sync resolves it as
///   executed or as unable to execute, and the refusal ends either way. Do
///   not send the tip again because the refusal ended; send it again only
///   once the earlier payment is known to have failed (its shielded
///   activity entry reads failed, not confirmed). The wallet admits the tip
///   before the mnemonic is resolved, so a second call made while the first
///   runs is refused however long its resolver takes.
/// - `ErrorShieldedTipRecipientMismatch` (86): `contact_identity_id` is
///   non-null and the reviewed recipient is another identity: the name moved
///   since the contact was saved. Nothing sent; the user picks the recipient
///   again.
/// - `ErrorIdentityNotFound` (7): the wallet holds no identity
///   `source_identity_id`. Nothing sent.
/// - Every other code: this call sent nothing (for example
///   `ErrorWalletOperation` for a recipient that changed since the review
///   or for a call made from a callback this library invokes,
///   `ErrorInvalidParameter` for a tip to the account it is paid from,
///   `ErrorUtf8Conversion` for a bad string, `ErrorInvalidParameter` for an
///   over-long memo, `ErrorShieldedNoRecordedAnchor` while the tree catches
///   up).
///
/// # Safety
/// - `wallet_id_bytes` and `expected_identity_id` must point to 32 readable
///   bytes; `expected_address` to 43.
/// - `source_identity_id` and `contact_identity_id` must each be null or
///   point to 32 readable bytes.
/// - `mnemonic_resolver_handle` must come from
///   `dash_sdk_mnemonic_resolver_create` and outlive this call; the caller
///   retains ownership.
/// - `username` must be a valid NUL-terminated C string, and `memo_text`
///   null or one, for the duration of the call.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn platform_wallet_manager_send_shielded_tip(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    source_identity_id: *const u8,
    source_account: u32,
    username: *const c_char,
    expected_identity_id: *const u8,
    expected_address: *const u8,
    contact_identity_id: *const u8,
    amount: u64,
    memo_text: *const c_char,
) -> PlatformWalletFFIResult {
    // A panic can strike after the notes were reserved or the transfer was
    // broadcast, so it reports the unconfirmed code, never a failure.
    catch_spend_panic(SEND_TIP_OPERATION, || {
        send_shielded_tip_inner(
            handle,
            wallet_id_bytes,
            mnemonic_resolver_handle,
            source_identity_id,
            source_account,
            username,
            expected_identity_id,
            expected_address,
            contact_identity_id,
            amount,
            memo_text,
        )
    })
}

/// The operation name the tip send export reports.
const SEND_TIP_OPERATION: &str = "shielded tip";

/// Body of [`platform_wallet_manager_send_shielded_tip`], as an ordinary Rust
/// function so a panic unwinds into [`catch_spend_panic`] instead of across
/// the C ABI.
///
/// # Safety
/// Identical contract to the export that calls it.
#[allow(clippy::too_many_arguments)]
unsafe fn send_shielded_tip_inner(
    handle: Handle,
    wallet_id_bytes: *const u8,
    mnemonic_resolver_handle: *mut MnemonicResolverHandle,
    source_identity_id: *const u8,
    source_account: u32,
    username: *const c_char,
    expected_identity_id: *const u8,
    expected_address: *const u8,
    contact_identity_id: *const u8,
    amount: u64,
    memo_text: *const c_char,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id_bytes);
    check_ptr!(mnemonic_resolver_handle);
    check_ptr!(username);
    check_ptr!(expected_identity_id);
    check_ptr!(expected_address);

    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let source = if source_identity_id.is_null() {
        ShieldedTipSource::Account(source_account)
    } else {
        match read_identifier(source_identity_id) {
            Ok(id) => ShieldedTipSource::Identity(id),
            Err(e) => return e.into(),
        }
    };
    let expected = ShieldedTipRecipient {
        identity_id: match read_identifier(expected_identity_id) {
            Ok(id) => id,
            Err(e) => return e.into(),
        },
        address: {
            let mut address = [0u8; 43];
            std::ptr::copy_nonoverlapping(expected_address, address.as_mut_ptr(), 43);
            address
        },
    };
    let contact_identity = match read_optional_identifier(contact_identity_id) {
        Ok(id) => id,
        Err(result) => return result,
    };
    let username = match read_utf8(username, "username") {
        Ok(username) => username.to_owned(),
        Err(result) => return result,
    };
    let memo_str = if memo_text.is_null() {
        None
    } else {
        match read_utf8(memo_text, "memo_text") {
            Ok(text) => Some(text),
            Err(result) => return result,
        }
    };
    let memo = match encode_memo_text(memo_str) {
        Ok(memo) => memo,
        Err(result) => return result,
    };

    if let Err(result) = ensure_not_on_runtime(SEND_TIP_OPERATION) {
        return result;
    }
    let Some(sender) = PLATFORM_WALLET_MANAGER_STORAGE.with_item(handle, |manager| {
        runtime().block_on(manager.shielded_tip_sender(&wallet_id))
    }) else {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidHandle,
            format!("invalid manager handle: {handle}"),
        );
    };
    let sender = match sender {
        Ok(sender) => sender,
        Err(e) => return map_tip_send_result(Err(e)),
    };
    // Admitted before the mnemonic is resolved: the resolver is the host's
    // and can take as long as a prompt stays open. A second call would
    // otherwise wait in it past the first tip's whole send, and then find
    // neither a tip in flight nor an unresolved transfer.
    let admission = match sender.admit() {
        Ok(admission) => admission,
        Err(e) => return map_tip_send_result(Err(e)),
    };
    let seed = match resolve_seed_from_resolver(mnemonic_resolver_handle, &wallet_id) {
        Ok(seed) => seed,
        Err(result) => return result,
    };

    // The proof runs on the 8 MB worker: Halo 2 circuit synthesis recurses
    // past the iOS dispatch-thread stack.
    let result = block_on_worker(async move {
        let prover = CachedOrchardProver::new();
        sender
            .send(
                admission,
                seed.as_ref(),
                source,
                &username,
                &expected,
                contact_identity.as_ref(),
                amount,
                memo,
                &prover,
            )
            .await
    });
    map_tip_send_result(result)
}

/// Map the outcome of a tip send as the shielded spends map theirs
/// ([`map_spend_result`]), keeping three more codes typed: an identity the
/// wallet does not hold, named as the paying identity; a reviewed recipient
/// of another identity than the contact the tip was started from; and an
/// argument the wallet refuses, a tip to the account it is paid from among
/// them. Nothing was sent in any of these.
fn map_tip_send_result(result: Result<(), PlatformWalletError>) -> PlatformWalletFFIResult {
    match result {
        Err(e @ PlatformWalletError::IdentityNotFound(..))
        | Err(e @ PlatformWalletError::ShieldedTipRecipientMismatch { .. })
        | Err(e @ PlatformWalletError::InvalidParameter(..)) => e.into(),
        other => map_spend_result(other, SEND_TIP_OPERATION),
    }
}

/// Read the shielded balance of the wallet identity `identity_id_bytes`'s own
/// account, with `PlatformWallet::shielded_identity_balance`. Reads local
/// state only. A host shows the state and does not add up stored notes.
///
/// `out_balance` is zeroed (`Unverified`, account 0, credits 0) on entry and
/// written only on success, where it holds:
/// - `Unverified`: the seed has not proved in this session which account is
///   the identity's; `platform_wallet_manager_prepare_shielded_identity_address`
///   proves it.
/// - `Unavailable` with `account`: the account's ledger is not established
///   (not bound, or neither restored nor scanned); the balance is not known.
/// - `Available` with `account` and `credits`.
///
/// Waits at most 100 ms for the coordinator's locks, as
/// `platform_wallet_manager_local_shielded_balance_snapshot` does. Result
/// codes:
/// - `ErrorShieldedBalanceBusy` (87): the read did not get the shielded store
///   within its wait budget, because a scan or a spend holds it. Nothing
///   failed: keep the last state shown and read again later. The only code
///   of this call that means that.
/// - `ErrorIdentityNotFound`: the wallet holds no identity with that id.
/// - `ErrorWalletOperation`: the wallet is unknown, no shielded coordinator
///   is configured, the call was made from a callback this library invokes
///   (the shielded balance callback among them: read the balance from
///   another thread), or another failure, with the reason in the message.
/// - `ErrorShieldedRecoveryCorrupted` / `ErrorShieldedRecoveryKeysRequired`:
///   the wallet's persisted shielded state is unusable.
///
/// Every code other than `Success` and `ErrorShieldedBalanceBusy` is a
/// failure to show, not a reason to read again.
///
/// In a build that unwinds, a panic inside the call is returned as
/// `ErrorWalletOperation`; the iOS profiles build with `panic = "abort"`.
///
/// # Safety
/// - `wallet_id_bytes` and `identity_id_bytes` must point to 32 readable bytes.
/// - `out_balance` must point to a writable `ShieldedIdentityBalanceFFI`.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_shielded_identity_balance(
    handle: Handle,
    wallet_id_bytes: *const u8,
    identity_id_bytes: *const u8,
    out_balance: *mut ShieldedIdentityBalanceFFI,
) -> PlatformWalletFFIResult {
    check_ptr!(out_balance);
    *out_balance = ShieldedIdentityBalanceFFI::default();
    check_ptr!(wallet_id_bytes);
    check_ptr!(identity_id_bytes);
    let mut wallet_id = [0u8; 32];
    std::ptr::copy_nonoverlapping(wallet_id_bytes, wallet_id.as_mut_ptr(), 32);
    let identity_id: Identifier = match read_identifier(identity_id_bytes) {
        Ok(id) => id,
        Err(e) => return e.into(),
    };
    if let Err(result) = ensure_not_on_runtime("shielded identity balance") {
        return result;
    }
    let mut balance = ShieldedIdentityBalanceFFI::default();
    let result = catch_query_panic("shielded identity balance", || {
        // Only the owned Arcs cross the wait: the registry guard is released
        // once resolution returns, as in the local balance snapshot.
        let (wallet, coordinator) = match resolve_wallet_and_coordinator(handle, &wallet_id) {
            Ok(value) => value,
            Err(result) => return result,
        };
        let read = runtime().block_on(async {
            tokio::time::timeout(
                IDENTITY_BALANCE_WAIT_BUDGET,
                wallet.shielded_identity_balance(&coordinator, &identity_id),
            )
            .await
        });
        match read {
            Err(_) => identity_balance_busy(),
            Ok(Ok(value)) => {
                balance = value.into();
                PlatformWalletFFIResult::ok()
            }
            Ok(Err(e @ PlatformWalletError::IdentityNotFound(..))) => e.into(),
            Ok(Err(e)) => map_shielded_error(e, "shielded identity balance"),
        }
    });
    if result.code == PlatformWalletFFIResultCode::Success {
        *out_balance = balance;
    }
    result
}

/// What the identity balance read reports when it did not get the shielded
/// store within `IDENTITY_BALANCE_WAIT_BUDGET`.
fn identity_balance_busy() -> PlatformWalletFFIResult {
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorShieldedBalanceBusy,
        "shielded identity balance busy; retry after sync releases its locks",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const INVALID_HANDLE: Handle = 0xDEAD_BEEF;
    /// `0xFF` alone is never valid UTF-8.
    const BAD_UTF8: &[u8] = b"\xff\0";

    fn message(result: &PlatformWalletFFIResult) -> String {
        unsafe { CStr::from_ptr(result.message) }
            .to_str()
            .expect("UTF-8 message")
            .to_owned()
    }

    fn free(mut result: PlatformWalletFFIResult) {
        unsafe { platform_wallet_ffi_result_free(&mut result) };
    }

    fn dirty_review() -> ShieldedTipReviewFFI {
        ShieldedTipReviewFFI {
            recipient_identity_id: [1; 32],
            recipient_address: [2; 43],
            change: ShieldedTipRecipientChangeFFI::Changed,
            previous_identity_id: [3; 32],
            previous_address: [4; 43],
            unresolved: true,
        }
    }

    fn dirty_balance() -> ShieldedIdentityBalanceFFI {
        ShieldedIdentityBalanceFFI {
            state: ShieldedIdentityBalanceStateFFI::Available,
            account: 9,
            credits: 9,
        }
    }

    #[test]
    fn should_parse_a_tip_amount_through_the_c_function() {
        let mut credits = 7u64;
        let result =
            unsafe { platform_wallet_parse_shielded_tip_amount(c"1,5".as_ptr(), &mut credits) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(credits, 150_000_000_000);
        free(result);
    }

    #[test]
    fn should_refuse_a_tip_amount_and_leave_the_output_zero() {
        let mut credits = 7u64;
        let result =
            unsafe { platform_wallet_parse_shielded_tip_amount(c"-1".as_ptr(), &mut credits) };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        assert_eq!(credits, 0);
        free(result);
    }

    #[test]
    fn should_report_bad_utf8_and_null_pointers_when_parsing_a_tip_amount() {
        let mut credits = 7u64;
        let result = unsafe {
            platform_wallet_parse_shielded_tip_amount(BAD_UTF8.as_ptr().cast(), &mut credits)
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert_eq!(credits, 0);
        free(result);

        let mut credits = 7u64;
        let result =
            unsafe { platform_wallet_parse_shielded_tip_amount(std::ptr::null(), &mut credits) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(credits, 0);
        free(result);

        let result = unsafe {
            platform_wallet_parse_shielded_tip_amount(c"1".as_ptr(), std::ptr::null_mut())
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);
    }

    #[test]
    fn should_zero_the_review_and_report_null_pointers_and_bad_utf8() {
        let wallet_id = [0u8; 32];
        let mut review = dirty_review();
        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                std::ptr::null(),
                c"alice".as_ptr(),
                std::ptr::null(),
                &mut review,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(review, ShieldedTipReviewFFI::default());
        free(result);

        let mut review = dirty_review();
        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                BAD_UTF8.as_ptr().cast(),
                std::ptr::null(),
                &mut review,
            )
        };
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert_eq!(review, ShieldedTipReviewFFI::default());
        free(result);

        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                c"alice".as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);
    }

    #[test]
    fn should_leave_the_review_zeroed_for_an_invalid_handle() {
        let wallet_id = [0u8; 32];
        let mut review = dirty_review();
        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                c"alice".as_ptr(),
                std::ptr::null(),
                &mut review,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(review, ShieldedTipReviewFFI::default());
        free(result);
    }

    #[test]
    fn should_report_null_pointers_and_bad_utf8_when_sending_a_tip() {
        let wallet_id = [0u8; 32];
        let identity = [0u8; 32];
        let address = [0u8; 43];
        // Not dereferenced: the strings fail before the resolver is used.
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let send = |resolver: *mut MnemonicResolverHandle,
                    username: *const c_char,
                    expected_address: *const u8,
                    memo: *const c_char| unsafe {
            platform_wallet_manager_send_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                resolver,
                std::ptr::null(),
                0,
                username,
                identity.as_ptr(),
                expected_address,
                std::ptr::null(),
                1,
                memo,
            )
        };

        let result = send(
            std::ptr::null_mut(),
            c"alice".as_ptr(),
            address.as_ptr(),
            std::ptr::null(),
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);

        let result = send(
            resolver,
            std::ptr::null(),
            address.as_ptr(),
            std::ptr::null(),
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);

        let result = send(
            resolver,
            c"alice".as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
        );
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);

        let result = send(
            resolver,
            BAD_UTF8.as_ptr().cast(),
            address.as_ptr(),
            std::ptr::null(),
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        free(result);

        let result = send(
            resolver,
            c"alice".as_ptr(),
            address.as_ptr(),
            BAD_UTF8.as_ptr().cast(),
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorUtf8Conversion
        );
        assert!(message(&result).contains("memo_text"));
        free(result);

        let result = send(
            resolver,
            c"alice".as_ptr(),
            address.as_ptr(),
            c"a memo far longer than thirty-two bytes".as_ptr(),
        );
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        free(result);
    }

    #[test]
    fn should_report_an_invalid_handle_when_sending_a_tip() {
        let wallet_id = [0u8; 32];
        let identity = [0u8; 32];
        let address = [0u8; 43];
        // Not dereferenced: the handle fails before the resolver is used.
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let result = unsafe {
            platform_wallet_manager_send_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                resolver,
                identity.as_ptr(),
                0,
                c"alice".as_ptr(),
                identity.as_ptr(),
                address.as_ptr(),
                std::ptr::null(),
                1,
                std::ptr::null(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        free(result);
    }

    /// A contact identity changes nothing about the local checks: a null
    /// argument and an invalid handle fail as before, with the review left
    /// zeroed.
    #[test]
    fn should_fail_locally_with_a_contact_identity_and_leave_the_review_zeroed() {
        let wallet_id = [0u8; 32];
        let contact = [9u8; 32];
        let mut review = dirty_review();
        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                std::ptr::null(),
                c"alice".as_ptr(),
                contact.as_ptr(),
                &mut review,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(review, ShieldedTipReviewFFI::default());
        free(result);

        let mut review = dirty_review();
        let result = unsafe {
            platform_wallet_manager_review_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                c"alice".as_ptr(),
                contact.as_ptr(),
                &mut review,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_eq!(review, ShieldedTipReviewFFI::default());
        free(result);
    }

    /// A contact identity changes nothing about the local checks of a send:
    /// a null argument and an invalid handle fail as before.
    #[test]
    fn should_fail_locally_with_a_contact_identity_when_sending_a_tip() {
        let wallet_id = [0u8; 32];
        let identity = [0u8; 32];
        let address = [0u8; 43];
        let contact = [9u8; 32];
        // Not dereferenced: both calls fail before the resolver is used.
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let send = |expected_address: *const u8| unsafe {
            platform_wallet_manager_send_shielded_tip(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                resolver,
                std::ptr::null(),
                0,
                c"alice".as_ptr(),
                identity.as_ptr(),
                expected_address,
                contact.as_ptr(),
                1,
                std::ptr::null(),
            )
        };

        let result = send(std::ptr::null());
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        free(result);

        let result = send(address.as_ptr());
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        free(result);
    }

    #[test]
    fn should_zero_the_identity_balance_and_report_null_pointers() {
        let wallet_id = [0u8; 32];
        let mut balance = dirty_balance();
        let result = unsafe {
            platform_wallet_manager_shielded_identity_balance(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                std::ptr::null(),
                &mut balance,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_eq!(balance, ShieldedIdentityBalanceFFI::default());
        free(result);

        let result = unsafe {
            platform_wallet_manager_shielded_identity_balance(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                wallet_id.as_ptr(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorNullPointer);
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBalanceBusy
        );
        free(result);
    }

    #[test]
    fn should_leave_the_identity_balance_zeroed_for_an_invalid_handle() {
        let wallet_id = [0u8; 32];
        let mut balance = dirty_balance();
        let result = unsafe {
            platform_wallet_manager_shielded_identity_balance(
                INVALID_HANDLE,
                wallet_id.as_ptr(),
                wallet_id.as_ptr(),
                &mut balance,
            )
        };
        assert_eq!(result.code, PlatformWalletFFIResultCode::ErrorInvalidHandle);
        assert_ne!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBalanceBusy
        );
        assert_eq!(balance, ShieldedIdentityBalanceFFI::default());
        free(result);
    }

    /// A busy identity balance read reports the one code that means "keep
    /// the last state and read again", with its reason.
    #[test]
    fn should_report_a_busy_identity_balance_by_its_own_code() {
        let result = identity_balance_busy();
        assert_eq!(
            result.code,
            PlatformWalletFFIResultCode::ErrorShieldedBalanceBusy
        );
        assert_eq!(result.code as u32, 87);
        assert!(
            message(&result).contains("shielded identity balance busy"),
            "{}",
            message(&result)
        );
        free(result);
    }

    #[test]
    fn should_carry_the_previous_recipient_only_when_it_changed() {
        let recipient = ShieldedTipRecipient {
            identity_id: Identifier::from([5; 32]),
            address: [6; 43],
        };
        let changed = ShieldedTipReviewFFI::from(ShieldedTipReview {
            recipient: recipient.clone(),
            change: ShieldedTipRecipientChange::Changed {
                previous: ShieldedTipRecipient {
                    identity_id: Identifier::from([7; 32]),
                    address: [8; 43],
                },
            },
            unresolved: true,
        });
        assert_eq!(changed.recipient_identity_id, [5; 32]);
        assert_eq!(changed.recipient_address, [6; 43]);
        assert_eq!(changed.change, ShieldedTipRecipientChangeFFI::Changed);
        assert_eq!(changed.previous_identity_id, [7; 32]);
        assert_eq!(changed.previous_address, [8; 43]);
        assert!(changed.unresolved);

        let unchanged = ShieldedTipReviewFFI::from(ShieldedTipReview {
            recipient,
            change: ShieldedTipRecipientChange::Unchanged,
            unresolved: false,
        });
        assert_eq!(unchanged.change, ShieldedTipRecipientChangeFFI::Unchanged);
        assert_eq!(unchanged.previous_identity_id, [0; 32]);
        assert_eq!(unchanged.previous_address, [0; 43]);
    }

    #[test]
    fn should_map_each_identity_balance_state() {
        assert_eq!(
            ShieldedIdentityBalanceFFI::from(ShieldedIdentityBalance::Unverified),
            ShieldedIdentityBalanceFFI::default()
        );
        assert_eq!(
            ShieldedIdentityBalanceFFI::from(ShieldedIdentityBalance::Unavailable { account: 3 }),
            ShieldedIdentityBalanceFFI {
                state: ShieldedIdentityBalanceStateFFI::Unavailable,
                account: 3,
                credits: 0,
            }
        );
        assert_eq!(
            ShieldedIdentityBalanceFFI::from(ShieldedIdentityBalance::Available {
                account: 3,
                credits: 42,
            }),
            ShieldedIdentityBalanceFFI {
                state: ShieldedIdentityBalanceStateFFI::Available,
                account: 3,
                credits: 42,
            }
        );
    }

    /// Called from a callback this library invokes, the caller is inside the
    /// runtime the exports block on. They refuse instead of panicking there
    /// (the iOS profiles abort on a panic), with the outputs left zeroed.
    #[test]
    fn should_refuse_a_call_made_from_inside_the_runtime() {
        let wallet_id = [0u8; 32];
        let identity_id = [1u8; 32];
        let address = [2u8; 43];
        let resolver = std::ptr::NonNull::<MnemonicResolverHandle>::dangling().as_ptr();
        let (review_result, review, balance_result, balance, send_result) =
            runtime().block_on(async {
                let mut review = dirty_review();
                let review_result = unsafe {
                    platform_wallet_manager_review_shielded_tip(
                        INVALID_HANDLE,
                        wallet_id.as_ptr(),
                        c"alice".as_ptr(),
                        std::ptr::null(),
                        &mut review,
                    )
                };
                let mut balance = dirty_balance();
                let balance_result = unsafe {
                    platform_wallet_manager_shielded_identity_balance(
                        INVALID_HANDLE,
                        wallet_id.as_ptr(),
                        identity_id.as_ptr(),
                        &mut balance,
                    )
                };
                let send_result = unsafe {
                    platform_wallet_manager_send_shielded_tip(
                        INVALID_HANDLE,
                        wallet_id.as_ptr(),
                        resolver,
                        std::ptr::null(),
                        0,
                        c"alice".as_ptr(),
                        identity_id.as_ptr(),
                        address.as_ptr(),
                        std::ptr::null(),
                        1,
                        std::ptr::null(),
                    )
                };
                (review_result, review, balance_result, balance, send_result)
            });
        assert_eq!(review, ShieldedTipReviewFFI::default());
        assert_eq!(balance, ShieldedIdentityBalanceFFI::default());
        for result in [review_result, balance_result, send_result] {
            assert_eq!(
                result.code,
                PlatformWalletFFIResultCode::ErrorWalletOperation
            );
            assert!(
                message(&result).contains("from inside a wallet callback"),
                "{}",
                message(&result)
            );
            free(result);
        }
    }

    fn recipient_mismatch() -> PlatformWalletError {
        PlatformWalletError::ShieldedTipRecipientMismatch {
            expected: Identifier::from([1; 32]),
            resolved: Identifier::from([2; 32]),
        }
    }

    /// A review refused because the name belongs to another identity than
    /// the contact keeps its own code; other failures map as the shielded
    /// reads map theirs.
    #[test]
    fn should_report_a_review_recipient_mismatch_by_its_own_code() {
        let mismatch = map_tip_review_error(recipient_mismatch());
        assert_eq!(
            mismatch.code,
            PlatformWalletFFIResultCode::ErrorShieldedTipRecipientMismatch
        );
        assert_eq!(mismatch.code as u32, 86);
        free(mismatch);

        let other = map_tip_review_error(PlatformWalletError::ShieldedNotBound);
        assert_eq!(
            other.code,
            PlatformWalletFFIResultCode::ErrorWalletOperation
        );
        free(other);
    }

    /// A refused tip, a recipient of another identity than the contact and
    /// an unknown source identity keep their own codes in what the send
    /// export reports, and a tip that may have left the wallet keeps the
    /// unconfirmed code: none of the first three is that code.
    #[test]
    fn should_report_a_tip_send_outcome_by_its_own_code() {
        let pending = map_tip_send_result(Err(PlatformWalletError::ShieldedTipPending));
        assert_eq!(
            pending.code,
            PlatformWalletFFIResultCode::ErrorShieldedTipPending
        );
        assert_eq!(pending.code as u32, 85);
        free(pending);

        let mismatch = map_tip_send_result(Err(recipient_mismatch()));
        assert_eq!(
            mismatch.code,
            PlatformWalletFFIResultCode::ErrorShieldedTipRecipientMismatch
        );
        assert_eq!(mismatch.code as u32, 86);
        free(mismatch);

        let unknown =
            map_tip_send_result(Err(PlatformWalletError::IdentityNotFound([7; 32].into())));
        assert_eq!(
            unknown.code,
            PlatformWalletFFIResultCode::ErrorIdentityNotFound
        );
        assert_eq!(unknown.code as u32, 7);
        free(unknown);

        let unconfirmed = map_tip_send_result(Err(PlatformWalletError::ShieldedSpendUnconfirmed {
            operation: "transfer",
            reason: "result wait timed out".to_string(),
        }));
        assert_eq!(
            unconfirmed.code,
            PlatformWalletFFIResultCode::ErrorShieldedSpendUnconfirmed
        );
        free(unconfirmed);

        // A tip to the account it is paid from: the documented code 2.
        let refused = map_tip_send_result(Err(PlatformWalletError::InvalidParameter(
            "A tip cannot pay the shielded account it is paid from".to_string(),
        )));
        assert_eq!(
            refused.code,
            PlatformWalletFFIResultCode::ErrorInvalidParameter
        );
        free(refused);

        let sent = map_tip_send_result(Ok(()));
        assert_eq!(sent.code, PlatformWalletFFIResultCode::Success);
        free(sent);
    }
}
