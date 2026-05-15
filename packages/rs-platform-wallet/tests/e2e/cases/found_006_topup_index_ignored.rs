//! Found-006 — `top_up_identity_with_funding` ignores caller-supplied
//! `topup_index`.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Found bugs → Found-006).
//! Pinned status: BUG-PIN, expected RED until upstream fix.
//!
//! ## Bug shape
//!
//! `wallet/identity/network/top_up.rs:60-106`. The doc says
//! `topup_index` is "An incrementing index distinguishing successive
//! top-ups for the same identity". The implementation prefixes the
//! parameter with `_` and the function body derives the funding key
//! path from `identity_index` alone (a `TODO(platform-wallet)` comment
//! confirms the parameter is unused). Upstream `CreditOutputFunding`
//! has no `top_up_index` field, so the routing bug cannot be fixed
//! downstream alone.
//!
//! ## QA-012 investigation (Marvin, 2026-05-12)
//!
//! The previous assertion (`assert_ne!(first_txid, second_txid)` after
//! `topup_index=0` then `topup_index=1`) was a FALSE-GREEN. It passed
//! because `next_private_key` (key-wallet managed_account_trait.rs:464)
//! advances a sequential cursor through the HD pool on every call —
//! regardless of `_topup_index`. Consecutive calls derived key[0] then
//! key[1], which are distinct, so the txids differed. The distinction
//! had nothing to do with routing `topup_index` correctly.
//!
//! ## Correct pin (per QA-012 §5)
//!
//! The real contract is: `top_up_identity_with_funding(..., topup_index=N, ...)`
//! must derive the credit-output key from HD slot N, not from the sequential
//! "next available" cursor. To observe the routing mismatch, use a
//! non-sequential `topup_index`:
//!
//! 1. Record the HD address at pool slot `TOPUP_INDEX_NONSEQ` BEFORE
//!    making any calls (the pool pre-generates `DEFAULT_SPECIAL_GAP_LIMIT`
//!    addresses, so slot `TOPUP_INDEX_NONSEQ` is already computed).
//! 2. First call: `topup_index=0` → sequential cursor advances to slot 0;
//!    both buggy and correct implementations agree.
//! 3. Second call: `topup_index=TOPUP_INDEX_NONSEQ` →
//!    - Buggy: sequential cursor advances to slot 1 → credit output = addr[1].
//!    - Correct: cursor is set from `topup_index` → credit output = addr[TOPUP_INDEX_NONSEQ].
//! 4. Assert: `actual_credit_output_address == expected_address_at_slot_TOPUP_INDEX_NONSEQ`.
//!    This FAILS today (addr[1] != addr[TOPUP_INDEX_NONSEQ]) and PASSES once
//!    upstream `CreditOutputFunding` gains `top_up_index` AND the downstream
//!    wiring is complete.
//!
//! ## FAILS UNTIL
//!
//! Upstream `key_wallet::CreditOutputFunding` gains a `top_up_index` field AND
//! `top_up_identity_with_funding` removes the `_topup_index` prefix and wires
//! the index through to `AssetLockFundingType`-based account resolution.
//!
//! Run with `cargo test -- --ignored` against a testnet bank with Core funding.

use std::time::Duration;

use dpp::identity::Identity;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::AccountType;
use platform_wallet::wallet::identity::types::funding::TopUpFundingMethod;
use platform_wallet::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};
use dash_sdk::platform::Fetch;

/// Core (Layer-1) duffs for the test wallet. Sized for two
/// back-to-back asset-lock top-ups (TOP_UP_AMOUNT × 2 + fee headroom).
const TEST_WALLET_CORE_FUNDING: u64 = 300_000_000;

/// Per-top-up asset-lock amount (duffs). Same shape as ID-002b.
const TOP_UP_AMOUNT: u64 = 100_000_000;

/// DIP-9 identity slot. Both top-ups target the same identity (this
/// IS the test).
const IDENTITY_INDEX: u32 = 0;

/// Registration funding via the cheaper address-funded path.
const REGISTRATION_FUNDING: u64 = 100_000_000;
const REGISTRATION_FUNDING_CREDITS: u64 = REGISTRATION_FUNDING + 150_000_000;

/// Per-step wait deadline.
const STEP_TIMEOUT: Duration = Duration::from_secs(180);

/// The non-sequential `topup_index` used for the second top-up call.
///
/// Must NOT equal the sequential-cursor position the second call would
/// land on (position 1 after the first call marks slot 0 used).
/// Must be within [0, DEFAULT_SPECIAL_GAP_LIMIT) = [0, 5) so the
/// expected address is pre-generated in the pool.
const TOPUP_INDEX_NONSEQ: u32 = 3;

#[ignore = "Found-006 — bug pin. EXPECTED to fail until upstream \
            `key_wallet::CreditOutputFunding` gains a `top_up_index` \
            field AND downstream `top_up_identity_with_funding` wires it \
            through. Run with `cargo test -- --ignored`; the failure mode \
            today is that the actual credit-output address for the second \
            call is addr[1] (sequential cursor), not addr[TOPUP_INDEX_NONSEQ] \
            (correct routing). See QA-012 investigation for details."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn found_006_topup_index_ignored() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");

    // Register an identity via the cheap address-funded path.
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
        &funding_addr,
        REGISTRATION_FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("register funding never observed");
    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, IDENTITY_INDEX)
        .await
        .expect("register_identity_from_addresses");
    let identity_id = registered.id;
    let _ = Identity::fetch(s.ctx.sdk(), identity_id)
        .await
        .expect("fetch pre")
        .expect("identity visible");

    // Provision the `IdentityTopUp` HD account for `IDENTITY_INDEX = 0`.
    // `add_identity_topup_account` pre-generates `DEFAULT_SPECIAL_GAP_LIMIT`
    // (= 5) addresses using the account xpub, so slots 0–4 are immediately
    // available without a private-key round-trip.  (QA-006)
    add_identity_topup_account(s.test_wallet.platform_wallet(), IDENTITY_INDEX)
        .await
        .expect("add IdentityTopUp HD account for IDENTITY_INDEX");

    // -- Pre-derive the expected address for TOPUP_INDEX_NONSEQ ---------
    //
    // The correct routing contract is: `topup_index=N` must use HD slot N.
    // Record slot `TOPUP_INDEX_NONSEQ` before any keys are consumed, so we
    // can compare it against the actual credit-output address of the second
    // call regardless of cursor advancement.
    let expected_credit_output_addr = {
        let wallet_id = s.test_wallet.platform_wallet().wallet_id();
        let wm = s
            .test_wallet
            .platform_wallet()
            .wallet_manager()
            .read()
            .await;
        let info = wm.get_wallet_info(&wallet_id).expect("wallet info present");
        let managed = info
            .core_wallet
            .accounts
            .identity_topup
            .get(&IDENTITY_INDEX)
            .expect("IdentityTopUp account must be provisioned");
        let pools = managed.managed_account_type().address_pools();
        let pool = pools
            .first()
            .expect("IdentityTopUp has exactly one address pool");
        pool.address_at_index(TOPUP_INDEX_NONSEQ)
            .unwrap_or_else(|| {
                panic!(
                    "Address at slot {TOPUP_INDEX_NONSEQ} must be pre-generated \
                     (DEFAULT_SPECIAL_GAP_LIMIT = 5 covers slots 0–4)"
                )
            })
    };

    // First top-up — topup_index = 0.
    // Sequential cursor advances from slot 0. Both buggy and correct
    // implementations agree here (0 == 0).
    s.test_wallet
        .platform_wallet()
        .identity()
        .top_up_identity_with_funding(
            &identity_id,
            TopUpFundingMethod::FundWithWallet {
                amount_duffs: TOP_UP_AMOUNT,
            },
            0,
            None,
        )
        .await
        .expect(
            "first top_up_identity_with_funding (topup_index=0) — \
             expected to succeed",
        );

    // Snapshot locks after the first call so we can isolate the second
    // call's tracked lock below.
    let first_locks = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    let first_txid_set: std::collections::HashSet<_> = first_locks
        .iter()
        .filter(|l| {
            matches!(
                l.funding_type,
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
            )
        })
        .map(|l| l.out_point.txid)
        .collect();

    // Second top-up — topup_index = TOPUP_INDEX_NONSEQ (= 3).
    //
    // CONTRACT: this call MUST derive its credit-output key from slot
    // `TOPUP_INDEX_NONSEQ`, yielding address[TOPUP_INDEX_NONSEQ].
    //
    // BUG (today): `_topup_index` is ignored; `next_private_key` advances
    // the sequential cursor to slot 1. The actual credit-output address is
    // address[1], NOT address[TOPUP_INDEX_NONSEQ]. The final assertion below
    // catches this mismatch and fails RED.
    s.test_wallet
        .platform_wallet()
        .identity()
        .top_up_identity_with_funding(
            &identity_id,
            TopUpFundingMethod::FundWithWallet {
                amount_duffs: TOP_UP_AMOUNT,
            },
            TOPUP_INDEX_NONSEQ,
            None,
        )
        .await
        .expect(
            "Found-006: second top_up_identity_with_funding \
             (topup_index=TOPUP_INDEX_NONSEQ) must succeed",
        );

    // Find the tracked lock introduced by the second call.
    let post_locks = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;

    let second_lock = post_locks
        .iter()
        .filter(|l| {
            matches!(
                l.funding_type,
                key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType::IdentityTopUp
            )
        })
        .find(|l| !first_txid_set.contains(&l.out_point.txid))
        .unwrap_or_else(|| {
            panic!(
                "Found-006: second top-up must produce a new IdentityTopUp tracked lock \
                 distinct from the first call's txids {:?}",
                first_txid_set,
            )
        });

    // Extract the credit-output address from the second asset-lock transaction.
    //
    // The credit output lives in the special transaction payload (AssetLockPayload),
    // not in `tx.output[]` — per the DIP-9 structure. Its `script_pubkey` encodes
    // the P2PKH address derived from the one-time funding key.
    let network = s.ctx.config.network;
    let actual_credit_output_addr = {
        use key_wallet::dashcore::blockdata::transaction::special_transaction::TransactionPayload;

        let payload = second_lock
            .transaction
            .special_transaction_payload
            .as_ref()
            .expect("asset-lock transaction must carry a special-transaction payload");

        let asset_lock = match payload {
            TransactionPayload::AssetLockPayloadType(p) => p,
            _ => {
                panic!("Found-006: expected AssetLockPayloadType payload, got a different variant")
            }
        };

        let credit_out = asset_lock
            .credit_outputs
            .first()
            .expect("AssetLockPayload must have at least one credit output");

        key_wallet::dashcore::Address::from_script(&credit_out.script_pubkey, network)
            .expect("credit output script must be a valid P2PKH address")
    };

    // The core assertion: the second call's credit-output address must match
    // the HD address pre-derived for slot `TOPUP_INDEX_NONSEQ`.
    //
    // TODAY (BUG): actual = address[1] (sequential cursor after first call);
    //              expected = address[TOPUP_INDEX_NONSEQ] = address[3].
    //              address[1] != address[3]  →  assertion FAILS  →  RED pin.
    //
    // AFTER FIX: `topup_index` is routed through `CreditOutputFunding`; the
    //             call derives key at slot TOPUP_INDEX_NONSEQ correctly.
    //             actual == expected  →  assertion PASSES.
    assert_eq!(
        actual_credit_output_addr, expected_credit_output_addr,
        "Found-006 (QA-012 rewrite): second top-up (topup_index={TOPUP_INDEX_NONSEQ}) \
         must derive its credit-output key from HD slot {TOPUP_INDEX_NONSEQ}, yielding \
         address[{TOPUP_INDEX_NONSEQ}] = {expected_credit_output_addr}. \
         Today the sequential cursor produces address[1] instead, which means \
         `topup_index` is being ignored. See QA-012 investigation and \
         TEST_SPEC.md Found-006 for the upstream root cause \
         (CreditOutputFunding lacks a `top_up_index` field)."
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
/// Without this, both top-up calls fail on the precondition before they
/// can even reach the `topup_index` routing path that Found-006 pins.
/// (QA-006)
async fn add_identity_topup_account(
    wallet: &std::sync::Arc<platform_wallet::PlatformWallet>,
    registration_index: u32,
) -> Result<(), PlatformWalletError> {
    let wallet_id = wallet.wallet_id();
    let mut wm = wallet.wallet_manager().write().await;
    let (kw, info) = wm
        .get_wallet_mut_and_info_mut(&wallet_id)
        .ok_or_else(|| PlatformWalletError::WalletNotFound(hex::encode(wallet_id)))?;
    kw.add_account(AccountType::IdentityTopUp { registration_index }, None)
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
