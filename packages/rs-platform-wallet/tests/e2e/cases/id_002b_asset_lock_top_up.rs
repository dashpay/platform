//! ID-002b — Asset-lock-funded top-up of existing identity.
//!
//! **Found-031 reachability proof**: step 3's precondition
//! (`add_identity_topup_account`) provisions the `IdentityTopUp` account on a
//! post-registration wallet. `register_wallet` calls
//! `downgrade_to_external_signable()` (`wallet_lifecycle.rs:244`), dropping the
//! root seed, so the resident-key `add_account(.., None)` path fails with
//! "External signable wallet has no private key". The account is instead
//! provisioned with an externally-derived `Some(xpub)` — watch-only, signed at
//! build/consume time by the external `core_signer` — proving the asset-lock
//! top-up is reachable on an external-signable wallet. See TEST_SPEC.md Found-031.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-002b).
//! Pinned status: ACTIVE — full test body, gated behind the `e2e` cargo feature
//! behind the `PLATFORM_WALLET_E2E_BANK_CORE_GATE` env var (same gate
//! CR-003 uses; default-on, 180 s deadline). Bank Core (Layer-1)
//! pre-funding required.
//!
//! Mirrors `CR-003` (asset-lock-funded registration) but drives the
//! sibling top-up path: register an identity via the cheaper
//! address-funded path (ID-001 helper), then top-up that identity
//! via `top_up_identity_with_funding(.., FundWithWallet, ..)` so the
//! asset-lock manager builds + broadcasts + waits on a Core asset
//! lock and the top-up state transition credits the identity.
//!
//! Pins the asset-lock-funded top-up contract:
//! 1. `setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)`
//!    lands `TEST_WALLET_CORE_FUNDING` duffs on the test wallet's
//!    BIP-44 account 0 (visible to SPV).
//! 2. Register an identity via `register_identity_from_addresses`
//!    (ID-001 helper) — cheaper than the asset-lock registration
//!    path for this test's needs.
//! 3. `IdentityWallet::top_up_identity_with_funding` with
//!    `AssetLockFunding::FromWalletBalance { amount_duffs: TOP_UP_ASSET_LOCK_AMOUNT, account_index: 0 }`
//!    drives the unified asset-lock flow internally —
//!    `AssetLockManager::create_funded_asset_lock_proof` (build →
//!    broadcast → wait IS / fall back to ChainLock) and submits an
//!    `IdentityTopUp` state transition against the resolved proof.
//! 4. The identity's on-chain balance increases by approximately
//!    `TOP_UP_ASSET_LOCK_AMOUNT * CREDITS_PER_DUFF` minus the
//!    (positive) top-up fee.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dashcore::secp256k1::Secp256k1;
use key_wallet::{AccountType, Network};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::AssetLockFunding;
use platform_wallet::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, wait_for_identity_balance,
    CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// Core (Layer-1) duffs the bank delivers to the test wallet's
/// BIP-44 account 0 prior to the asset-lock top-up. Sized to cover
/// the top-up lock amount + asset-lock build fee + Core change UTXO
/// without forcing the operator to top up between runs. Matches
/// CR-003's floor.
const TEST_WALLET_CORE_FUNDING: u64 = 200_000_000;

/// Amount locked into the top-up asset-lock output (in duffs). Per
/// spec ID-002b — 100 M duffs ≈ 0.001 DASH.
const TOP_UP_ASSET_LOCK_AMOUNT: u64 = 100_000_000;

/// DIP-9 identity slot used for the registered + topped-up identity.
const IDENTITY_INDEX: u32 = 0;

/// Credits committed to the address-funded registration. Sized
/// identically to `id_001` so the registered identity's post-reg
/// balance clears the cleanup floor.
const REGISTRATION_FUNDING: u64 = 100_000_000;
const REGISTRATION_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;

/// Per-step wait deadline. 120 s mirrors `id_002` — generous enough
/// for concurrent test runs sharing the testnet.
const STEP_TIMEOUT: Duration = Duration::from_secs(120);

/// Deadline for the on-chain identity balance to reflect the top-up.
const TOP_UP_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(180);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_002b_asset_lock_funded_top_up() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: bring up a Core-funded test wallet. Same shape as
    // CR-003's first step; the helper waits for the SPV-observed
    // confirmed balance to reach `TEST_WALLET_CORE_FUNDING` before
    // returning.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");

    let pre_setup_core = s.test_wallet.core_balance_confirmed();
    assert!(
        pre_setup_core >= TEST_WALLET_CORE_FUNDING,
        "PRE-pin violated: setup_with_core_funded_test_wallet returned with \
         confirmed Core balance {pre_setup_core} < TEST_WALLET_CORE_FUNDING \
         {TEST_WALLET_CORE_FUNDING}"
    );

    // Step 2: register an identity via the address-funded path. ID-002b
    // doesn't care HOW the identity was created — only that there is
    // one to top up. Address-funded is faster and cheaper than asset
    // lock for this purpose, and is what `id_002` already uses.
    let register_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive register address");
    s.ctx
        .bank()
        .fund_address(&register_addr, REGISTRATION_FUNDING_CREDITS)
        .await
        .expect("bank.fund_address(register)");
    // Found-025: the rs-sdk address-sync drops a fetched balance update
    // when the address isn't yet in `pending_addresses`, poisoning the
    // wallet's local sync map under multi-thread churn so
    // `wait_for_balance`'s local-view precondition never reaches target
    // and its proof-verified hand-off never runs. Observe the funding
    // directly via the proof-verified `AddressInfo::fetch` path —
    // the chain-state read the validator itself walks — bypassing the
    // poisoned map. Mirrors `setup_with_per_identity_funding`.
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &register_addr,
        REGISTRATION_FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("register funding never observed");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(register_addr, REGISTRATION_FUNDING, IDENTITY_INDEX)
        .await
        .expect("register_identity_from_addresses");
    let identity_id = registered.id;

    let pre_balance = Identity::fetch(s.ctx.sdk(), identity_id)
        .await
        .expect("fetch pre")
        .expect("identity visible")
        .balance();
    assert!(
        pre_balance > 0,
        "PRE-pin violated: registered identity must have non-zero balance \
         pre top-up (got {pre_balance})"
    );

    // Step 3: drive the asset-lock-funded top-up.
    //
    // Precondition (QA-006): `top_up_identity_with_funding` with
    // `FundWithWallet` calls `create_funded_asset_lock_proof` which
    // looks up the `IdentityTopUp { registration_index: IDENTITY_INDEX }`
    // HD account in the wallet's managed account collection. That account
    // is absent when the wallet is created with
    // `WalletAccountCreationOptions::Default`. Provision it now.
    let seed = s.test_wallet.seed_bytes();
    let network = s.test_wallet.platform_wallet().core().network();
    add_identity_topup_account(
        s.test_wallet.platform_wallet(),
        &seed,
        network,
        IDENTITY_INDEX,
    )
    .await
    .expect(
        "provision IdentityTopUp account via externally-derived xpub — the \
         post-registration wallet is external-signable, so the account must be \
         added with Some(xpub) (watch-only), not the resident-key None path. \
         See TEST_SPEC.md Found-031.",
    );

    // Internally:
    //   1. AssetLockManager::create_funded_asset_lock_proof — builds
    //      the asset-lock tx on Core, broadcasts via SPV, waits for
    //      IS-lock (or falls back to ChainLock).
    //   2. Submits IdentityTopUp with the resolved proof.
    //   3. Updates the local identity-manager balance cache.
    let core_signer = SeedBackedCoreSigner::new(
        s.test_wallet.seed_bytes(),
        s.test_wallet.platform_wallet().core().network(),
    );
    s.test_wallet
        .platform_wallet()
        .identity()
        .top_up_identity_with_funding(
            &identity_id,
            AssetLockFunding::FromWalletBalance {
                amount_duffs: TOP_UP_ASSET_LOCK_AMOUNT,
                account_index: 0,
            },
            &core_signer,
            None,
        )
        .await
        .expect(
            "top_up_identity_with_funding (ID-002b — asset-lock-funded top-up \
             of registered identity)",
        );

    // Step 4: wait for the chain-visible balance to reflect the top-up.
    // The minimum we accept is `pre_balance + (TOP_UP_ASSET_LOCK_AMOUNT
    // * CREDITS_PER_DUFF) / 2` — half-credit threshold mirrors CR-003's
    // half-lock contract, fee-tolerant against protocol-version drift.
    let credited = TOP_UP_ASSET_LOCK_AMOUNT.saturating_mul(CREDITS_PER_DUFF);
    let expected_min = pre_balance.saturating_add(credited / 2);
    let expected_max = pre_balance.saturating_add(credited);
    let post_balance = wait_for_identity_balance(
        s.ctx.sdk(),
        identity_id,
        expected_min,
        TOP_UP_VISIBILITY_TIMEOUT,
    )
    .await
    .expect("identity balance never reflected the top-up");

    // Step 5: pin the upper bound — top-up cannot credit more than the
    // asset-lock output value (fees are subtracted, not added).
    assert!(
        post_balance <= expected_max,
        "POST-pin violated: post-top-up identity balance {post_balance} > \
         expected_max {expected_max} (= pre_balance {pre_balance} + \
         credited {credited}). Top-up cannot credit more than the \
         asset-lock output."
    );

    // Step 6: assert the top-up fee was positive. The fee equals
    // `expected_max - post_balance` — i.e. the credit shortfall vs the
    // theoretical lock amount.
    let top_up_fee = expected_max.saturating_sub(post_balance);
    assert!(
        top_up_fee > 0,
        "POST-pin violated: top_up_fee {top_up_fee} must be positive — \
         on-chain top-up always pays a chain-time fee"
    );

    // Step 7: the unified top-up flow is mark-on-success, NOT
    // remove-on-success — `top_up_identity_with_funding` calls
    // `consume_asset_lock` once Platform accepts the top-up, which sets
    // the entry to `Consumed`, clears its one-shot `proof`, and RETAINS it
    // in the tracked-locks registry. So a successful top-up leaves a
    // `Consumed` IdentityTopUp entry behind — this loop runs over it
    // rather than passing vacuously. Each entry must be in a finalised
    // state: IS/CL (proof materialised) or `Consumed` (Platform accepted a
    // valid proof — strictly stronger). Mirrors CR-003's Step 6.
    let tracked = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    let top_up_locks: Vec<_> = tracked
        .iter()
        .filter(|l| {
            matches!(
                l.funding_type,
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
            )
        })
        .collect();
    for lock in &top_up_locks {
        assert!(
            matches!(
                lock.status,
                AssetLockStatus::InstantSendLocked
                    | AssetLockStatus::ChainLocked
                    | AssetLockStatus::Consumed
            ),
            "POST-pin violated: tracked top-up asset lock {:?} is in \
             non-finalised status {:?} after top_up_identity_with_funding \
             completed",
            lock.out_point,
            lock.status
        );
    }

    // Step 8: assert the test wallet's confirmed Core balance dropped
    // by approximately (TOP_UP_ASSET_LOCK_AMOUNT + asset_lock_fee +
    // core_send_fee). Use a generous lower bound on the drop to stay
    // fee-tolerant; the upper bound is unbounded (large fee = larger
    // drop).
    s.test_wallet
        .sync_balances()
        .await
        .expect("post-top-up sync");
    let post_setup_core = s.test_wallet.core_balance_confirmed();
    let core_drop = pre_setup_core.saturating_sub(post_setup_core);
    assert!(
        core_drop >= TOP_UP_ASSET_LOCK_AMOUNT,
        "POST-pin violated: test-wallet Core balance dropped only {core_drop} \
         duffs (< TOP_UP_ASSET_LOCK_AMOUNT {TOP_UP_ASSET_LOCK_AMOUNT}). The \
         asset-lock build must have consumed at least the lock amount from \
         BIP-44 account 0."
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::id_002b",
        %identity_id,
        pre_balance,
        post_balance,
        top_up_fee,
        core_drop,
        "ID-002b: asset-lock-funded top-up snapshot"
    );

    s.teardown().await.expect("teardown");
}

// ---------------------------------------------------------------------------
// Inline helpers
// ---------------------------------------------------------------------------

/// Provision an `IdentityTopUp { registration_index }` HD account in
/// the wallet's key-wallet and managed-account collection.
///
/// `top_up_identity_with_funding` with `FundWithWallet` calls
/// `create_funded_asset_lock_proof(AssetLockFundingType::IdentityTopUp,
/// identity_index)` which looks up the account keyed by `identity_index`
/// in `wallet_info.accounts.identity_topup`. That map starts empty
/// when the wallet is created with `WalletAccountCreationOptions::Default`
/// — provisioning it here is the required precondition. (QA-006)
///
/// The wallet is external-signable after `register_wallet` (root seed
/// dropped), so `add_account(.., None)` — which re-derives from the resident
/// key — fails. Derive the account xpub from the seed's master exactly as the
/// `None` branch would (`master.derive_priv(account_path).neuter()`) and
/// provision via `Some(xpub)`. The account is watch-only; the asset-lock
/// build and consume both sign through the external `core_signer`, so no
/// resident key is needed. Mirrors the DashPay contact provisioning in
/// `src/wallet/identity/network/contacts.rs`.
async fn add_identity_topup_account(
    wallet: &std::sync::Arc<platform_wallet::PlatformWallet>,
    seed_bytes: &[u8; 64],
    network: Network,
    registration_index: u32,
) -> Result<(), PlatformWalletError> {
    let account_type = AccountType::IdentityTopUp { registration_index };
    let secp = Secp256k1::new();
    let master = ExtendedPrivKey::new_master(network, seed_bytes)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let derivation_path = account_type
        .derivation_path(network)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let account_xpriv = master
        .derive_priv(&secp, &derivation_path)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let account_xpub = ExtendedPubKey::from_priv(&secp, &account_xpriv);

    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let (kw, info) = wm
        .get_wallet_mut_and_info_mut(&wallet_id)
        .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
    kw.add_account(account_type, Some(account_xpub))
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))?;
    let account = kw
        .accounts
        .identity_topup
        .get(&registration_index)
        .expect("just inserted");
    let managed = key_wallet::managed_account::ManagedCoreKeysAccount::from_account(account);
    info.core_wallet
        .accounts
        .insert_keys_bearing_account(managed)
        .map_err(|e| PlatformWalletError::InvalidIdentityData(e.to_string()))
}
