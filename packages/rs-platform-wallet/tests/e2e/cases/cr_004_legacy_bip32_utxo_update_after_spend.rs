//! CR-004 — Legacy BIP32 account: balance + UTXO state updates after spend.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-004).
//! Status: passing-as-regression — runs only via `cargo test -- --ignored`
//! (testnet + bank Core funding required).
//!
//! Pins two contracts:
//! 1. The multi-variant `next_receive_addresses(count=2, advance=true)` API
//!    advances the address pool per slot (upstream `next_unused_multiple`
//!    path; see `key_wallet::AddressPool::next_unused` idempotency contract
//!    at `address_pool.rs:1196-1214`).
//! 2. dash-evo-tool#845: after a legacy BIP-32 account spends, the change
//!    UTXO is routed *back into the BIP-32 account*, not lost. Post-broadcast
//!    `check_core_transaction`
//!    (`packages/rs-platform-wallet/src/wallet/core/broadcast.rs:252`)
//!    marks every consumed BIP-32 input spent AND registers the change
//!    output as a fresh spendable UTXO on the BIP-32 account collection —
//!    symmetric with the BIP-44 path (TransactionRouter →
//!    ManagedAccountCollection → check_transaction_for_match →
//!    update_utxos). The send leaves an above-dust change UTXO; a second
//!    send must spend exactly that routed-back change and succeed —
//!    proving the change was tracked back into BIP-32 (the #845 contract),
//!    not orphaned.

use std::time::Duration;

use dashcore::Address as DashAddress;
use key_wallet::account::account_type::StandardAccountType;
use platform_wallet::PlatformWalletError;

use crate::framework::bank::core_send_from_account;
use crate::framework::prelude::*;
use crate::framework::signer::SeedBackedCoreSigner;
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
const CORE_BALANCE_TIMEOUT: Duration = Duration::from_secs(180);

/// Headroom (duffs) left unspent by the step-5 send so a real change
/// UTXO survives on the BIP-32 account. Invariant: the headroom minus the
/// largest observed 2-in/2-out P2PKH fee must stay strictly above the
/// upstream `546`-duff dust gate (`if change_amount > 546` at
/// `rust-dashcore/.../managed_wallet_info/transaction_builder.rs:294`),
/// so a spendable change UTXO is *always* emitted and routed back into
/// BIP-32 — the dash-evo-tool#845 contract. With observed testnet fees
/// in `[226, 500]` the change lands in `[1_999_500, 1_999_774]`, far
/// above dust and itself large enough to fund the step-7 follow-up spend.
const SEND_ALL_HEADROOM: u64 = 2_000_000; // 0.02 DASH testnet

/// Amount the step-7 follow-up spends from the *routed-back* BIP-32
/// change UTXO. Comfortably below the surviving change so coin selection
/// succeeds, proving the change was tracked back into BIP-32 (not lost).
const CHANGE_RESPEND_AMOUNT: u64 = 500_000;

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

    // Step 5: spend most of the BIP-32 balance via
    // `core_send_from_account(StandardAccountType::BIP32Account, 0, ...)`,
    // leaving `SEND_ALL_HEADROOM` unspent so a real, above-dust change
    // UTXO survives. The upstream gate `if change_amount > 546`
    // (`rust-dashcore/.../managed_wallet_info/transaction_builder.rs:294`)
    // emits the change output; post-broadcast `check_core_transaction`
    // (`broadcast.rs:252`) must route it back onto the BIP-32 account —
    // the dash-evo-tool#845 contract.
    //
    // We send to the bank's primary Core receive address so the swept duffs
    // are recoverable on teardown failure.
    let sink = s
        .ctx
        .bank()
        .primary_core_receive_address()
        .await
        .expect("bank.primary_core_receive_address");
    let send_amount = TOTAL_FUNDING.saturating_sub(SEND_ALL_HEADROOM);
    let core_signer = SeedBackedCoreSigner::new(
        s.test_wallet.seed_bytes(),
        s.test_wallet.platform_wallet().core().network(),
    );
    let tx = core_send_from_account(
        s.test_wallet.platform_wallet(),
        StandardAccountType::BIP32Account,
        0,
        vec![(sink.clone(), send_amount)],
        &core_signer,
    )
    .await
    .expect("Core BIP32 build/sign/broadcast failed — broadcast path is broken");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_004",
        txid = %tx.txid(),
        sink = %sink,
        "CR-004: legacy BIP32 partial spend broadcast"
    );

    // Step 6: assert the post-broadcast state mutation actually
    // happened on `standard_bip32_accounts[0]`. The #845 contract:
    //
    // - The mempool-context `check_core_transaction` call inside
    //   the broadcast helper marks
    //   both consumed BIP-32 inputs spent AND registers the above-dust
    //   change output as a fresh spendable UTXO on the BIP-32 account.
    // - The two funding UTXOs are spent and exactly one change UTXO is
    //   routed back, so `spendable_utxos` on the legacy account is
    //   `{change}` — count == 1. A count of 0 means the change was
    //   orphaned (the #845 regression); a count of 2 means a consumed
    //   input was not marked spent. The change must NOT leak onto BIP-44.
    let (bip44_count_post, bip32_count_post) = utxo_counts(&s.test_wallet, 0).await;
    assert_eq!(
        bip44_count_post, 0,
        "POST-pin violated: BIP-44 account 0 grew to {bip44_count_post} \
         UTXOs after a BIP-32 spend — the broadcast or its \
         post-broadcast hook is mis-attributing the change output."
    );
    assert_eq!(
        bip32_count_post, 1,
        "dash-evo-tool#845 regression: BIP-32 account 0 has \
         {bip32_count_post} spendable UTXOs after the spend — expected \
         exactly 1 (the routed-back change). 0 means the change UTXO was \
         orphaned instead of tracked back into BIP-32; 2 means a consumed \
         input was not marked spent."
    );

    // Step 7: the positive #845 proof. Spend again from the BIP-32
    // account. The two funding UTXOs are spent; the only spendable input
    // is the change UTXO routed back by step 5's post-broadcast
    // `check_core_transaction`. If that change was correctly tracked
    // back into BIP-32 this send succeeds, selecting exactly the change
    // outpoint. A `NoSpendableInputs`/`TransactionBuild` here means the
    // change was orphaned (the #845 regression: change not routed back);
    // a wrong-input selection would mean a consumed input was not marked
    // spent.
    let respend = core_send_from_account(
        s.test_wallet.platform_wallet(),
        StandardAccountType::BIP32Account,
        0,
        vec![(sink.clone(), CHANGE_RESPEND_AMOUNT)],
        &core_signer,
    )
    .await;
    match respend {
        Ok(tx) => {
            tracing::info!(
                target: "platform_wallet::e2e::cases::cr_004",
                txid = %tx.txid(),
                "CR-004: respend consumed the routed-back BIP-32 change UTXO (#845 held)"
            );
        }
        Err(PlatformWalletError::NoSpendableInputs { context, .. }) => {
            panic!(
                "dash-evo-tool#845 regression: BIP-32 change UTXO was not \
                 routed back, so the follow-up spend found no inputs \
                 ({context}). The change was orphaned instead of tracked \
                 into the BIP-32 account."
            );
        }
        Err(other) => {
            panic!(
                "follow-up spend of the routed-back BIP-32 change failed \
                 unexpectedly: {other:?}"
            );
        }
    }

    // Sanity assert the sink address is on the same network as the
    // wallet — a network mismatch here would mean the send target was
    // wrong all along and the earlier broadcast went somewhere
    // unexpected. `Address<NetworkChecked>` exposes no network getter, so
    // validate through the unchecked view's `is_valid_for_network`.
    assert!(
        sink.as_unchecked()
            .is_valid_for_network(s.ctx.config.network),
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
/// the split Core transaction builder/broadcast path.
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
