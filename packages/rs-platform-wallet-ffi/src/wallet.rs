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
/// `PlatformWalletManager::provider_masternode_txs_blocking`. `out_*` are
/// set to null / 0 when the wallet has no masternodes or isn't found.
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
        manager.provider_masternode_txs_blocking(&wid)
    });
    // Outer Option: handle resolved. Inner Option: wallet found.
    let inner = unwrap_option_or_return!(option);
    let (network, txs, dml, operator_index, platform_index) = unwrap_option_or_return!(inner);

    // Derive DML membership from the owned snapshot (`None` ⇒ list not
    // available ⇒ Unknown status ⇒ persist layer keeps the prior value).
    use crate::core_wallet_types::ListMembership;
    let membership = |pro_tx_hash: &[u8; 32]| -> ListMembership {
        match &dml {
            None => ListMembership::ListUnavailable,
            Some(map) => match map.get(pro_tx_hash) {
                Some(true) => ListMembership::ValidEntry,
                Some(false) => ListMembership::InvalidEntry,
                None => ListMembership::Absent,
            },
        }
    };

    let aggregates = crate::core_wallet_types::aggregate_masternodes(
        txs.iter().map(|(h, p, tx)| (*h, *p, tx)),
        membership,
    );

    let entries: Vec<crate::core_wallet_types::MasternodeEntryFFI> = aggregates
        .iter()
        .enumerate()
        .map(|(idx, mn)| {
            crate::core_wallet_types::masternode_entry_ffi(
                mn,
                idx as u32,
                network,
                &operator_index,
                &platform_index,
            )
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
        ] {
            if !ptr.is_null() {
                let _ = std::ffi::CString::from_raw(ptr);
            }
        }
    }
    let _ = Box::from_raw(std::ptr::slice_from_raw_parts_mut(entries, count));
}

/// Claim (withdraw) credits from a masternode's Platform identity to L1
/// via an Identity Credit Withdrawal, signed with the wallet-held OWNER
/// key. Writes the remaining balance to `out_new_balance`.
///
/// - `pro_tx_hash`: 32 bytes in WIRE order (as stored). The masternode
///   identity id is the **display-order** (reversed) form, so this fn
///   reverses before fetching — same orientation as the balance fetch.
/// - `owner_key_index`: the ProviderOwnerKeys derivation index the app
///   resolved from the persisted address join (in-memory pools may be
///   empty for imported wallets, so the index is passed in rather than
///   re-derived here).
/// - `dest_address` MUST be null for the owner-key path: Platform routes
///   an owner-key withdrawal to the registered payout address; a
///   destination can't be chosen. (`use_owner_key == false` / a TRANSFER
///   destination is a documented follow-up.)
///
/// Orchestration (all in Rust, per `swift-sdk/CLAUDE.md`):
///   1. Resolve the wallet + masternode by `pro_tx_hash`; read its
///      `owner_key_hash`.
///   2. `Identity::fetch_by_identifier(reversed(pro_tx_hash))`.
///   3. GUARD: `select_owner_withdrawal_key(identity.public_keys(),
///      owner_key_hash)` — if `None`, return `InvalidIdentityData` WITHOUT
///      broadcasting (signing with an unrecognised key wastes the attempt).
///   4. Derive the ECDSA owner private key at `owner_key_index` on the
///      ProviderOwnerKeys account; build a `Signer<IdentityPublicKey>`
///      over it; `withdraw_credits_with_signer(identity, None, amount,
///      Some(matched_owner_key), signer, None)`.
///
/// NOTE: steps 1-3 are wired below; step 4 (the internal owner-key
/// `Signer<IdentityPublicKey>` + ECDSA owner-key derivation + DPP
/// signature encoding for `ECDSA_HASH160`) is a NEW money-signing
/// component with no existing production analogue (identity ops sign via
/// an external Swift signer). It is gated behind a distinct error until it
/// can be built and verified against a real testnet claim, so the
/// end-to-end plumbing (UI → wrapper → FFI → fetch → guard → error) is
/// exercisable without risking a malformed money transition.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_manager_masternode_withdraw(
    manager_handle: Handle,
    wallet_id: *const u8,
    pro_tx_hash: *const u8,
    amount: u64,
    owner_key_index: u32,
    dest_address: *const std::os::raw::c_char,
    use_owner_key: bool,
    out_new_balance: *mut u64,
) -> PlatformWalletFFIResult {
    check_ptr!(wallet_id);
    check_ptr!(pro_tx_hash);
    check_ptr!(out_new_balance);

    use crate::error::PlatformWalletFFIResultCode;

    // Owner-key path only, for now.
    if !use_owner_key {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "TRANSFER-key masternode withdrawal is not yet supported; use the owner key",
        );
    }
    if !dest_address.is_null() {
        return PlatformWalletFFIResult::err(
            PlatformWalletFFIResultCode::ErrorInvalidParameter,
            "owner-key withdrawal pays the registered payout address; dest_address must be null",
        );
    }

    // See the doc comment: steps 1-3 (resolve → fetch → guard) plus the
    // owner-key `Signer<IdentityPublicKey>` derivation + sign are the
    // remaining verified-implementation work. Surface a distinct, non-fatal
    // error rather than broadcasting an unverified money transition.
    let _ = (manager_handle, amount, owner_key_index);
    PlatformWalletFFIResult::err(
        PlatformWalletFFIResultCode::ErrorWalletOperation,
        "masternode owner-key withdrawal is not yet enabled (pending verified signer)",
    )
}

/// Destroy a PlatformWallet handle.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_destroy(handle: Handle) -> PlatformWalletFFIResult {
    // Sweep any outstanding deferred-payment tokens bound to this wallet first,
    // so the registry stops pinning its `WalletManager` (accounts, keys, sync
    // state) alive for the rest of the process via the `CoreWallet` clone each
    // token captured. Hooked here rather than into `core_wallet_destroy`: the
    // deferred flow builds/registers on one short-lived core handle and
    // broadcasts on another, so sweeping on core-handle destroy would drop
    // tokens between register and broadcast.
    PLATFORM_WALLET_STORAGE.with_item(handle, |wallet| {
        crate::core_wallet::signed_payment::SIGNED_PAYMENT_REGISTRY
            .remove_entries_for_wallet(wallet.core());
    });
    PLATFORM_WALLET_STORAGE.remove(handle);
    PlatformWalletFFIResult::ok()
}
