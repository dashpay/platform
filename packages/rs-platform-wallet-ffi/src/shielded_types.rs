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

#[cfg(feature = "shielded")]
use platform_wallet::wallet::shielded::{IdentityDebitRecoveryRecord, IdentityDebitRecoveryStatus};

/// Availability of a local ledger, independent of its numeric balance.
/// Interpret this status only after the snapshot function returns Success.
/// The default/reset value is empty allocation storage, not a ledger verdict.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedLocalBalanceStatusFFI {
    #[default]
    Unbound = 0,
    RestoreIncomplete = 1,
    Ready = 2,
}

/// Evidence backing an account's local balance. NoHistory zero is not a
/// successfully scanned or restored zero.
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ShieldedBalanceSourceFFI {
    #[default]
    NoHistory = 0,
    Restored = 1,
    ScannedThisSession = 2,
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ShieldedLocalAccountBalanceFFI {
    pub account_index: u32,
    pub spendable_credits: u64,
    pub last_scanned_index: u64,
    /// Preserves the distinction between no watermark and an explicit zero.
    pub has_last_scanned_index: bool,
    pub source: ShieldedBalanceSourceFFI,
}

/// Owned snapshot. Free it exactly once with
/// `platform_wallet_manager_local_shielded_balance_snapshot_free`; that call
/// frees the flat account array and resets this value to its empty default.
/// After a non-Success result, the output is safe to free but its status and
/// numeric fields must not be interpreted as a successfully read ledger.
#[repr(C)]
#[derive(Debug)]
pub struct ShieldedLocalBalanceSnapshotFFI {
    pub status: ShieldedLocalBalanceStatusFFI,
    pub accounts: *const ShieldedLocalAccountBalanceFFI,
    pub accounts_count: usize,
}

impl Default for ShieldedLocalBalanceSnapshotFFI {
    fn default() -> Self {
        Self {
            status: ShieldedLocalBalanceStatusFFI::Unbound,
            accounts: std::ptr::null(),
            accounts_count: 0,
        }
    }
}

/// Cached Platform-to-shielded capacity for one payment account.
///
/// The Rust wallet planner computes every field from the same lexicographic
/// candidate set later used by the shield execution path, including the
/// versioned address-input cap. A normal no-capacity state is represented by
/// `can_shield == false`, not by an FFI error; the Success-coded result message
/// carries the optional explanation.
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

/// One durable identity-funded shield recovery record, scoped by the requested
/// wallet id plus `account_index` and `activity_id`. Optional values are absent
/// when the signed record cannot be read. The zero payload of an absent value
/// must not be interpreted as its actual value.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShieldedIdentityDebitRecoveryRecordFFI {
    pub account_index: u32,
    pub activity_id: [u8; 32],
    pub has_identity_id: bool,
    pub identity_id: [u8; 32],
    pub has_nonce: bool,
    pub nonce: u64,
    pub has_amount: bool,
    pub amount: u64,
    /// 0 Retrying, 1 Parked, 2 Unknown (explicitly abandoned).
    /// This is distinct from the activity-history status tag.
    pub status: u8,
}

#[cfg(feature = "shielded")]
impl From<IdentityDebitRecoveryRecord> for ShieldedIdentityDebitRecoveryRecordFFI {
    fn from(record: IdentityDebitRecoveryRecord) -> Self {
        Self {
            account_index: record.account_index,
            activity_id: record.activity_id,
            has_identity_id: record.identity_id.is_some(),
            identity_id: record.identity_id.unwrap_or_default(),
            has_nonce: record.nonce.is_some(),
            nonce: record.nonce.unwrap_or_default(),
            has_amount: record.amount.is_some(),
            amount: record.amount.unwrap_or_default(),
            status: match record.status {
                IdentityDebitRecoveryStatus::Retrying => 0,
                IdentityDebitRecoveryStatus::Parked => 1,
                IdentityDebitRecoveryStatus::Unknown => 2,
            },
        }
    }
}

#[cfg(all(test, feature = "shielded"))]
mod recovery_tests {
    use super::ShieldedIdentityDebitRecoveryRecordFFI;
    use platform_wallet::wallet::shielded::{
        IdentityDebitRecoveryRecord, IdentityDebitRecoveryStatus,
    };

    #[test]
    fn should_preserve_unreadable_record_scope_without_fabricating_fields() {
        let ffi = ShieldedIdentityDebitRecoveryRecordFFI::from(IdentityDebitRecoveryRecord {
            account_index: 9,
            activity_id: [7; 32],
            identity_id: None,
            nonce: None,
            amount: None,
            status: IdentityDebitRecoveryStatus::Parked,
        });
        assert_eq!(ffi.account_index, 9);
        assert_eq!(ffi.activity_id, [7; 32]);
        assert!(!ffi.has_identity_id);
        assert!(!ffi.has_nonce);
        assert!(!ffi.has_amount);
        assert_eq!(ffi.status, 1);
    }

    #[test]
    fn should_preserve_maximum_values_and_unknown_after_abandonment() {
        let ffi = ShieldedIdentityDebitRecoveryRecordFFI::from(IdentityDebitRecoveryRecord {
            account_index: u32::MAX,
            activity_id: [7; 32],
            identity_id: Some([8; 32]),
            nonce: Some(u64::MAX),
            amount: Some(u64::MAX),
            status: IdentityDebitRecoveryStatus::Unknown,
        });
        assert_eq!(ffi.account_index, u32::MAX);
        assert!(ffi.has_identity_id && ffi.has_nonce && ffi.has_amount);
        assert_eq!(ffi.identity_id, [8; 32]);
        assert_eq!(ffi.nonce, u64::MAX);
        assert_eq!(ffi.amount, u64::MAX);
        assert_eq!(ffi.status, 2);
    }
}
