//! C-ABI types for the shielded sync surface.
//!
//! Defined unconditionally — without the `shielded` Cargo feature
//! the `shielded_sync` module is omitted, but
//! [`EventHandlerCallbacks`](crate::event_handler::EventHandlerCallbacks)
//! still has to carry the `on_shielded_sync_completed_fn` slot so
//! the C struct layout doesn't drift between feature configurations.
//! The types here have no functional dependency on the shielded code
//! path; they're just `#[repr(C)]` data carriers.

use std::os::raw::c_char;

/// Cached Platform-to-shielded capacity for one payment account.
///
/// The Rust wallet planner computes every field from the same lexicographic
/// candidate suffix later used by the shield execution path. A normal
/// no-capacity state is represented by `can_shield == false`, not by an FFI
/// error; the Success-coded result message carries the optional explanation.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ShieldedShieldPreflightFFI {
    pub can_shield: bool,
    pub account_balance_credits: u64,
    pub usable_balance_credits: u64,
    pub fee_reserve_credits: u64,
    pub max_shieldable_credits: u64,
}

/// Per-wallet outcome from a completed shielded sync pass.
///
/// Mirrors
/// [`PlatformAddressSyncWalletResultFFI`](crate::platform_address_sync::PlatformAddressSyncWalletResultFFI)
/// for the shielded path. The status fields encode three states:
///
/// - `success == true`: sync succeeded; the numeric fields are
///   meaningful and `error_message` is NULL.
/// - `skipped == true`: the wallet had no bound shielded sub-wallet
///   so the pass passed it over; `success` is false and
///   `error_message` is NULL.
/// - both flags `false` and `error_message != NULL`: the sync
///   itself failed.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShieldedSyncWalletResultFFI {
    pub wallet_id: [u8; 32],
    /// `true` only on a successful sync.
    pub success: bool,
    /// `true` if the wallet had no bound shielded sub-wallet (so the
    /// pass simply skipped it). Mutually exclusive with `success`.
    pub skipped: bool,
    /// `true` when `success` is true but the pass was short-circuited
    /// by the caught-up cooldown — no SDK fetch / trial-decrypt /
    /// nullifier scan / balance read ran. When this flag is set,
    /// every numeric field on this struct is zero / default — the
    /// host should preserve its prior cached balance and counters
    /// rather than apply the payload. `false` for every pass that
    /// actually walked Platform.
    pub cooldown_skip: bool,
    /// New decrypted notes detected this pass.
    pub new_notes: u32,
    /// Total encrypted notes scanned (decrypted + skipped).
    pub total_scanned: u64,
    /// Notes newly detected as spent this pass.
    pub newly_spent: u32,
    /// Current unspent shielded balance after the pass.
    pub balance: u64,
    /// NUL-terminated UTF-8 error message; valid until the callback
    /// returns. NULL on success and skipped wallets.
    pub error_message: *const c_char,
}

impl Default for ShieldedSyncWalletResultFFI {
    fn default() -> Self {
        Self {
            wallet_id: [0; 32],
            success: false,
            skipped: false,
            cooldown_skip: false,
            new_notes: 0,
            total_scanned: 0,
            newly_spent: 0,
            balance: 0,
            error_message: std::ptr::null(),
        }
    }
}

// Constructors only used by the feature-gated event-handler dispatch.
// Annotated rather than feature-gated at the impl level so the type
// can stay unconditional but the helpers don't generate dead-code
// warnings on no-shielded builds.
#[cfg(feature = "shielded")]
impl ShieldedSyncWalletResultFFI {
    pub(crate) fn skipped(wallet_id: [u8; 32]) -> Self {
        Self {
            wallet_id,
            skipped: true,
            ..Self::default()
        }
    }

    pub(crate) fn err(wallet_id: [u8; 32], error_ptr: *const c_char) -> Self {
        Self {
            wallet_id,
            error_message: error_ptr,
            ..Self::default()
        }
    }
}
