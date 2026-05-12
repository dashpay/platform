//! Found-022 — `AssetLockBuilder::build` marks the change-pool index used
//! before `build_asset_lock` can fail, contradicting the doc-comment guarantee.
//!
//! **Spec**: `tests/e2e/TEST_SPEC.md` (§ Found bugs → Found-022).
//! **Upstream defect site**:
//!   `key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs`
//!   (`set_funding` calls `next_change_address(..., add_to_state = true)` before
//!   `build_signed` can fail).
//! **Pinned status**: RED-BY-DESIGN — pure unit test; pins upstream bug until fix lands.
//!
//! ## Bug shape
//!
//! The doc-comment on `build_asset_lock` says:
//!
//! > The transaction is built first, and keys are only derived after a successful
//! > build — so no addresses are consumed if the build fails.
//!
//! This guarantee applies to the credit-key derivation, NOT to the BIP-44 change
//! address. Inside `TransactionBuilder::set_funding` the change address is obtained
//! via `next_change_address(..., add_to_state = true)` — the `true` argument marks
//! that index as used immediately.
//!
//! If the subsequent `build_signed` call fails (e.g. the wallet has no UTXOs so
//! coin selection cannot run), the change-pool `highest_used` index has already
//! advanced, silently drifting the pool even though no transaction was built. A
//! follow-up `build_asset_lock` call skips the already-consumed index.
//!
//! ## What this test pins
//!
//! The structural invariant:
//!
//!   * Before a failed build attempt, `internal_addresses.highest_used` is `None`
//!     (no change address has ever been used in a fresh wallet).
//!   * After a failed build (coin-selection error), the invariant MUST still hold:
//!     `highest_used == None` — the pool was not advanced.
//!   * Counter-assertion (today's buggy behaviour): `highest_used` is `Some(0)`
//!     after the failed build — the index was consumed despite no tx being built.
//!
//! The test drives `ManagedWalletInfo::build_asset_lock` directly through its
//! public API with a wallet that has no UTXOs (causing coin selection to fail),
//! then inspects the BIP44-account-0 internal-address pool via the public
//! `standard_bip44_accounts` field and `ManagedAccountTrait::managed_account_type()`.
//! No SPV harness, no network connection required.
//!
//! ## Test lifecycle
//!
//! **Today (bug present)**: after the failed build, `internal_addresses.highest_used`
//! is `Some(0)` — the pool drifted. The assertion fires (red).
//!
//! **After upstream fix**: `next_change_address` is called with `add_to_state = false`
//! (peek mode) and the index is only committed once the build succeeds; OR the change
//! address is derived after the successful build. Either way, `highest_used` remains
//! `None` after a failed build and the assertion passes. This test must be updated
//! alongside the fix.
//!
//! ## Why no live network
//!
//! The bug lives entirely in the eager `add_to_state = true` call at build time.
//! A wallet with zero UTXOs reliably triggers coin-selection failure without any
//! broadcast or chain interaction.

use key_wallet::account::ManagedAccountTrait;
use key_wallet::dashcore::{ScriptBuf, TxOut};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::{ManagedAccountType, Network, Wallet};

/// Build a single credit-output funding spec — the minimum required to exercise
/// the full `build_asset_lock` path (past the empty-outputs guard and into the
/// transaction-building stage where coin selection happens).
fn one_credit_output(amount: u64) -> Vec<CreditOutputFunding> {
    vec![CreditOutputFunding {
        output: TxOut {
            value: amount,
            // Minimal P2PKH-shaped script; the builder only reads `value`.
            script_pubkey: ScriptBuf::from(vec![
                0x76, 0xa9, 0x14, // OP_DUP OP_HASH160 PUSH20
                0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
                0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x88, 0xac, // OP_EQUALVERIFY OP_CHECKSIG
            ]),
        },
        funding_type: AssetLockFundingType::AssetLockAddressTopUp,
        identity_index: 0,
    }]
}

/// Returns the `highest_used` index of the BIP44 account-0 internal (change)
/// address pool, or `None` if no change address has been consumed yet.
fn change_pool_highest_used(info: &ManagedWalletInfo) -> Option<u32> {
    let account = info.accounts.standard_bip44_accounts.get(&0)?;
    if let ManagedAccountType::Standard {
        internal_addresses, ..
    } = account.managed_account_type()
    {
        internal_addresses.highest_used
    } else {
        None
    }
}

/// Bug-pin for Found-022.
///
/// **RED today**: after a failed `build_asset_lock` (coin-selection error) the
/// BIP44-account-0 change-pool `highest_used` is `Some(0)` — one index was
/// silently consumed despite no transaction being produced.
///
/// **GREEN after fix**: `next_change_address` is deferred until after a successful
/// build; `highest_used` remains `None` after a failed call.
#[ignore = "Found-022 bug pin — pins upstream bug in \
            key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs \
            (set_funding calls next_change_address with add_to_state=true before \
            build_signed, contradicting the doc-comment guarantee); \
            pure unit test (no live network); \
            run with `cargo test -- --ignored`"]
#[tokio_shared_rt::test(shared)]
async fn found_022_asset_lock_builder_consumes_change_index_on_failure() {
    // ── 1. Create a fresh wallet with default accounts but NO UTXOs ─────
    //
    // `WalletAccountCreationOptions::Default` creates BIP44 account 0 (among
    // others). The account's UTXO set is empty — coin selection must fail.
    let wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default)
        .expect("wallet creation must succeed");
    // Birth height 0: this is a fresh test wallet with no on-chain history.
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);

    // ── 2. Record the change-pool state before the build attempt ────────
    //
    // In a fresh wallet with no prior transactions, no change address has
    // ever been handed out. `highest_used` must be `None`.
    let before = change_pool_highest_used(&info);
    assert_eq!(
        before, None,
        "pre-condition: fresh wallet must have highest_used == None in the change pool"
    );

    // ── 3. Attempt a build that must fail at coin selection ─────────────
    //
    // The wallet has zero UTXOs, so `build_signed` (which calls coin selection
    // internally via `set_funding`) cannot fund even a small output. The
    // doc-comment guarantees that no addresses are consumed on failure — this
    // is the contract we pin.
    let result = info
        .build_asset_lock(
            &wallet,
            0, // BIP44 account 0
            one_credit_output(100_000),
            1_000, // fee_per_kb (duffs)
        )
        .await;

    // Build must have failed (no UTXOs to select).
    assert!(
        result.is_err(),
        "pre-condition: build must fail on an empty UTXO set; got Ok(_)"
    );

    // ── 4. Assert the change-pool was not advanced ──────────────────────
    //
    // This assertion FAILS today (bug present):
    //   `highest_used` is `Some(0)` after the failed build — the pool drifted
    //   because `set_funding` called `next_change_address(..., true)` before
    //   `build_signed` could report a coin-selection error.
    //
    // After the upstream fix:
    //   `highest_used` is still `None` — the failed build consumed nothing.
    let after = change_pool_highest_used(&info);
    assert_eq!(
        after, None,
        "Found-022 (RED-by-design): change-pool highest_used is {:?} after a failed \
         build_asset_lock — the BIP44-account-0 internal address pool was silently \
         advanced even though no transaction was produced. \
         TransactionBuilder::set_funding calls next_change_address(..., add_to_state=true) \
         before build_signed; if coin selection fails, the consumed index is never reclaimed. \
         Fix: call next_change_address with add_to_state=false (peek), then commit the index \
         only after build_signed succeeds; or move change-address derivation to after the \
         successful build. \
         See TEST_SPEC.md Found-022.",
        after
    );
}
