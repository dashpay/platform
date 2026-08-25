//! FFI bindings for PlatformWallet (sub-wallet access, balance, persistence).

use crate::error::*;
use crate::handle::*;
use crate::runtime::runtime;
use crate::{check_ptr, unwrap_option_or_return, unwrap_result_or_return};

/// Get the wallet ID (32 bytes).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_id(
    handle: Handle,
    out_wallet_id: *mut [u8; 32],
) -> PlatformWalletFFIResult {
    check_ptr!(out_wallet_id);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.wallet_id());
    *out_wallet_id = unwrap_option_or_return!(option);
    PlatformWalletFFIResult::ok()
}

/// Get lock-free balance (spendable, unconfirmed, immature, locked).
///
/// These are atomic reads — no lock contention.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_balance(
    handle: Handle,
    out_confirmed: *mut u64,
    out_unconfirmed: *mut u64,
    out_immature: *mut u64,
    out_locked: *mut u64,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        let b = wallet.balance();
        (b.confirmed(), b.unconfirmed(), b.immature(), b.locked())
    });
    let (confirmed, unconfirmed, immature, locked) = unwrap_option_or_return!(option);
    if !out_confirmed.is_null() {
        *out_confirmed = confirmed;
    }
    if !out_unconfirmed.is_null() {
        *out_unconfirmed = unconfirmed;
    }
    if !out_immature.is_null() {
        *out_immature = immature;
    }
    if !out_locked.is_null() {
        *out_locked = locked;
    }
    PlatformWalletFFIResult::ok()
}

/// Get a PlatformAddressWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_platform(
    handle: Handle,
    out_platform_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_platform_handle);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.platform().clone());
    let platform_wallet = unwrap_option_or_return!(option);
    *out_platform_handle = PLATFORM_ADDRESS_WALLET_STORAGE.insert(platform_wallet);
    PlatformWalletFFIResult::ok()
}

/// Get an AssetLockManager handle from a PlatformWallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_asset_locks(
    handle: Handle,
    out_asset_lock_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_asset_lock_handle);

    let option = PLATFORM_WALLET_STORAGE
        .with_item(handle, |wallet| std::sync::Arc::clone(wallet.asset_locks()));
    let asset_locks = unwrap_option_or_return!(option);
    *out_asset_lock_handle = ASSET_LOCK_MANAGER_STORAGE.insert(asset_locks);
    PlatformWalletFFIResult::ok()
}

/// Get a CoreWallet handle from a PlatformWallet.
///
/// The returned handle is a clone (cheap — all Arc internals).
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_get_core(
    handle: Handle,
    out_core_handle: *mut Handle,
) -> PlatformWalletFFIResult {
    check_ptr!(out_core_handle);

    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.core().clone());
    let core_wallet = unwrap_option_or_return!(option);
    *out_core_handle = CORE_WALLET_STORAGE.insert(core_wallet);
    PlatformWalletFFIResult::ok()
}

/// Flush all queued changesets to the storage backend.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_flush_persist(handle: Handle) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| wallet.flush_persist());
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Load persisted state and apply it to the in-memory wallet.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_load_and_apply_persisted(
    handle: Handle,
) -> PlatformWalletFFIResult {
    let option = PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        runtime().block_on(wallet.load_and_apply_persisted())
    });
    let result = unwrap_option_or_return!(option);
    unwrap_result_or_return!(result);
    PlatformWalletFFIResult::ok()
}

/// Query per-account balances from the in-memory `WalletManager`.
///
/// Returns an array of [`AccountBalanceEntryFFI`] — one per account
/// in the wallet's `ManagedAccountCollection`. The caller owns the
/// returned array and must free it via
/// [`platform_wallet_manager_free_account_balances`].
///
/// `out_entries` receives a pointer to the heap-allocated array;
/// `out_count` receives the element count.  Both are set to
/// null / 0 when the wallet is not found.
///
/// Reads the wallet manager lock via `blocking_read` — must not be
/// called from within a tokio async context.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_get_account_balances(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_entries: *mut *const crate::core_wallet_types::AccountBalanceEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_entries);
    check_ptr!(out_count);

    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.account_balances_blocking(&wid)
    });
    let balances = unwrap_option_or_return!(option);

    let entries: Vec<crate::core_wallet_types::AccountBalanceEntryFFI> = balances
        .into_iter()
        .map(|row| {
            let tags = crate::core_wallet_types::account_type_to_tags(&row.account_type);
            crate::core_wallet_types::AccountBalanceEntryFFI {
                type_tag: tags.type_tag,
                standard_tag: tags.standard_tag,
                index: tags.index,
                registration_index: tags.registration_index,
                key_class: tags.key_class,
                user_identity_id: tags.user_identity_id,
                friend_identity_id: tags.friend_identity_id,
                confirmed: row.balance.confirmed(),
                unconfirmed: row.balance.unconfirmed(),
                immature: row.balance.immature(),
                locked: row.balance.locked(),
                keys_used: row.keys_used,
                keys_total: row.keys_total,
            }
        })
        .collect();
    let count = entries.len();

    if count == 0 {
        *out_entries = std::ptr::null();
        *out_count = 0;
        return PlatformWalletFFIResult::ok();
    }

    let boxed = entries.into_boxed_slice();
    *out_entries = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by [`platform_wallet_manager_get_account_balances`].
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_account_balances(
    entries: *mut crate::core_wallet_types::AccountBalanceEntryFFI,
    count: usize,
) {
    if !entries.is_null() && count > 0 {
        let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
    }
}

/// Aggregate the wallet's masternodes from its retained provider special
/// transactions (ProRegTx / ProUpServTx / ProUpRegTx / ProUpRevTx),
/// grouped by proTxHash. Returns an array of
/// [`MasternodeEntryFFI`](crate::core_wallet_types::MasternodeEntryFFI),
/// one per masternode, sorted by registration order. The caller owns the
/// array and must free it via
/// [`platform_wallet_manager_free_masternodes`].
///
/// The record source (rust-dashcore #876 provider-payload retention) is
/// populated in every feature configuration; see
/// `PlatformWalletManager::wallet_masternodes_blocking`, which also resolves
/// status and operator / platform key ownership — this function only
/// marshals. `out_*` are set to null / 0 when the wallet has no masternodes
/// or isn't found.
///
/// Reads the wallet manager lock via `blocking_read` — must not be called
/// from within a tokio async context.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_list_masternodes(
    manager_handle: Handle,
    wallet_id: *const u8,
    out_entries: *mut *const crate::core_wallet_types::MasternodeEntryFFI,
    out_count: *mut usize,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(out_entries);
    check_ptr!(out_count);
    // Initialise outputs immediately so the invalid-handle / unknown-wallet
    // early returns below leave the caller looking at valid empty state
    // rather than uninitialised / stale pointers.
    *out_entries = std::ptr::null();
    *out_count = 0;

    let wid: [u8; 32] = std::ptr::read(wallet_id as *const [u8; 32]);

    let option = PLATFORM_WALLET_MANAGER_STORAGE.with_item(manager_handle, |manager| {
        manager.wallet_masternodes_blocking(&wid)
    });
    // Outer Option: handle resolved. Inner Option: wallet found.
    let inner = unwrap_option_or_return!(option);
    let masternodes = unwrap_option_or_return!(inner);

    let entries: Vec<crate::core_wallet_types::MasternodeEntryFFI> = masternodes
        .records
        .iter()
        .map(|mn| crate::core_wallet_types::masternode_entry_ffi(mn, masternodes.network))
        .collect();
    let count = entries.len();

    if count == 0 {
        *out_entries = std::ptr::null();
        *out_count = 0;
        return PlatformWalletFFIResult::ok();
    }

    let boxed = entries.into_boxed_slice();
    *out_entries = Box::into_raw(boxed) as *const _;
    *out_count = count;
    PlatformWalletFFIResult::ok()
}

/// Free an array returned by [`platform_wallet_manager_list_masternodes`],
/// including each entry's heap C strings.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_free_masternodes(
    entries: *mut crate::core_wallet_types::MasternodeEntryFFI,
    count: usize,
) {
    if entries.is_null() || count == 0 {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(entries, count);
    for entry in slice.iter() {
        for ptr in [
            entry.service_address,
            entry.owner_address,
            entry.voting_address,
            entry.payout_address,
            entry.operator_pseudo_address,
            entry.platform_node_address,
            entry.label,
        ] {
            if !ptr.is_null() {
                let _ = std::ffi::CString::from_raw(ptr);
            }
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
}

/// Destroy a PlatformWallet handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_destroy(handle: Handle) -> PlatformWalletFFIResult {
    // Destroying a wrapper alias must NOT touch the deferred-payment registry.
    //
    // `platform_wallet_manager_get_wallet` hands out an independent handle for
    // each alias of a wallet *generation*, but none of those wrappers OWN the
    // logical wallet — the manager still owns it and can hand out another alias,
    // `platform_wallet_get_core` yields independently-owned core handles, and
    // each registry entry pins its own `CoreWallet` (keeping the reservation
    // live). A registered deferred-payment token is owned by the payment flow
    // that minted it, NOT by any wrapper handle, so closing or garbage-collecting
    // the last wrapper must leave the token intact: a later merchant ack has to
    // remain broadcastable through a retained core handle or a re-acquired alias.
    //
    // Token cleanup therefore follows the payment owner (an explicit
    // broadcast/release) or actual wallet-generation removal
    // (`platform_wallet_manager_remove_wallet`, which drops the entries because
    // the reservation ceases to exist with the generation) — never a transient
    // wrapper-alias count. Dropping this handle just releases its `Arc`s; the
    // registry entry's own `CoreWallet` clone keeps the generation alive as long
    // as a token references it. (`dashpay/platform#4185`, blocker 2.)
    let _ = PLATFORM_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}

#[cfg(test)]
mod destroy_tests {
    use super::*;
    use crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY;
    use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
    use platform_wallet::test_support::test_platform_wallet_manager;
    use platform_wallet::SignedCoreTransaction;

    fn dummy_tx() -> dashcore::Transaction {
        dashcore::Transaction {
            version: 3,
            lock_time: 0,
            input: vec![],
            output: vec![],
            special_transaction_payload: None,
        }
    }

    /// Destroying wrapper alias handles must NEVER invalidate a deferred-payment
    /// token — not even when the FINAL alias is destroyed. A wrapper handle does
    /// not own the logical wallet or the registered payment; the token is owned
    /// by the payment flow that minted it and stays live and actionable until its
    /// owner broadcasts/releases it or the generation is actually removed
    /// (`platform_wallet_manager_remove_wallet`). Regression for
    /// `dashpay/platform#4185` blocker 2: the old final-alias sweep consumed
    /// independently-owned payments.
    #[test]
    fn destroying_wrapper_aliases_never_sweeps_tokens() {
        // Asserts `outstanding()` DELTAS against a captured baseline, so it must
        // not run while a sibling test mints or consumes tokens in the same
        // process-global registry.
        let _registry = crate::core_wallet::signed_payment::registry_test_guard();

        // Async setup only. `platform_wallet_destroy` and the final `release`
        // each do their own `runtime().block_on(...)`, exactly as the JNI /
        // NativeCleaner threads do (never from inside a tokio runtime). Calling
        // them from within an outer `block_on` would nest runtimes and abort, so
        // they run on the plain test thread below.
        let (manager, handle_a, handle_b, token, baseline) = runtime().block_on(async {
            let (manager, wallet_id) = test_platform_wallet_manager().await;

            // Two independent handles for the SAME logical wallet, exactly as two
            // `platform_wallet_manager_get_wallet` calls would hand out.
            let alias_a = manager.get_wallet(&wallet_id).await.expect("alias a");
            let alias_b = manager.get_wallet(&wallet_id).await.expect("alias b");
            let core = alias_a.core().clone();
            let handle_a = PLATFORM_WALLET_STORAGE.insert(alias_a);
            let handle_b = PLATFORM_WALLET_STORAGE.insert(alias_b);

            // Register a deferred-payment token (the process-global registry is
            // shared, so reason about deltas against a captured baseline). The
            // dummy tx reserved nothing (reservation height 0, no funding token) —
            // this test exercises destroy/ownership, not the age or owner guard.
            let baseline = SIGNED_PAYMENT_REGISTRY.outstanding();
            let token = SIGNED_PAYMENT_REGISTRY
                .register(
                    core.clone(),
                    SignedCoreTransaction::new_for_test(
                        dummy_tx(),
                        0,
                        vec![AccountTypePreference::BIP44
                            .account_type(0)
                            .expect("single account")],
                        0,
                        None,
                        // Bind the finalized payment to this exact wallet
                        // generation so `register` accepts it (it now validates
                        // the wallet against the finalizing generation).
                        core.test_generation_marker(),
                    ),
                )
                .expect("register with the same generation");
            assert_eq!(SIGNED_PAYMENT_REGISTRY.outstanding(), baseline + 1);
            (manager, handle_a, handle_b, token, baseline)
        });

        // Destroy alias A while B is still live → token must survive.
        let result = unsafe { platform_wallet_destroy(handle_a) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(
            SIGNED_PAYMENT_REGISTRY.outstanding(),
            baseline + 1,
            "a sibling alias's token must survive destroying another alias"
        );

        // Destroy the FINAL alias B → the token STILL survives: a wrapper alias
        // does not own the payment, so its destruction must not consume the token.
        let result = unsafe { platform_wallet_destroy(handle_b) };
        assert_eq!(result.code, PlatformWalletFFIResultCode::Success);
        assert_eq!(
            SIGNED_PAYMENT_REGISTRY.outstanding(),
            baseline + 1,
            "destroying the final wrapper alias must NOT sweep an independently-owned token"
        );

        // The token is still fully live: its owner can release it even after both
        // wrappers are gone (the registry entry pinned its own `CoreWallet`).
        runtime().block_on(SIGNED_PAYMENT_REGISTRY.release(token));
        assert_eq!(
            SIGNED_PAYMENT_REGISTRY.outstanding(),
            baseline,
            "the payment owner can still release the surviving token"
        );

        // Keep the manager alive until the end (owns the wallet + adapter).
        drop(manager);
    }
}
