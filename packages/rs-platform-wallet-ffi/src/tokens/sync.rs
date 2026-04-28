//! FFI binding for [`TokenWallet::watch`] + [`TokenWallet::sync`].
//!
//! Watching and syncing are wallet-scope bookkeeping (no signer, no
//! state transition) — the call just registers `(identity_id,
//! token_id)` pairs in the in-memory watch registry and then queries
//! Platform per identity for the matching balances. The resulting
//! `TokenBalanceChangeSet` flows through the persister, surfacing as
//! `on_persist_token_balances_fn` on the Swift side.
//!
//! This is the single entry point Swift needs to populate
//! `PersistentTokenBalance` rows: it ships in the `(identity, token)`
//! pairs the UI cares about, Rust does the watch + Platform fetch +
//! changeset emission, and the persister callback writes them to
//! SwiftData.

use std::slice;

use crate::error::*;
use crate::handle::*;
use crate::runtime::block_on_worker;
use crate::token_persistence::TokenBalanceUpsertFFI;

/// Watch every `(identity_id, token_id)` pair in `pairs`, then run a
/// single Platform sync round to refresh the cached balances.
///
/// The persister callback (`on_persist_token_balances_fn`) fires once
/// the sync round completes with the resulting upsert / removal lists.
///
/// `pairs` reuses [`TokenBalanceUpsertFFI`] for its layout — the
/// 32-byte identity id + 32-byte token id — and ignores `balance`. We
/// reuse the type rather than introducing a near-identical
/// `TokenBalancePairFFI` so the Swift side can share the same struct
/// for input + persist callbacks. Pass `pairs_count = 0` to skip the
/// watch step (sync alone).
///
/// # Safety
/// - `wallet_handle` must come from the platform-wallet handle registry.
/// - `pairs` must be either NULL or point at exactly `pairs_count`
///   readable [`TokenBalanceUpsertFFI`] entries.
/// - `out_error` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn platform_wallet_token_watch_and_sync(
    wallet_handle: Handle,
    pairs: *const TokenBalanceUpsertFFI,
    pairs_count: usize,
    out_error: *mut PlatformWalletFFIError,
) -> PlatformWalletFFIResult {
    let pair_slice: &[TokenBalanceUpsertFFI] = if pairs.is_null() || pairs_count == 0 {
        &[]
    } else {
        unsafe { slice::from_raw_parts(pairs, pairs_count) }
    };

    // Materialize the watch list now while we hold the &[u8; 32] view
    // — the async block below can't borrow from this stack frame.
    let watch_pairs: Vec<(dpp::prelude::Identifier, dpp::prelude::Identifier)> = pair_slice
        .iter()
        .map(|p| {
            (
                dpp::prelude::Identifier::from(p.identity_id),
                dpp::prelude::Identifier::from(p.token_id),
            )
        })
        .collect();

    PLATFORM_WALLET_STORAGE
        .with_item(wallet_handle, |wallet| {
            let token_wallet = wallet.tokens().clone();
            let result = block_on_worker(async move {
                for (identity_id, token_id) in &watch_pairs {
                    token_wallet.watch(*identity_id, *token_id).await;
                }
                token_wallet.sync().await
            });
            match result {
                Ok(_) => PlatformWalletFFIResult::Success,
                Err(e) => {
                    if !out_error.is_null() {
                        *out_error = PlatformWalletFFIError::new(
                            PlatformWalletFFIResult::ErrorWalletOperation,
                            format!("token_watch_and_sync failed: {e}"),
                        );
                    }
                    PlatformWalletFFIResult::ErrorWalletOperation
                }
            }
        })
        .unwrap_or_else(|| {
            if !out_error.is_null() {
                *out_error = PlatformWalletFFIError::new(
                    PlatformWalletFFIResult::ErrorInvalidHandle,
                    "Invalid platform-wallet handle",
                );
            }
            PlatformWalletFFIResult::ErrorInvalidHandle
        })
}
