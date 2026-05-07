//! CR-004 — Legacy BIP32 account: balance + UTXO state updates after spend.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-004).
//! Pinned status: FAILING-by-design — the canonical contract for the
//! post-broadcast UTXO mutation on `standard_bip32_accounts` is asserted
//! end-to-end. Mirrors the bug surfaced by
//! [dashpay/dash-evo-tool#845](https://github.com/dashpay/dash-evo-tool/issues/845):
//! after a "send all" spend on the legacy BIP32 account, the wallet's
//! local UTXO set is left stale and a follow-up
//! [`CoreWallet::send_to_addresses`] fails with a coin-selection error
//! ("No UTXOs available for selection") instead of a clean
//! insufficient-funds path.
//!
//! Why FAILING-by-design and not WIRED:
//! 1. The bug ships from a downstream consumer (DET v0.9.x). The test
//!    pins the rs-platform-wallet contract so a future routing
//!    regression in `key_wallet::ManagedWalletInfo::check_core_transaction`
//!    that drops `StandardBIP32` from the standard-tx routing table is
//!    caught here, not in dash-evo-tool's issue tracker.
//! 2. The production fix lives behind the `wallet/core/broadcast.rs:185`
//!    `check_core_transaction(Mempool, ...)` call site (now annotated with
//!    a `TODO(CR-004)` breadcrumb). The downstream router currently DOES
//!    iterate BIP32 (see `key_wallet::transaction_checking::transaction_router`
//!    at the pinned revision — `StandardBIP32` is in
//!    `get_relevant_account_types(TransactionType::Standard)`), but the
//!    test exercises the contract end-to-end so any silent regression is
//!    visible the moment a CI run flips status.
//! 3. The harness does NOT yet expose
//!    `CoreWallet::next_receive_address_for_bip32_account` (the BIP-44
//!    sibling on `wallet/core/wallet.rs:59` has no BIP-32 counterpart);
//!    the helper this test reaches for is implemented INLINE here. When
//!    a parallel branch lifts that into the public surface, the inline
//!    helper collapses to a one-line call.

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

#[ignore = "CR-004 — FAILING-by-design until SPV runtime gates clear AND the \
            harness exposes a stable BIP32-receive-address derivation point. \
            Pins the post-broadcast UTXO-mutation contract on \
            `standard_bip32_accounts` (dash-evo-tool#845). Requires testnet \
            + bank Core (Layer-1) pre-funding (TOTAL_FUNDING duffs + per-tx \
            fee reserve, twice — once per UTXO). The legacy BIP32 account \
            derivation must NOT cross-contaminate the wallet's default \
            BIP-44 Core account UTXO set; assertions read \
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

    // Step 2: derive the legacy BIP32 account 0 receive address INLINE.
    // `CoreWallet` has no `next_receive_address_for_bip32_account` helper
    // today (see module docstring); we mirror the BIP-44 helper's shape
    // (`packages/rs-platform-wallet/src/wallet/core/wallet.rs:59`) directly
    // against `standard_bip32_accounts`.
    let bip32_recv_1 = next_receive_address_for_bip32_account(&s.test_wallet, 0)
        .await
        .expect("derive legacy BIP32 receive address (slot 1)");
    let bip32_recv_2 = next_receive_address_for_bip32_account(&s.test_wallet, 0)
        .await
        .expect("derive legacy BIP32 receive address (slot 2)");
    assert_ne!(
        bip32_recv_1, bip32_recv_2,
        "PRE-pin violated: BIP32 receive-address pool returned the same \
         address twice — pool advance is broken or marking-used dropped \
         the inbound funding event."
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
    // the FULL TOTAL_FUNDING to a fresh sink address forces selection
    // to consume both UTXOs and emit a near-zero (or zero) change
    // output, exercising the "send all" semantics the bug report
    // names.
    //
    // The fee is taken from the consumed inputs, so the actual
    // delivered amount lands slightly under TOTAL_FUNDING — that's
    // fine, the contract under test is "the SOURCE account's UTXO set
    // becomes empty after broadcast", not "the destination receives
    // exactly N". We send to the bank's primary Core receive address
    // so the swept duffs are recoverable on teardown failure.
    let sink = s
        .ctx
        .bank()
        .primary_core_receive_address()
        .await
        .expect("bank.primary_core_receive_address");
    let send_all = TOTAL_FUNDING.saturating_sub(50_000); // leave headroom for fee
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
        .expect(
            "send_to_addresses(BIP32Account, 0, send_all) — the legacy \
             BIP32 broadcast path must succeed; failure here means the \
             broadcast itself is broken, not the post-broadcast state \
             update CR-004 actually pins.",
        );
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
    //   return an empty set (or, at most, an unspent change output —
    //   but since we sent `TOTAL_FUNDING - 50_000` with fee deducted
    //   from inputs, the change is below the dust floor and the
    //   builder will have folded it into the fee, so we expect
    //   strictly empty here).
    let (bip44_count_post, bip32_count_post) = utxo_counts(&s.test_wallet, 0).await;
    assert_eq!(
        bip44_count_post, 0,
        "POST-pin violated: BIP-44 account 0 grew to {bip44_count_post} \
         UTXOs after a BIP-32 send-all — the broadcast or its
         post-broadcast hook is mis-attributing the change output."
    );
    assert_eq!(
        bip32_count_post, 0,
        "POST-pin violated (dash-evo-tool#845): legacy BIP-32 account \
         0 has {bip32_count_post} spendable UTXOs after a `send_to_addresses` \
         broadcast that consumed both inputs. The post-broadcast \
         `check_core_transaction(Mempool, ...)` call at \
         `wallet/core/broadcast.rs:185` must have marked the consumed \
         UTXOs spent on `standard_bip32_accounts[0]`. If this fires, \
         either (a) the routing in \
         `key_wallet::transaction_checking::transaction_router::TransactionRouter::get_relevant_account_types(TransactionType::Standard)` \
         dropped `StandardBIP32` (regression — the legacy DET 0.9.x \
         path), or (b) the post-broadcast hook is being invoked but \
         the `account_type_match` for BIP-32 has lost its mark-spent \
         side effect."
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
                "POST-pin violated: second send_to_addresses on a drained \
                 legacy BIP-32 account failed with TransactionBuild but \
                 the message does NOT identify the empty-input cause: \
                 {msg:?}. The contract requires a clean \
                 'no spendable inputs' / 'insufficient' / coin-selection \
                 message."
            );
            tracing::info!(
                target: "platform_wallet::e2e::cases::cr_004",
                msg,
                "CR-004: post-drain second send failed cleanly (expected)"
            );
        }
        Err(other) => {
            panic!(
                "POST-pin violated: second send_to_addresses on a drained \
                 legacy BIP-32 account failed with {other:?} — expected \
                 PlatformWalletError::TransactionBuild naming the empty \
                 input set."
            );
        }
        Ok(tx) => {
            panic!(
                "POST-pin violated (dash-evo-tool#845): second \
                 send_to_addresses on a drained legacy BIP-32 account \
                 RETURNED Ok with txid {} — the wallet selected phantom \
                 UTXOs that the post-broadcast hook should have marked \
                 spent. This is the exact buggy path the upstream issue \
                 reports.",
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
// Inline helpers (lift to `framework/` when CR-004 graduates from
// FAILING-by-design — see module docstring point 3).
// ---------------------------------------------------------------------------

/// Derive the next unused receive address on the wallet's legacy BIP-32
/// account at `account_index`. Mirror of
/// [`platform_wallet::wallet::core::CoreWallet::next_receive_address_for_account`]
/// (`packages/rs-platform-wallet/src/wallet/core/wallet.rs:59`) but
/// against `standard_bip32_accounts`.
async fn next_receive_address_for_bip32_account(
    test_wallet: &crate::framework::wallet_factory::TestWallet,
    account_index: u32,
) -> Result<DashAddress, PlatformWalletError> {
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
        .next_receive_address(Some(&xpub), true)
        .map_err(|e| PlatformWalletError::AddressOperation(e.to_string()))
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
