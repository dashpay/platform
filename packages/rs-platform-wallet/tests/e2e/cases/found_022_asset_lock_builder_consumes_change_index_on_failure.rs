//! Found-022 — `AssetLockBuilder::build` performs change-address derivation work
//! before `build_asset_lock` can fail, bumping `monitor_revision` on the
//! BIP-44-account-0 funds account even though no transaction is produced.
//!
//! **Spec**: `tests/e2e/TEST_SPEC.md` (§ Found bugs → Found-022).
//! **Upstream defect site**:
//!   `key-wallet/src/wallet/managed_wallet_info/transaction_builder.rs:79-83`
//!   at resolved rev `5313086` — `TransactionBuilder::set_funding` calls
//!   `funds_acc.next_change_address(..., add_to_state = true)` BEFORE
//!   `build_signed` can run coin selection.
//! **Tracking issue**: dashpay/rust-dashcore#764.
//!
//! **Pinned status**: RED-BY-DESIGN — pure unit test; pins upstream bug until fix lands.
//!
//! ## Bug shape
//!
//! The doc-comment on `build_asset_lock` says:
//!
//! > The transaction is built first, and keys are only derived after a successful
//! > build — so no addresses are consumed if the build fails.
//!
//! `TransactionBuilder::set_funding` violates this. It calls
//! `next_change_address(..., add_to_state = true)` eagerly, which delegates to
//! `ManagedCoreFundsAccount::next_change_address`. That method always calls
//! `self.keys.bump_monitor_revision()` before returning
//! (`key-wallet/src/managed_account/managed_core_funds_account.rs:540`),
//! regardless of whether a transaction is ever produced.
//!
//! ## Why this is the right pin
//!
//! Three other tempting assertions don't bite under realistic test setup:
//!
//! 1. `internal_addresses.highest_used == None` — `highest_used` is only
//!    mutated by `mark_used` / `mark_index_used` / `scan_for_usage`, none of
//!    which are on the eager-derivation path. Holds in both bug-present and
//!    bug-fixed states. (This was the original v47 pin; it had no bite.)
//! 2. `internal_addresses.addresses.is_empty()` —
//!    `WalletAccountCreationOptions::Default` pre-populates the internal pool
//!    with a full gap-limit window (30 derived addresses at indices 0..=29).
//!    The pool is never empty.
//! 3. Any equality check on `(addresses.len(), highest_generated)` —
//!    `AddressPool::next_unused` (`address_pool.rs:521-540`) first scans
//!    `0..=highest_generated` for an already-generated unused entry and
//!    returns it without mutating state. With 30 unused gap-limit entries
//!    present, the eager call short-circuits before
//!    `generate_address_at_index` runs. So `addresses` and
//!    `highest_generated` are unchanged in both states.
//!
//! The single observable side-effect of the eager call is the unconditional
//! `bump_monitor_revision()` on the funds account. That counter is the only
//! footprint of the bug visible from a fresh-wallet test, and it pins
//! deterministically: today (bug present) `monitor_revision` bumps `0 -> 1`
//! across the failed build; after upstream fix that defers
//! `next_change_address` past `build_signed`, the counter stays put.
//!
//! ## What this test pins
//!
//! Snapshot `account.monitor_revision()` immediately before
//! `build_asset_lock`, then assert it is **unchanged** after the build fails
//! at coin selection. Snapshot-and-compare (rather than asserting against a
//! literal) absorbs any incidental bumps from unrelated setup paths.
//!
//! The test drives `ManagedWalletInfo::build_asset_lock` directly through its
//! public API with a wallet that has no UTXOs, then reads
//! `monitor_revision()` via the public `ManagedAccountTrait`. No SPV harness,
//! no network connection required.
//!
//! ## Test lifecycle
//!
//! **Today (bug present)**: the failed build bumps `monitor_revision` by 1
//! (the eager `next_change_address` call always bumps). The assertion fires.
//!
//! **After upstream fix**: `next_change_address` is moved past `build_signed`
//! (or `set_funding` learns to peek without bumping). The failed build path
//! makes no funds-account mutation; `monitor_revision` is unchanged. The
//! assertion passes. This test must be updated alongside the fix.
//!
//! ## Why no live network
//!
//! The bug lives entirely in the eager call at build time. A wallet with zero
//! UTXOs reliably triggers coin-selection failure (`NoUtxosAvailable`) without
//! any broadcast or chain interaction.

use key_wallet::account::ManagedAccountTrait;
use key_wallet::dashcore::{ScriptBuf, TxOut};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::{
    AssetLockFundingType, CreditOutputFunding,
};
use key_wallet::wallet::managed_wallet_info::transaction_building::AccountTypePreference;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::{Network, Wallet};

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

/// Reads `monitor_revision` from the BIP-44-account-0 funds account.
///
/// This counter is bumped by every funds-mutating call on the underlying
/// `ManagedCoreFundsAccount` — including `next_change_address`, which the
/// upstream defect calls eagerly inside `TransactionBuilder::set_funding`.
fn bip44_account_0_monitor_revision(info: &ManagedWalletInfo) -> u64 {
    info.accounts
        .standard_bip44_accounts
        .get(&0)
        .expect("BIP-44 account 0 must exist on a default-options wallet")
        .monitor_revision()
}

/// Bug-pin for Found-022.
///
/// **RED today**: after a failed `build_asset_lock` (coin-selection error) the
/// BIP-44-account-0 `monitor_revision` has bumped by one — the only observable
/// footprint of `TransactionBuilder::set_funding` calling
/// `next_change_address(..., add_to_state=true)` eagerly before `build_signed`
/// could report `NoUtxosAvailable`.
///
/// **GREEN after fix**: `next_change_address` is deferred until after a
/// successful build (or `set_funding` learns to peek without mutating);
/// `monitor_revision` is unchanged on the failure path.
#[tokio_shared_rt::test(shared)]
async fn found_022_asset_lock_builder_consumes_change_index_on_failure() {
    // ── 1. Create a fresh wallet with default accounts but NO UTXOs ─────
    //
    // `WalletAccountCreationOptions::Default` creates BIP44 account 0 (among
    // others) and pre-populates its internal-address pool with a full
    // gap-limit window. The account's UTXO set is empty — coin selection
    // must fail.
    let wallet = Wallet::new_random(Network::Testnet, WalletAccountCreationOptions::Default)
        .expect("wallet creation must succeed");
    // Birth height 0: this is a fresh test wallet with no on-chain history.
    let mut info = ManagedWalletInfo::from_wallet(&wallet, 0);

    // ── 2. Snapshot monitor_revision before the build attempt ───────────
    //
    // Snapshot-and-compare absorbs any incidental bumps from earlier setup
    // and lets us pin the diff caused solely by `build_asset_lock`.
    let revision_before = bip44_account_0_monitor_revision(&info);

    // ── 3. Attempt a build that must fail at coin selection ─────────────
    //
    // The wallet has zero UTXOs, so `build_signed` (which calls coin selection
    // internally via `set_funding`) cannot fund even a small output. The
    // doc-comment guarantees that no addresses are consumed on failure — this
    // is the contract we pin.
    let result = info
        .build_asset_lock(
            &wallet,
            // Single explicit source: BIP-44 account 0, the funds account this
            // case snapshots. `drain = false` keeps the caller-supplied credit
            // output value, matching the pre-#944 single-account behaviour.
            &[AccountTypePreference::BIP44],
            0, // BIP44 account 0
            one_credit_output(100_000),
            1_000, // fee_per_kb (duffs)
            false, // drain
        )
        .await;

    // Build must have failed (no UTXOs to select).
    assert!(
        result.is_err(),
        "pre-condition: build must fail on an empty UTXO set; got Ok(_)"
    );

    // ── 4. Assert the funds account was not mutated ─────────────────────
    //
    // This assertion FAILS today (bug present):
    //   `monitor_revision` has advanced by one — `set_funding` called
    //   `next_change_address(..., add_to_state=true)`, which always invokes
    //   `bump_monitor_revision()` on the funds account
    //   (`managed_core_funds_account.rs:540`) before `build_signed` could
    //   report `NoUtxosAvailable`.
    //
    // After the upstream fix:
    //   `monitor_revision` is unchanged — no derivation work runs on the
    //   failure path.
    let revision_after = bip44_account_0_monitor_revision(&info);
    assert_eq!(
        revision_after, revision_before,
        "Found-022 (RED-by-design): BIP-44-account-0 monitor_revision advanced from \
         {revision_before} to {revision_after} across a failed build_asset_lock — the \
         funds account was mutated even though no transaction was produced. \
         TransactionBuilder::set_funding (transaction_builder.rs:79-83, rev 5313086) calls \
         funds_acc.next_change_address(..., add_to_state=true) BEFORE build_signed runs \
         coin selection. That call unconditionally invokes \
         self.keys.bump_monitor_revision() (managed_core_funds_account.rs:540) regardless \
         of whether a transaction is ever produced. With the gap-limit window already \
         pre-populated, `AddressPool::next_unused` short-circuits to an existing entry \
         without mutating the pool — so the monitor-revision bump is the only observable \
         footprint of this defect from a fresh-wallet test, but it is deterministic and \
         load-bearing for any consumer that watches monitor_revision for \"did the account \
         change?\" signaling. \
         Fix: defer next_change_address past build_signed; or make set_funding peek \
         without bumping. See TEST_SPEC.md Found-022."
    );
}
