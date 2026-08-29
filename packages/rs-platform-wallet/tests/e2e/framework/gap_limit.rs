//! Test-only batch fresh-unused-address derivation.
//!
//! Lives in the e2e harness (not in production) because the only
//! caller is PA-005b: production flows take one address at a time
//! through `PlatformAddressWallet::next_unused_receive_address`. This
//! module exposes:
//!
//! - [`next_unused_receive_addresses`] — lock-and-lookup wrapper
//!   around the wallet manager that reaches into the test wallet's
//!   default platform-payment account, derives `count` consecutive
//!   fresh addresses past `highest_generated`, and converts them to
//!   [`PlatformAddress`].
//! - [`derive_fresh_unused_addresses`] — the pure pool-level helper
//!   the wrapper delegates to. Exposed `pub(super)` for the unit
//!   tests that pin the gap-limit ceiling math without spinning a
//!   `WalletManager + Sdk` fixture.
//!
//! Both helpers reject `count` overflowing the pool's headroom with
//! [`GapLimitError::Exceeded`] and leave the pool untouched.
//!
//! ## Why this is test-only
//!
//! Marking `gap_limit` consecutive addresses fresh-past-watermark
//! drives `highest_generated` to `highest_used + gap_limit`, which
//! immediately starves the next single-address request unless the
//! caller marks one used. Production wallets don't want that
//! semantics — they hand out one address at a time and let funding
//! sync mark used. Keep it in the harness so a future test that wants
//! the batch-fresh shape can reach for it without bloating the
//! production surface.
//!
//! Mirrors the `next_unused_receive_addresses` accessor that briefly
//! lived on `PlatformAddressWallet` (commit `468e77472c`-style revert,
//! requested on PR #3609).

use dpp::address_funds::PlatformAddress;
use key_wallet::account::account_collection::PlatformPaymentAccountKey;
use platform_wallet::{PlatformWallet, PlatformWalletError};

/// Test-only error type for the gap-limit batch-derivation helpers.
///
/// The gap-limit ceiling rejection is a contract this harness pins
/// (PA-005b) but production wallets never construct: the production
/// receive-address API hands out one index at a time. Keeping the
/// variant inside the harness keeps `PlatformWalletError` free of
/// test-only shapes.
#[derive(Debug, thiserror::Error)]
pub enum GapLimitError {
    /// `count` would push the unused run past `highest_used + gap_limit`.
    /// The pool is left untouched.
    #[error(
        "gap-limit exceeded: requested {requested} fresh unused addresses but only \
         {available} are derivable past the current gap-limit boundary \
         (highest_used={highest_used:?}, highest_generated={highest_generated:?}, \
         gap_limit={gap_limit})"
    )]
    Exceeded {
        requested: usize,
        available: u32,
        highest_used: Option<u32>,
        highest_generated: Option<u32>,
        gap_limit: u32,
    },

    /// Underlying wallet-manager or pool-derivation failure. Boxed
    /// because `PlatformWalletError` is large (~2.6 KiB) and would
    /// otherwise dominate this enum's stack footprint.
    #[error(transparent)]
    Wallet(#[from] Box<PlatformWalletError>),
}

impl From<PlatformWalletError> for GapLimitError {
    fn from(err: PlatformWalletError) -> Self {
        GapLimitError::Wallet(Box::new(err))
    }
}

/// Derive `count` consecutive fresh-unused receive addresses on the
/// default platform-payment account, always extending past
/// `highest_generated`.
///
/// Unlike the production
/// [`PlatformAddressWallet::next_unused_receive_address`](platform_wallet::wallet::platform_addresses::PlatformAddressWallet::next_unused_receive_address)
/// (which hands out one index at a time and reserves it on hand-out),
/// this helper permanently advances the pool's `highest_generated`
/// watermark on every call, so consecutive invocations on the same
/// wallet yield non-overlapping ranges. This is the contract PA-005b
/// pins at the `gap_limit` boundary.
///
/// **Gap-limit interaction**: an `AddressPool` exposes `gap_limit`
/// unused addresses past the highest-used index (or `gap_limit` total
/// when nothing is used yet). If `count` would push the unused run
/// past that ceiling — i.e.
/// `(highest_generated + count) - highest_used > gap_limit` — the
/// call returns [`GapLimitError::Exceeded`] without mutating pool
/// state. Callers can mark an address used (e.g. by funding it) to
/// open more headroom and retry.
///
/// # Errors
///
/// - [`GapLimitError::Exceeded`] when `count` exceeds the pool's
///   current headroom.
/// - [`GapLimitError::Wallet`] wrapping [`PlatformWalletError::WalletNotFound`]
///   when the wallet id is missing from the manager, or
///   [`PlatformWalletError::AddressSync`] for any underlying
///   pool-level derivation or conversion failure.
pub async fn next_unused_receive_addresses(
    wallet: &std::sync::Arc<PlatformWallet>,
    account_key: PlatformPaymentAccountKey,
    count: usize,
) -> Result<Vec<PlatformAddress>, GapLimitError> {
    if count == 0 {
        return Ok(Vec::new());
    }

    let mut wm = wallet.wallet_manager().write().await;
    let wallet_id = wallet.wallet_id();
    let (managed_wallet, info) = wm.get_wallet_mut_and_info_mut(&wallet_id).ok_or_else(|| {
        PlatformWalletError::WalletNotFound(format!(
            "Wallet {:?} not found",
            hex::encode(wallet_id)
        ))
    })?;

    // TODO: reaches into the deprecated platform_payment_managed_account
    // pool for state the modern PlatformPaymentAddressProvider doesn't yet
    // expose (batch-derive helper). Migrate or retire once the pool moves
    // to the provider (per @QuantumExplorer's review on #3648).
    #[allow(deprecated)]
    let managed_account = info
        .core_wallet
        .platform_payment_managed_account_at_index_mut(account_key.account)
        .ok_or_else(|| {
            PlatformWalletError::AddressSync(format!(
                "No platform payment account at index {}",
                account_key.account
            ))
        })?;

    let key_source = {
        let xpub = managed_wallet
            .accounts
            .platform_payment_accounts
            .get(&account_key)
            .map(|acct| acct.account_xpub)
            .ok_or_else(|| {
                PlatformWalletError::AddressSync(format!(
                    "No platform payment account key for {:?}",
                    account_key
                ))
            })?;
        key_wallet::KeySource::Public(xpub)
    };

    let addresses =
        derive_fresh_unused_addresses(&mut managed_account.addresses, &key_source, count)?;

    addresses
        .into_iter()
        .map(|address| {
            PlatformAddress::try_from(address)
                .map_err(|e| {
                    PlatformWalletError::AddressSync(format!(
                        "Failed to convert to PlatformAddress: {e}"
                    ))
                })
                .map_err(GapLimitError::from)
        })
        .collect()
}

/// Derive `count` consecutive fresh-unused addresses from `pool`,
/// always extending past `highest_generated`. Pure pool-level helper
/// driven by [`next_unused_receive_addresses`] above.
///
/// Returns [`GapLimitError::Exceeded`] without mutating the pool when
/// `count` exceeds the current headroom. The caller is expected to
/// hold an exclusive (`&mut`) borrow of the pool.
pub(super) fn derive_fresh_unused_addresses(
    pool: &mut key_wallet::AddressPool,
    key_source: &key_wallet::KeySource,
    count: usize,
) -> Result<Vec<key_wallet::Address>, GapLimitError> {
    if count == 0 {
        return Ok(Vec::new());
    }

    // Headroom = (highest_used + gap_limit) - highest_generated, where
    // missing watermarks fall back to the empty-pool case (highest_used
    // absent ⇒ ceiling at gap_limit-1; highest_generated absent ⇒
    // start at index 0). All arithmetic stays in u32: gap_limit is u32
    // and the watermarks are u32.
    let gap_limit = pool.gap_limit;
    let ceiling: u32 = match pool.highest_used {
        None => gap_limit.saturating_sub(1),
        Some(highest) => highest.saturating_add(gap_limit),
    };
    let next_index: u32 = pool
        .highest_generated
        .map(|h| h.saturating_add(1))
        .unwrap_or(0);
    let available: u32 = ceiling.saturating_sub(next_index).saturating_add(1);
    let count_u32 = u32::try_from(count).unwrap_or(u32::MAX);
    if count_u32 > available {
        return Err(GapLimitError::Exceeded {
            requested: count,
            available,
            highest_used: pool.highest_used,
            highest_generated: pool.highest_generated,
            gap_limit,
        });
    }

    pool.generate_addresses(count_u32, key_source, true)
        .map_err(|e| GapLimitError::from(PlatformWalletError::AddressSync(e.to_string())))
}

#[cfg(test)]
mod tests {
    //! Pool-level unit tests for [`derive_fresh_unused_addresses`].
    //! Driving the wallet entry point directly requires a full
    //! `WalletManager + Sdk` fixture, exercised by PA-005b's three
    //! sub-cases. The helper itself is the meaningful contract — the
    //! wallet wrapper is a thin lock-and-lookup pass-through.

    use super::{derive_fresh_unused_addresses, GapLimitError};
    use key_wallet::bip32::{ChildNumber, DerivationPath, ExtendedPrivKey};
    use key_wallet::dashcore::secp256k1::Secp256k1;
    use key_wallet::managed_account::address_pool::{AddressPool, AddressPoolType};
    use key_wallet::mnemonic::{Language, Mnemonic};
    use key_wallet::{KeySource, Network};

    fn test_key_source() -> KeySource {
        let mnemonic = Mnemonic::from_phrase(
            "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            Language::English,
        )
        .expect("mnemonic parses");
        let seed = mnemonic.to_seed("");
        let master = ExtendedPrivKey::new_master(Network::Testnet, &seed).expect("master xprv");
        let secp = Secp256k1::new();
        let path = DerivationPath::from(vec![
            ChildNumber::from_hardened_idx(44).unwrap(),
            ChildNumber::from_hardened_idx(1).unwrap(),
            ChildNumber::from_hardened_idx(0).unwrap(),
        ]);
        let account_key = master
            .derive_priv(&secp, &path)
            .expect("account derivation");
        KeySource::Private(account_key)
    }

    fn empty_pool(gap_limit: u32) -> AddressPool {
        let base_path = DerivationPath::from(vec![ChildNumber::from_normal_idx(0).unwrap()]);
        AddressPool::new_without_generation(
            base_path,
            AddressPoolType::External,
            gap_limit,
            Network::Testnet,
        )
    }

    #[test]
    fn returns_count_addresses_all_distinct() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let addrs = derive_fresh_unused_addresses(&mut pool, &key_source, 19)
            .expect("19 ≤ gap_limit, must succeed");
        assert_eq!(addrs.len(), 19);
        let unique: std::collections::HashSet<_> = addrs.iter().collect();
        assert_eq!(unique.len(), 19, "all 19 addresses must be distinct");
        assert_eq!(pool.highest_generated, Some(18));
    }

    #[test]
    fn consecutive_calls_yield_non_overlapping_ranges() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let first = derive_fresh_unused_addresses(&mut pool, &key_source, 5)
            .expect("first batch fits in gap_limit");
        // After 5 generated and none used, headroom is 20 - 5 = 15;
        // request another 5 to lock the non-overlap contract.
        let second = derive_fresh_unused_addresses(&mut pool, &key_source, 5)
            .expect("second batch fits in remaining headroom");
        assert_eq!(first.len(), 5);
        assert_eq!(second.len(), 5);
        let intersection: std::collections::HashSet<_> = first.iter().collect();
        assert!(
            second.iter().all(|a| !intersection.contains(a)),
            "consecutive calls must not return any overlapping address"
        );
        assert_eq!(pool.highest_generated, Some(9));
    }

    #[test]
    fn does_not_exceed_gap_limit_cap() {
        let gap_limit = 20;
        let mut pool = empty_pool(gap_limit);
        let key_source = test_key_source();
        // No used indices ⇒ ceiling at index gap_limit-1=19, headroom = gap_limit = 20.
        // Requesting 21 must error rather than over-extend.
        let err = derive_fresh_unused_addresses(&mut pool, &key_source, 21).unwrap_err();
        match err {
            GapLimitError::Exceeded {
                requested,
                available,
                gap_limit: gl,
                ..
            } => {
                assert_eq!(requested, 21);
                assert_eq!(available, 20);
                assert_eq!(gl, gap_limit);
            }
            other => panic!("expected GapLimitError::Exceeded, got {:?}", other),
        }
        // Pool must remain untouched after a rejected request.
        assert_eq!(pool.highest_generated, None);
    }

    #[test]
    fn count_zero_is_no_op() {
        let mut pool = empty_pool(20);
        let key_source = test_key_source();
        let addrs = derive_fresh_unused_addresses(&mut pool, &key_source, 0)
            .expect("count = 0 is a no-op success");
        assert!(addrs.is_empty());
        assert_eq!(pool.highest_generated, None);
    }

    #[test]
    fn marking_used_extends_headroom() {
        // Once an index is marked used, the gap-limit ceiling shifts
        // up by `gap_limit`, so a subsequent request that would have
        // exceeded the original cap can succeed.
        let gap_limit = 20;
        let mut pool = empty_pool(gap_limit);
        let key_source = test_key_source();
        let first = derive_fresh_unused_addresses(&mut pool, &key_source, gap_limit as usize)
            .expect("first batch fits exactly in initial gap_limit window");
        assert_eq!(first.len(), gap_limit as usize);
        // Mark the lowest one used to advance highest_used to 0; new
        // ceiling = 0 + gap_limit = 20, but highest_generated is 19,
        // so headroom = 1 fresh address.
        pool.mark_used(&first[0]);
        let second =
            derive_fresh_unused_addresses(&mut pool, &key_source, 1).expect("one more fits");
        assert_eq!(second.len(), 1);
        assert!(!first.contains(&second[0]));
    }
}
