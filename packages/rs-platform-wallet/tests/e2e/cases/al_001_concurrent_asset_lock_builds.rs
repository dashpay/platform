//! AL-001 — Concurrent asset-lock builds from same wallet.
//!
//! **Found-031 reachability proof (under concurrency)**: step 3's precondition
//! (`add_identity_topup_account`) provisions one `IdentityTopUp` account per
//! identity slot on a post-registration wallet. `register_wallet` calls
//! `downgrade_to_external_signable()` (`wallet_lifecycle.rs:244`), dropping the
//! root seed, so the resident-key `add_account(.., None)` path fails with
//! "External signable wallet has no private key". Each account is instead
//! provisioned with an externally-derived `Some(xpub)` — watch-only, signed at
//! build/consume time by the external `core_signer` — proving the asset-lock
//! top-up is reachable on an external-signable wallet under N-way concurrent
//! load. See TEST_SPEC.md Found-031.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Asset Lock (AL) → AL-001).
//! Pinned status: active regression guard — full test body implemented,
//! gated behind the `e2e` cargo feature, then ONLY runs behind the same `PLATFORM_WALLET_E2E_BANK_CORE_GATE`
//! env gate CR-003 and ID-002b use (funded testnet run; exercised by the
//! gated solo concurrency job, not the default suite). Found-008 is FIXED
//! (waiter-side pre-arm in `sync/proof.rs`, both wait loops, #3634); this
//! test now guards against a regression of that fix under concurrent load.
//! Requires bank Core (Layer-1) pre-funding large enough for N parallel
//! asset locks + fees (~5 DASH testnet).
//!
//! `AssetLockManager` is critical-path code that every asset-lock-funded
//! registration and top-up goes through, but CR-003 / ID-002b only
//! exercise the sequential single-build happy path. AL-001 fires
//! `N = 3` concurrent `top_up_identity_with_funding` calls (each
//! against a DIFFERENT identity so every task drives an independent
//! asset-lock build path with no shared HD-slot contention)
//! so the manager's locking, UTXO-reservation, and proof-correlation
//! logic is exercised under concurrent load.
//!
//! Assertions:
//! - All N tasks return `Ok(_)` (Step 4) — reachability under concurrency.
//! - All N identity balances increased post-top-up (Step 6).
//! - No lingering `tracked_asset_locks` entry is in a non-finalised status
//!   (Step 5). `consume_asset_lock` marks each entry `Consumed` on Platform
//!   accept and RETAINS it for historical lookup, so a full-success run
//!   leaves a registry of `Consumed` entries rather than an empty one; this
//!   pins that no half-finalised lock survives.
//! - Pairwise-distinct asset-lock txids and pairwise-disjoint funding
//!   inputs (no manager collision, no UTXO double-spend) are enforced
//!   by key-wallet's funding-account reservation set; a violation fails
//!   a build at coin-selection/broadcast, so it surfaces as a task `Err`
//!   in Step 4 (the consumed entries can't be re-inspected post-success).
//!
//! Critical-path interactions this guard also covers:
//! - Found-008 (`LockNotifyHandler` / `wait_for_proof` missed-wakeup) is
//!   FIXED: `sync/proof.rs` arms the `Notify` future (`notified();
//!   pin!; enable()`) BEFORE the state check in both wait loops, so an
//!   IS-lock event arriving in the former check/await gap is no longer
//!   lost. Under concurrent load (N parallel `wait_for_proof` waiters)
//!   this is exactly the path that previously stalled on
//!   `FinalityTimeout`; a failing task fails this guard loudly.
//!
//!   `FinalityTimeout` has two causes and Step 4 classifies which via
//!   the in-process tracked-lock status (NOT log-scraping): if the
//!   proof materialised (lock `InstantSendLocked`/`ChainLocked`) but
//!   the waiter still timed out → Found-008 regression, fail as such;
//!   if no proof materialised (lock still `Built`/`Broadcast`) → the
//!   chain never produced finality in the production budget — testnet
//!   IS-lock/ChainLock liveness under concurrency, still a hard fail
//!   but classified ENVIRONMENTAL so it is not misread as Found-008
//!   (validation run #544: 2/3 asset-lock txs got no IS-lock and
//!   testnet ChainLock cadence stalled ~242 s vs the production 300 s
//!   `wait_for_proof` budget — the #3634 pre-arm woke the waiter on
//!   every event; zero missed wakeups).
//!
//!   NOTE — the finality budget is a PRODUCTION constant
//!   (`asset_lock/build.rs` `wait_for_proof(.., 300 s)`,
//!   `registration.rs` `CL_FALLBACK_TIMEOUT = 180 s`), reached via
//!   `top_up_identity_with_funding`. AL-001 has no test-side timeout
//!   lever and deliberately does not retune production finality timing
//!   (matches commit 7e57f7227a's "al_001 untouched (env)" decision):
//!   the environmental classification + diagnostic message is the
//!   correct test-side response, not inflating a shared production
//!   timeout for every identity registration/top-up in the codebase.
//! - Found-012 (account-type tunnel vision in `validate_or_upgrade_proof`)
//!   is also on the path. If any of the N asset-lock transactions
//!   ends up funded from a non-BIP-44 account, the test hits
//!   Found-012. With BIP-44 account 0 funding this is not expected
//!   today; flag it if a future harness extension changes the
//!   account routing.
//!
//! QA-011: AL-001 requires N+1 pre-split UTXOs on the test wallet's
//! BIP-44 account 0 before the concurrent fan-out in step 3. Without
//! the split, all N tasks compete for a single UTXO; with key-wallet's
//! UTXO reservations, N-1 tasks fail fast with
//! `NoSpendableInputs` (formerly `Coin selection error: No UTXOs available
//! for selection` before the variant split). Step 1b self-sends the
//! entire Core balance to N+1 fresh receive addresses so coin selection
//! always has a dedicated candidate per task.

use std::time::Duration;

use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;
use dpp::prelude::Identifier;
use key_wallet::account::account_type::StandardAccountType;
use key_wallet::bip32::{ExtendedPrivKey, ExtendedPubKey};
use key_wallet::dashcore::secp256k1::Secp256k1;
use key_wallet::{AccountType, Network};
use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
use platform_wallet::AssetLockFunding;
use platform_wallet::PlatformWalletError;

use crate::framework::bank::core_send_from_account;
use crate::framework::prelude::*;
use crate::framework::signer::SeedBackedCoreSigner;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, wait_for_core_balance, wait_for_identity_balance,
    CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};
use dash_sdk::platform::Fetch;

/// Number of concurrent asset-lock builds AL-001 fires. Per spec —
/// 3 is enough to exercise concurrency without exhausting testnet
/// bank funding.
const N: usize = 3;

/// Per-lock asset-lock amount (duffs). 100 M duffs ≈ 0.001 DASH, same
/// shape CR-003 / ID-002b use.
const LOCK_AMOUNT: u64 = 100_000_000;

/// Core (Layer-1) duffs the bank delivers to the test wallet's
/// BIP-44 account 0. Spec floor:
/// `N × (LOCK_AMOUNT + asset_lock_fee + core_tx_fee) + setup_overhead`
/// ≈ 500 M duffs (5 DASH testnet). 500 M leaves comfortable headroom
/// for fee variance and the bank's own change UTXO.
const CONCURRENT_LOCK_FUNDING_TOTAL: u64 = 500_000_000;

/// Credits committed to each address-funded identity registration.
/// Same shape as `id_001` — well above the 50 M `IDENTITY_SWEEP_FLOOR`
/// so teardown sweeps the residual credits.
const REGISTRATION_FUNDING: u64 = 100_000_000;
const REGISTRATION_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;

/// Fee headroom reserved for the N+1-output UTXO split self-send. The
/// split tx is small (~1-5K duffs at typical testnet fee rates); 10K
/// gives comfortable margin so coin selection always succeeds.
const SPLIT_TX_FEE_RESERVE: u64 = 10_000;

/// Per-step wait deadline. Concurrent-load tests warrant a longer
/// deadline than the single-shot cases.
const STEP_TIMEOUT: Duration = Duration::from_secs(180);

/// Deadline for chain-visible balance reflection on each topped-up
/// identity.
const TOP_UP_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(240);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn al_001_concurrent_asset_lock_builds() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: Core-funded test wallet sized for N parallel locks.
    let s = crate::framework::setup_with_core_funded_test_wallet(CONCURRENT_LOCK_FUNDING_TOTAL)
        .await
        .expect("setup_with_core_funded_test_wallet failed");

    let pre_setup_core = s.test_wallet.core_balance_confirmed();
    assert!(
        pre_setup_core >= CONCURRENT_LOCK_FUNDING_TOTAL,
        "PRE-pin violated: confirmed Core balance {pre_setup_core} < \
         CONCURRENT_LOCK_FUNDING_TOTAL {CONCURRENT_LOCK_FUNDING_TOTAL}"
    );

    // Step 1b: pre-split the bank UTXO into N+1 separate UTXOs so
    // each concurrent top-up task in step 3 has a dedicated coin to
    // select. Without the split, all N tasks compete for a single
    // bank UTXO and N-1 of them fail with "No UTXOs available for
    // selection" (QA-011). We build a self-send with N+1 outputs to
    // freshly-derived BIP-44 account-0 receive addresses, each
    // carrying ~CONCURRENT_LOCK_FUNDING_TOTAL / (N+1) duffs.
    // Reserve fee headroom for the split tx itself (1-5K duffs typical;
    // 10K gives margin). Each downstream concurrent top-up still gets
    // ~125M duffs minus a small remainder.
    let split_amount = (CONCURRENT_LOCK_FUNDING_TOTAL - SPLIT_TX_FEE_RESERVE) / (N as u64 + 1);
    let mut split_outputs: Vec<(dashcore::Address, u64)> = Vec::with_capacity(N + 1);
    for _ in 0..=N {
        let addr = s
            .test_wallet
            .platform_wallet()
            .core()
            .next_receive_address_for_account(0)
            .await
            .expect("derive BIP-44 receive address for UTXO split");
        split_outputs.push((addr, split_amount));
    }
    let split_signer = SeedBackedCoreSigner::new(
        s.test_wallet.seed_bytes(),
        s.test_wallet.platform_wallet().core().network(),
    );
    let split_tx = core_send_from_account(
        s.test_wallet.platform_wallet(),
        StandardAccountType::BIP44Account,
        0,
        split_outputs,
        &split_signer,
    )
    .await
    .expect("UTXO pre-split self-send failed");
    tracing::info!(
        target: "platform_wallet::e2e::cases::al_001",
        txid = %split_tx.txid(),
        n_outputs = N + 1,
        split_amount,
        "AL-001: pre-split into N+1 UTXOs for concurrent coin selection"
    );
    // Wait for the split to be SPV-visible before spawning concurrent tasks.
    //
    // Two-phase gate:
    // 1. Aggregate balance — the SPV-updated atomic reaches the expected
    //    total. Fast to observe; guards against the split tx not arriving
    //    at all.
    // 2. Spendable-UTXO count — BIP-44 account 0 has at least N+1
    //    individual UTXOs in its spendable set. This is the condition
    //    `build_asset_lock` actually needs: coin selection reads the
    //    UTXO list, not the balance atomic. The aggregate atomic can
    //    update before the UTXO index catches up, so gating only on
    //    step 1 leaves a window where all N concurrent tasks see "No
    //    UTXOs available for selection" (v47 failure mode).
    let expected_post_split = split_amount.saturating_mul(N as u64 + 1);
    wait_for_core_balance(&s.test_wallet, expected_post_split, STEP_TIMEOUT)
        .await
        .expect("UTXO pre-split aggregate balance not observed by SPV within timeout");

    {
        use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
        let wallet = s.test_wallet.platform_wallet();
        let wallet_id = wallet.wallet_id();
        wait_for(
            || async {
                let wm = wallet.wallet_manager().read().await;
                let info = wm.get_wallet_info(&wallet_id)?;
                let height = info.core_wallet.synced_height();
                let count = info
                    .core_wallet
                    .accounts
                    .standard_bip44_accounts
                    .get(&0)
                    .map(|a| a.spendable_utxos(height).len())
                    .unwrap_or(0);
                if count > N {
                    Some(())
                } else {
                    None
                }
            },
            STEP_TIMEOUT,
        )
        .await
        .expect("split UTXOs not spendable in BIP-44 account 0 within timeout");
    }

    // Step 2: register N identities via the address-funded path. The
    // concurrent top-ups in step 3 target DIFFERENT identities so each
    // drives an independent asset-lock build path.
    let mut identity_ids: Vec<Identifier> = Vec::with_capacity(N);
    let mut pre_balances: Vec<u64> = Vec::with_capacity(N);
    for i in 0..N {
        let identity_index = i as u32;
        let funding_addr = s
            .test_wallet
            .next_unused_address()
            .await
            .expect("derive register address");
        s.ctx
            .bank()
            .fund_address(&funding_addr, REGISTRATION_FUNDING_CREDITS)
            .await
            .expect("bank.fund_address(register)");
        // Found-025: the rs-sdk address-sync drops a fetched balance
        // update when the address isn't yet in `pending_addresses`,
        // poisoning the wallet's local sync map under multi-thread churn
        // so `wait_for_balance`'s local-view precondition never reaches
        // target and its proof-verified hand-off never runs. Observe the
        // funding directly via the proof-verified `AddressInfo::fetch`
        // path — the chain-state read the validator itself walks —
        // bypassing the poisoned map. Mirrors
        // `setup_with_per_identity_funding`.
        wait_for_address_balance_chain_confirmed_n(
            s.ctx.sdk(),
            &funding_addr,
            REGISTRATION_FUNDING_CREDITS,
            CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
            STEP_TIMEOUT,
        )
        .await
        .expect("register funding never observed");

        let registered = s
            .test_wallet
            .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, identity_index)
            .await
            .expect("register_identity_from_addresses");

        let pre = Identity::fetch(s.ctx.sdk(), registered.id)
            .await
            .expect("fetch pre")
            .expect("identity visible")
            .balance();
        identity_ids.push(registered.id);
        pre_balances.push(pre);
    }

    // Step 3: spawn N concurrent top-up tasks. Each task drives an
    // independent asset-lock build via `top_up_identity_with_funding`'s
    // `FundWithWallet` path. The wallet handle is `Arc`-shared via
    // `platform_wallet()` so each task gets its own clone.
    //
    // Precondition (QA-006): provision one `IdentityTopUp` HD account
    // per identity slot (0..N). `top_up_identity_with_funding` with
    // `FundWithWallet` looks up the account by `identity_index` in the
    // managed account collection, which starts empty under
    // `WalletAccountCreationOptions::Default`.
    let core_network = s.test_wallet.platform_wallet().core().network();
    let seed = s.test_wallet.seed_bytes();
    for i in 0..N {
        add_identity_topup_account(
            s.test_wallet.platform_wallet(),
            &seed,
            core_network,
            i as u32,
        )
        .await
        .unwrap_or_else(|e| {
            panic!(
                "provision IdentityTopUp account (slot {i}) via externally-derived \
                     xpub — the post-registration wallet is external-signable, so the \
                     account must be added with Some(xpub), not the resident-key None \
                     path. See TEST_SPEC.md Found-031. error={e}"
            )
        });
    }

    let mut handles = Vec::with_capacity(N);
    for identity_id in identity_ids.iter() {
        let wallet = s.test_wallet.platform_wallet().clone();
        let id = *identity_id;
        let signer = SeedBackedCoreSigner::new(seed, core_network);
        let handle = tokio::spawn(async move {
            wallet
                .identity()
                .top_up_identity_with_funding(
                    &id,
                    AssetLockFunding::FromWalletBalance {
                        amount_duffs: LOCK_AMOUNT,
                        account_index: 0,
                    },
                    &signer,
                    None,
                )
                .await
        });
        handles.push(handle);
    }

    // Step 4: collect results. Every task must succeed.
    let mut results = Vec::with_capacity(N);
    for (i, handle) in handles.into_iter().enumerate() {
        let res = handle
            .await
            .unwrap_or_else(|e| panic!("task {i} panicked: {e}"));
        results.push(res);
    }

    // Found-008-vs-environmental discriminator.
    //
    // A failing task is ALWAYS a hard failure here — this guard never
    // green-paints. But a `FinalityTimeout` has two distinct causes and
    // the panic message must say which, so CI / a human can tell a real
    // regression from testnet variance without log-scraping 20k+ lines
    // (dashpay/platform#3641, validation run #544):
    //
    //   - Found-008 regression (the #3634 `sync/proof.rs` waiter pre-arm
    //     broke): the proof DID materialise — the tracked asset lock
    //     reached `InstantSendLocked`/`ChainLocked` — but `wait_for_proof`
    //     was not woken to consume it and still returned `FinalityTimeout`.
    //     This is the missed-wakeup signature; fail LOUDLY as Found-008.
    //
    //   - Environmental: the chain never produced the finality proof in
    //     budget — the tracked lock is still `Built`/`Broadcast` with
    //     `proof: None`. testnet IS-lock + ChainLock liveness, not a
    //     code defect. Still a hard failure (the run did not prove the
    //     guard green), but classified so it is not misread as Found-008.
    //
    // Status is read from the in-process tracked-lock registry
    // (`list_tracked_locks`), not logs — a robust, non-brittle signal.
    let tracked_for_diag = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    for (i, res) in results.iter().enumerate() {
        let Err(err) = res else {
            continue;
        };
        if let PlatformWalletError::FinalityTimeout(out_point) = err {
            let lock = tracked_for_diag.iter().find(|l| l.out_point == *out_point);
            let proof_materialised = lock.is_some_and(|l| {
                matches!(
                    l.status,
                    AssetLockStatus::InstantSendLocked | AssetLockStatus::ChainLocked
                ) || l.proof.is_some()
            });
            if proof_materialised {
                panic!(
                    "FOUND-008 REGRESSION (task {i}): the asset-lock proof \
                     for {out_point:?} materialised (tracked status {:?}, \
                     proof present) but `wait_for_proof` was not woken to \
                     consume it and returned FinalityTimeout. This is the \
                     missed-wakeup signature the #3634 `sync/proof.rs` \
                     waiter pre-arm exists to prevent — the pre-arm has \
                     regressed. See dashpay/platform#3641.",
                    lock.map(|l| &l.status)
                );
            }
            panic!(
                "ENVIRONMENTAL (task {i}, NOT Found-008): FinalityTimeout \
                 for {out_point:?} with no proof materialised (tracked \
                 status {:?}). The chain did not produce an InstantSend \
                 lock or ChainLock for this asset-lock tx within the \
                 production finality budget — testnet IS-lock / ChainLock \
                 liveness under N-way concurrent load, not a code \
                 regression and not the Found-008 pre-arm (which only \
                 matters once a proof exists to be woken for). Re-run \
                 solo during a healthier testnet window. See \
                 dashpay/platform#3641 and validation run #544.",
                lock.map(|l| &l.status)
            );
        }
        panic!("POST-pin violated: concurrent top-up task {i} failed: {res:?}");
    }

    // Step 5: the unified top-up flow is mark-on-success, NOT
    // remove-on-success — `top_up_identity_with_funding` calls
    // `consume_asset_lock` once Platform accepts each top-up, which sets
    // that entry to `Consumed`, clears its one-shot `proof`, and RETAINS
    // it for historical lookup (`asset_lock/sync/tracking.rs`). After N
    // successful concurrent top-ups the registry therefore holds N
    // `Consumed` IdentityTopUp entries, not zero. Each must be in a
    // finalised state: IS/CL (proof materialised) or `Consumed`
    // (Platform accepted a valid proof — strictly stronger).
    //
    // The concurrency invariants AL-001 guards — pairwise-distinct
    // asset-lock txids and pairwise-disjoint funding inputs (no manager
    // collision, no UTXO double-spend) — are enforced upstream by
    // key-wallet's funding-account reservation set: a violation makes a
    // build fail at coin-selection/broadcast, surfacing as a task `Err`
    // that Step 4 fails on. `top_up_identity_with_funding` returns only the new
    // balance (no txid), and a consumed entry has its `proof` cleared as
    // one-shot, so those properties can't be re-derived from the registry
    // post-success;
    // they are proven transitively by every task returning `Ok`
    // (Step 4) and every identity balance increasing (Step 6).
    let tracked = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    for lock in tracked.iter().filter(|l| {
        matches!(
            l.funding_type,
            key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
        )
    }) {
        assert!(
            matches!(
                lock.status,
                AssetLockStatus::InstantSendLocked
                    | AssetLockStatus::ChainLocked
                    | AssetLockStatus::Consumed
            ),
            "POST-pin violated: tracked top-up asset lock {:?} in \
             non-finalised status {:?}",
            lock.out_point,
            lock.status
        );
    }

    // Step 6: every identity must have a chain-visible balance increase.
    for (i, identity_id) in identity_ids.iter().enumerate() {
        let pre = pre_balances[i];
        let credited = LOCK_AMOUNT.saturating_mul(CREDITS_PER_DUFF);
        let expected_min = pre.saturating_add(credited / 2);
        let observed = wait_for_identity_balance(
            s.ctx.sdk(),
            *identity_id,
            expected_min,
            TOP_UP_VISIBILITY_TIMEOUT,
        )
        .await
        .unwrap_or_else(|e| {
            panic!(
                "POST-pin violated: identity {identity_id} (slot {i}) \
                 balance never reached {expected_min} (pre={pre}, \
                 credited={credited}): {e:?}"
            )
        });
        assert!(
            observed > pre,
            "POST-pin violated: identity {identity_id} balance \
             {observed} did not increase from pre {pre} after \
             concurrent top-up"
        );
    }

    tracing::info!(
        target: "platform_wallet::e2e::cases::al_001",
        n = N,
        lock_amount = LOCK_AMOUNT,
        "AL-001: {N} concurrent asset-lock builds succeeded"
    );

    s.teardown().await.expect("teardown");
}

// ---------------------------------------------------------------------------
// Inline helpers
// ---------------------------------------------------------------------------

/// Provision an `IdentityTopUp { registration_index }` HD account in
/// the wallet's key-wallet and managed-account collection.
///
/// `top_up_identity_with_funding` with `FundWithWallet` looks up the
/// account by `identity_index` in `wallet_info.accounts.identity_topup`,
/// which starts empty under `WalletAccountCreationOptions::Default`.
/// Provision the slot here before spawning the concurrent top-up tasks.
/// (QA-006)
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
