//! CR-004 — Legacy BIP32 account: balance + UTXO state updates after spend.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-004).
//! Status: FAILING-by-design — runs only via `cargo test -- --ignored`
//! and is expected to fail until the upstream contract is fixed.
//! Exercises the multi-variant `next_receive_addresses(count=2, advance=true)`
//! API which forces pool advance; assertion `addr1 != addr2` is now consistent
//! with the upstream contract (`key_wallet::AddressPool::next_unused` is
//! idempotent by design — see upstream `address_pool.rs:1196-1214`; the
//! multi-variant `next_unused_multiple` is the correct API for N distinct
//! frontier addresses).
//! Pins the post-broadcast UTXO-mutation contract on
//! `standard_bip32_accounts` against
//! [dashpay/dash-evo-tool#845](https://github.com/dashpay/dash-evo-tool/issues/845):
//! a "send all" on the legacy BIP32 account must drain the local UTXO
//! set so a follow-up `send_to_addresses` fails cleanly on empty inputs
//! rather than reselecting phantom UTXOs.

use std::time::Duration;

use dashcore::Address as DashAddress;
use key_wallet::account::account_type::StandardAccountType;
use platform_wallet::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::wait::wait_for_core_balance;

/// Per-UTXO funding amount in duffs. Two distinct UTXOs land on the
/// legacy BIP32 account so coin selection has more than one input to
/// consider — matching the SPEC's "build a 'send all' transfer that
/// consumes every UTXO" requirement (without ≥ 2 UTXOs the contract
/// degenerates to "single-input spend", which doesn't exercise the
/// "send all" semantics).
const PER_UTXO_FUNDING: u64 = 50_000_000; // 0.5 DASH testnet

/// Total funding the bank delivers across two `send_core_to` calls.
/// Sized so the bank's `confirmed >= TOTAL + CORE_TX_FEE_RESERVE` gate
/// clears with the same pre-funding floor used by CR-003.
const TOTAL_FUNDING: u64 = PER_UTXO_FUNDING * 2;

/// Deadline for each post-broadcast wait. Matches CR-003's
/// `CORE_FUNDING_TIMEOUT` so cold-cache SPV scans don't false-fail.
const CORE_BALANCE_TIMEOUT: Duration = Duration::from_secs(300);

/// Small Core transfer amount used in step 5 — the second send-attempt
/// after the legacy account has been drained. The exact number doesn't
/// matter; what matters is that coin selection is invoked on a known-empty
/// UTXO set and surfaces a clean failure (NOT an unrelated "select-failed
/// on stale UTXO" error path).
const POST_DRAIN_PROBE_AMOUNT: u64 = 1_000_000;

#[ignore = "CR-004 — FAILING-by-design pin for dash-evo-tool#845; runs only \
            via `cargo test -- --ignored` and is expected to fail until the \
            SPV/BIP32 derivation contract is fixed. Pins the post-broadcast \
            UTXO-mutation contract on `standard_bip32_accounts`. Requires \
            testnet + bank Core (Layer-1) pre-funding (TOTAL_FUNDING duffs + \
            per-tx fee reserve, twice — once per UTXO). The legacy BIP32 \
            account derivation must NOT cross-contaminate the wallet's \
            default BIP-44 Core account UTXO set; assertions read \
            `standard_bip32_accounts[0]` directly."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_004_legacy_bip32_utxo_update_after_spend() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: bring up a fresh test wallet. `WalletAccountCreationOptions::Default`
    // creates BOTH `standard_bip44_accounts[0]` AND `standard_bip32_accounts[0]`
    // (see `key_wallet::wallet::initialization::WalletAccountCreationOptions::Default`
    // doc), so the BIP32 collection is already populated on `setup`.
    let s = crate::framework::setup()
        .await
        .expect("setup (CR-004 — fresh-seeded test wallet with default account set)");

    // Step 2: derive two distinct legacy BIP32 account 0 receive addresses.
    // `next_receive_addresses(count=2, advance=true)` uses the upstream
    // `next_unused_multiple` path which advances the pool index per slot,
    // producing two distinct frontier addresses in one call.
    let [bip32_recv_1, bip32_recv_2] =
        next_two_receive_addresses_for_bip32_account(&s.test_wallet, 0)
            .await
            .expect("derive two legacy BIP32 receive addresses")
            .try_into()
            .expect("next_receive_addresses(2) returned wrong count");
    assert_ne!(
        bip32_recv_1, bip32_recv_2,
        "PRE-pin violated: BIP32 receive-address pool returned the same \
         address for both slots — next_unused_multiple pool advance is broken."
    );

    // Step 3: bank-fund the legacy account with TWO distinct UTXOs.
    // Two `send_core_to` calls — one per receive-address slot — give us
    // two outpoints on the same legacy account, so step 4's "send all"
    // exercises true multi-UTXO coin selection (not a degenerate
    // single-input shape).
    let txid_1 = s
        .ctx
        .bank()
        .send_core_to(&bip32_recv_1, PER_UTXO_FUNDING)
        .await
        .expect("bank.send_core_to (legacy BIP32 slot 1)");
    let txid_2 = s
        .ctx
        .bank()
        .send_core_to(&bip32_recv_2, PER_UTXO_FUNDING)
        .await
        .expect("bank.send_core_to (legacy BIP32 slot 2)");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_004",
        %txid_1,
        %txid_2,
        per_utxo = PER_UTXO_FUNDING,
        total = TOTAL_FUNDING,
        "CR-004: bank delivered two UTXOs to legacy BIP32 account 0"
    );

    // Step 4: wait for the SPV bloom filter to observe the inbound
    // UTXOs. `core_balance_confirmed` aggregates across BIP-44 + BIP-32
    // accounts (see `wallet/core/balance_handler.rs:42` — the upstream
    // `WalletCoreBalance` is wallet-aggregate, not per-account). The
    // test wallet's BIP-44 account 0 is unfunded at this point, so any
    // confirmed balance came from the BIP-32 sends.
    let observed = wait_for_core_balance(&s.test_wallet, TOTAL_FUNDING, CORE_BALANCE_TIMEOUT)
        .await
        .expect("wait_for_core_balance (TOTAL_FUNDING on legacy BIP32 account)");
    assert!(
        observed >= TOTAL_FUNDING,
        "PRE-pin violated: wait_for_core_balance returned with \
         observed {observed} < TOTAL_FUNDING {TOTAL_FUNDING}"
    );

    // Step 4b: cross-account contamination check. The legacy account
    // (BIP-32) must own the new UTXOs, NOT the wallet's default BIP-44
    // account 0. If the BIP-44 account is non-empty, the routing layer
    // is mis-attributing inbound UTXOs and the rest of the test would
    // pass for the wrong reason.
    let (bip44_count_pre, bip32_count_pre) = utxo_counts(&s.test_wallet, 0).await;
    assert_eq!(
        bip44_count_pre, 0,
        "PRE-pin violated: BIP-44 account 0 has {bip44_count_pre} UTXOs \
         after funding the BIP-32 account — cross-account contamination \
         would let the test pass for the wrong reason."
    );
    assert_eq!(
        bip32_count_pre, 2,
        "PRE-pin violated: legacy BIP-32 account 0 has {bip32_count_pre} \
         UTXOs after the bank's two `send_core_to` calls — expected 2."
    );

    // Step 5: build a "send all" Core transfer via
    // `CoreWallet::send_to_addresses(StandardAccountType::BIP32Account, 0, ...)`.
    // `send_to_addresses` selects from the BIP-32 account's spendable
    // set internally and sends change back to the same account; sending
    // TOTAL_FUNDING - 3_000 to a fresh sink address leaves ~500–2_500 duffs
    // of potential change after a typical testnet fee of ~226–500 duffs,
    // which is below the P2PKH dust threshold (~2_730 duffs). The builder
    // folds sub-dust change into the fee, producing a zero-change transaction
    // and leaving the BIP-32 account with no spendable UTXOs.
    //
    // QA-008: the original send amount (TOTAL_FUNDING - 50_000) left ~45_000
    // duffs of change — far above dust — so the builder correctly emitted a
    // change UTXO and `spendable_utxos` returned 1, not 0.
    // QA-009: TOTAL_FUNDING - 2_000 still left change above the dust
    // threshold on low-fee testnet runs (~500 duff fee → ~1_500 duff
    // residual that the builder emitted as change). Raising the headroom
    // to 3_000 ensures the post-fee residual stays sub-dust (~2_730)
    // across the observed testnet fee range of 226–500 duffs.
    //
    // We send to the bank's primary Core receive address so the swept duffs
    // are recoverable on teardown failure.
    let sink = s
        .ctx
        .bank()
        .primary_core_receive_address()
        .await
        .expect("bank.primary_core_receive_address");
    // Subtract 3_000 duffs so the post-fee residual is sub-dust.
    let send_all = TOTAL_FUNDING.saturating_sub(3_000);
    let tx = s
        .test_wallet
        .platform_wallet()
        .core()
        .send_to_addresses(
            StandardAccountType::BIP32Account,
            0,
            vec![(sink.clone(), send_all)],
        )
        .await
        .expect("send_to_addresses(BIP32Account, 0, send_all) failed — broadcast path is broken");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_004",
        txid = %tx.txid(),
        sink = %sink,
        "CR-004: legacy BIP32 send-all broadcast"
    );

    // Step 6: assert the post-broadcast state mutation actually
    // happened on `standard_bip32_accounts[0]`. The contract:
    //
    // - The mempool-context `check_core_transaction` call inside
    //   `send_to_addresses` (see `wallet/core/broadcast.rs:185`) must
    //   route the just-broadcast tx through the BIP-32 account
    //   collection AND mark every consumed UTXO as spent.
    // - `spendable_utxos(current_height)` on the legacy account must
    //   return an empty set. We sent `TOTAL_FUNDING - 3_000` duffs:
    //   observed testnet fees for a 2-in/1-out P2PKH tx are 226–500
    //   duffs, leaving ~2_500–2_774 duffs of potential change — below
    //   the P2PKH dust threshold (~2_730 duffs). The builder folds
    //   sub-dust change into the fee, so no change UTXO is emitted and
    //   the account's spendable set is strictly empty post-broadcast.
    let (bip44_count_post, bip32_count_post) = utxo_counts(&s.test_wallet, 0).await;
    assert_eq!(
        bip44_count_post, 0,
        "POST-pin violated: BIP-44 account 0 grew to {bip44_count_post} \
         UTXOs after a BIP-32 send-all — the broadcast or its
         post-broadcast hook is mis-attributing the change output."
    );
    assert_eq!(
        bip32_count_post, 0,
        "BIP-32 account 0 has {bip32_count_post} spendable UTXOs after send-all \
         (dash-evo-tool#845 regression)"
    );

    // Step 7: re-attempt a Core transfer on the now-drained legacy
    // account. The bug surface in DET#845 is "this fails with a
    // coin-selection error pretending UTXOs exist"; the fix is for
    // it to fail cleanly with a build-stage error that names the
    // empty input set. We pin the looser contract: `Err(_)` AND the
    // error message names "No UTXOs" / "no spendable inputs" / the
    // word "selection" so a regression that returns `Ok(...)` (i.e.
    // the wallet attempts to spend phantom UTXOs) flips the test
    // immediately.
    let probe = s
        .test_wallet
        .platform_wallet()
        .core()
        .send_to_addresses(
            StandardAccountType::BIP32Account,
            0,
            vec![(sink.clone(), POST_DRAIN_PROBE_AMOUNT)],
        )
        .await;
    match probe {
        Err(PlatformWalletError::TransactionBuild(msg)) => {
            assert!(
                msg.to_lowercase().contains("no utxos")
                    || msg.to_lowercase().contains("no spendable")
                    || msg.to_lowercase().contains("coin selection")
                    || msg.to_lowercase().contains("insufficient"),
                "TransactionBuild error does not name the empty-input cause: {msg:?}"
            );
            tracing::info!(
                target: "platform_wallet::e2e::cases::cr_004",
                msg,
                "CR-004: post-drain second send failed cleanly (expected)"
            );
        }
        Err(other) => {
            panic!("expected TransactionBuild on drained BIP-32 account, got {other:?}");
        }
        Ok(tx) => {
            panic!(
                "drained BIP-32 account selected phantom UTXOs (dash-evo-tool#845): txid={}",
                tx.txid()
            );
        }
    }

    // Sanity assert the sink address is on the same network as the
    // wallet — a network mismatch here would mean the send target was
    // wrong all along and the earlier broadcast went somewhere
    // unexpected. `key_wallet::Network` is a re-export of
    // `dashcore::Network`, so a direct `==` works without casting.
    assert_eq!(
        *sink.network(),
        s.ctx.config.network,
        "PRE-pin violated: sink address network does not match test \
         wallet network; CR-004 sweep would broadcast to the wrong chain."
    );

    s.teardown().await.expect("teardown");
}

// ---------------------------------------------------------------------------
// Inline helpers — lift to `framework/` once a stable BIP-32 receive-address
// derivation point lands on `CoreWallet`.
// ---------------------------------------------------------------------------

/// Derive `count=2` distinct receive addresses on the wallet's legacy BIP-32
/// account at `account_index` using the upstream `next_unused_multiple` path
/// (via `ManagedCoreFundsAccount::next_receive_addresses`). Passing
/// `add_to_state=true` forces the pool to commit the generated indices and
/// bump `highest_generated`, guaranteeing the returned addresses are distinct.
async fn next_two_receive_addresses_for_bip32_account(
    test_wallet: &crate::framework::wallet_factory::TestWallet,
    account_index: u32,
) -> Result<Vec<DashAddress>, PlatformWalletError> {
    let wallet = test_wallet.platform_wallet();
    let mut wm = wallet.wallet_manager().write().await;
    let wallet_id = wallet.wallet_id();
    let (kw, info) = wm.get_wallet_and_info_mut(&wallet_id).ok_or_else(|| {
        PlatformWalletError::WalletNotFound(format!(
            "wallet {} missing from manager during BIP-32 receive-address derive",
            hex::encode(wallet_id)
        ))
    })?;

    let xpub = kw
        .accounts
        .standard_bip32_accounts
        .get(&account_index)
        .map(|a| a.account_xpub)
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "BIP-32 account {} not found in wallet — \
                 WalletAccountCreationOptions::Default should have created it",
                account_index
            ))
        })?;

    let account = info
        .core_wallet
        .accounts
        .standard_bip32_accounts
        .get_mut(&account_index)
        .ok_or_else(|| {
            PlatformWalletError::WalletNotFound(format!(
                "BIP-32 managed account {} not found in wallet info",
                account_index
            ))
        })?;

    account
        .next_receive_addresses(Some(&xpub), 2, true)
        .map_err(PlatformWalletError::AddressOperation)
}

/// Snapshot `(bip44_spendable_count, bip32_spendable_count)` at
/// account index `account_index`. Used as a cross-contamination check
/// (BIP-44 must stay empty when only BIP-32 is funded) and as the
/// post-broadcast assertion target (BIP-32 must drop to 0 after a
/// "send all"). Reads through the wallet manager write lock so the
/// snapshot is consistent with the synced height used inside
/// `send_to_addresses`.
async fn utxo_counts(
    test_wallet: &crate::framework::wallet_factory::TestWallet,
    account_index: u32,
) -> (usize, usize) {
    use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;

    let wallet = test_wallet.platform_wallet();
    let wallet_id = wallet.wallet_id();
    let wm = wallet.wallet_manager().read().await;
    let info = wm
        .get_wallet_info(&wallet_id)
        .expect("wallet present in manager");

    let height = info.core_wallet.synced_height();

    let bip44 = info
        .core_wallet
        .accounts
        .standard_bip44_accounts
        .get(&account_index)
        .map(|a| a.spendable_utxos(height).len())
        .unwrap_or(0);
    let bip32 = info
        .core_wallet
        .accounts
        .standard_bip32_accounts
        .get(&account_index)
        .map(|a| a.spendable_utxos(height).len())
        .unwrap_or(0);
    (bip44, bip32)
}
