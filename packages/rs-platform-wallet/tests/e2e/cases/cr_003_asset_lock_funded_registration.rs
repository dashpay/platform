//! CR-003 — Asset-lock-funded identity registration (full path).
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Core (CR) → CR-003).
//! Pinned status: STUB — full test body implemented, gated behind the `e2e` cargo feature
//! behind testnet env vars + bank Core funding. SPV runtime is live
//! (Task #15) and `BankWallet::send_core_to` is wired (CR-003
//! prerequisite that landed with `ID-007`). The remaining gating is a
//! pre-funded bank Core (Layer-1) receive address on testnet — the
//! address is logged at framework init under target
//! `platform_wallet::e2e::bank` and embedded in the
//! `FrameworkError::Bank` "Bank Core under-funded" message that
//! `setup_with_core_funded_test_wallet` surfaces when the floor isn't
//! met. End-to-end runs require at least
//! `TEST_WALLET_CORE_FUNDING + CORE_TX_FEE_RESERVE` duffs so the
//! initial bank → test-wallet Core send clears.
//!
//! Pins the canonical asset-lock-funded registration contract:
//! 1. `setup_with_core_funded_test_wallet` lands `TEST_WALLET_CORE_FUNDING`
//!    duffs on the test wallet's BIP-44 account 0 (visible to SPV).
//! 2. `IdentityWallet::register_identity_with_funding`
//!    with `AssetLockFunding::FromWalletBalance { amount_duffs = ASSET_LOCK_AMOUNT, account_index = 0 }`
//!    drives the unified asset-lock flow — internally calls
//!    `AssetLockManager::create_funded_asset_lock_proof` (build →
//!    broadcast → wait IS / fall back to ChainLock) and submits the
//!    `IdentityCreateTransition` against the resolved proof.
//! 3. The returned `Identity` is fetchable on Platform with a balance
//!    >= half the lock amount (post-fee deterministic threshold).
//!
//! Mirrors DET's `test_tc004_create_registration_asset_lock` in
//! `dash-evo-tool/tests/backend-e2e/core_tasks.rs`.

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::{KeyID, Purpose, SecurityLevel};
use dpp::prelude::Identity;
use platform_wallet::AssetLockFunding;

use crate::framework::prelude::*;
use crate::framework::signer::{
    derive_identity_key, SeedBackedCoreSigner, SeedBackedIdentitySigner,
};
use crate::framework::wait::{wait_for_identity_balance, wait_for_identity_visible_to_platform};

/// DIP-9 identity index used for the asset-lock registration. Slot 0
/// is canonical for "first identity on this wallet" — same convention
/// `setup_with_n_identities` uses for its `0..n` enumeration.
const IDENTITY_INDEX: u32 = 0;

/// Core (Layer-1) duffs the bank delivers to the test wallet's BIP-44
/// account 0 prior to the asset-lock build. Sized at 2 DASH (testnet)
/// to comfortably cover the lock amount + fee reserve + change UTXO
/// without forcing the operator to top up between runs. The bank's
/// `send_core_to` floor is `TEST_WALLET_CORE_FUNDING + CORE_TX_FEE_RESERVE`.
const TEST_WALLET_CORE_FUNDING: u64 = 200_000_000;

/// Amount locked into the asset-lock output (in duffs). 1 DASH on
/// testnet — well above any min-asset-lock floor and well below the
/// `TEST_WALLET_CORE_FUNDING` cap so coin selection always has change
/// to spare.
const ASSET_LOCK_AMOUNT: u64 = 100_000_000;

/// Deadline for the on-chain identity to become balance-visible after
/// the registration transition is submitted. Matches the shape used by
/// `wallet_factory::register_identity_from_addresses` (30 s).
const IDENTITY_VISIBILITY_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_003_asset_lock_funded_registration() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: bring up a test wallet pre-funded on its Core (Layer-1)
    // BIP-44 account 0. The helper waits for the SPV-observed
    // confirmed balance to reach `TEST_WALLET_CORE_FUNDING` before
    // returning, so the asset-lock builder's coin selection has a
    // confirmed UTXO available on entry.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");

    let network = s.ctx.config.network;
    let seed_bytes = s.test_wallet.seed_bytes();
    let pre_lock_core = s.test_wallet.core_balance_confirmed();
    assert!(
        pre_lock_core >= TEST_WALLET_CORE_FUNDING,
        "PRE-pin violated: setup_with_core_funded_test_wallet returned with \
         confirmed Core balance {pre_lock_core} < TEST_WALLET_CORE_FUNDING \
         {TEST_WALLET_CORE_FUNDING} — the helper's wait_for_core_balance \
         contract has been broken or the funding amount changed without \
         updating this assertion."
    );

    // Step 2: derive the identity key set the new identity will be
    // created with. Slot 0 → MASTER (mandatory signer for the
    // IdentityCreate transition itself); slot 1 → HIGH (general
    // signing); slot 2 → TRANSFER + CRITICAL (DPP enforces a TRANSFER
    // key for credit-transfer transitions). Matches the trio
    // `register_identity_from_addresses` builds for the address-funded
    // path so downstream consumers (id_003 / id_005 / dpns_001) can
    // exercise this identity uniformly with the address-funded ones.
    let master_key = derive_identity_key(
        &seed_bytes,
        network,
        IDENTITY_INDEX,
        0,
        Purpose::AUTHENTICATION,
        SecurityLevel::MASTER,
    )
    .expect("derive MASTER auth key (slot 0, key 0)");
    let high_key = derive_identity_key(
        &seed_bytes,
        network,
        IDENTITY_INDEX,
        1,
        Purpose::AUTHENTICATION,
        SecurityLevel::HIGH,
    )
    .expect("derive HIGH auth key (slot 0, key 1)");
    let transfer_key = derive_identity_key(
        &seed_bytes,
        network,
        IDENTITY_INDEX,
        2,
        Purpose::TRANSFER,
        SecurityLevel::CRITICAL,
    )
    .expect("derive TRANSFER key (slot 0, key 2)");

    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    let mut keys_map = BTreeMap::new();
    keys_map.insert(master_key.id() as u32, master_key.clone());
    keys_map.insert(high_key.id() as u32, high_key.clone());
    keys_map.insert(transfer_key.id() as u32, transfer_key.clone());

    let identity_signer = SeedBackedIdentitySigner::new(&seed_bytes, network, IDENTITY_INDEX)
        .expect("build SeedBackedIdentitySigner for identity slot 0");
    let asset_lock_signer = SeedBackedCoreSigner::new(seed_bytes, network);

    // Step 3: drive the unified asset-lock-funded registration. The
    // wallet:
    //   1. Calls AssetLockManager::create_funded_asset_lock_proof —
    //      builds the asset-lock tx, broadcasts it, waits for the
    //      InstantSend lock (or falls back to ChainLock if needed),
    //      derives the one-time private key.
    //   2. Submits the IdentityCreate transition with the resolved
    //      proof + per-key signatures via the supplied signer.
    //   3. Returns the confirmed `Identity` with its balance populated.
    let identity = s
        .test_wallet
        .platform_wallet()
        .identity()
        .register_identity_with_funding(
            AssetLockFunding::FromWalletBalance {
                amount_duffs: ASSET_LOCK_AMOUNT,
                account_index: 0,
            },
            IDENTITY_INDEX,
            keys_map,
            &identity_signer,
            &asset_lock_signer,
            None,
        )
        .await
        .expect(
            "register_identity_with_funding (CR-003 — \
             asset-lock-funded identity registration)",
        );

    let identity_id = identity.id();
    let initial_balance = identity.balance();
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_003",
        %identity_id,
        initial_balance,
        asset_lock_amount = ASSET_LOCK_AMOUNT,
        "CR-003: identity registered via asset lock"
    );

    // Step 4: assert the identity is independently fetchable on
    // Platform with a balance >= half the lock amount. The half-lock
    // threshold is a deterministic, fee-tolerant lower bound — testnet
    // chain-time fees are well below `ASSET_LOCK_AMOUNT / 2`, so this
    // round-trips even across protocol-version fee bumps without
    // pinning a brittle exact number. Identity balances are denominated
    // in credits (`dpp::fee::Credits`), the asset-lock amount in duffs;
    // the per-duff conversion factor is `CREDITS_PER_DUFF` (= 1000) per
    // dpp's `balances::credits` module.
    let expected_credits_min = ASSET_LOCK_AMOUNT.saturating_mul(CREDITS_PER_DUFF) / 2;
    let expected_credits_max = ASSET_LOCK_AMOUNT.saturating_mul(CREDITS_PER_DUFF);
    let observed_credits = wait_for_identity_balance(
        s.test_wallet.platform_wallet().sdk(),
        identity_id,
        expected_credits_min,
        IDENTITY_VISIBILITY_TIMEOUT,
    )
    .await
    .expect("identity balance (credits) reached half-lock threshold");
    assert!(
        observed_credits <= expected_credits_max,
        "POST-pin violated: observed identity balance {observed_credits} credits \
         > full asset-lock {expected_credits_max} credits \
         (= ASSET_LOCK_AMOUNT {ASSET_LOCK_AMOUNT} duffs * CREDITS_PER_DUFF \
         {CREDITS_PER_DUFF}). Registration cannot credit more than the \
         asset-lock output value (fees are subtracted, not added)."
    );

    // Step 5: wait for the identity to be visible across enough DAPI
    // replicas before the round-trip fetch. The asset-lock-funded path
    // has different proof convergence than the address-funded path —
    // `wait_for_identity_balance` above confirms credits landed, but
    // a subsequent `Identity::fetch` on a still-lagging replica returns
    // `Ok(None)`. Two consecutive successes bias toward distinct nodes
    // having replicated the identity (QA-911).
    wait_for_identity_visible_to_platform(
        s.test_wallet.platform_wallet().sdk(),
        identity_id,
        IDENTITY_VISIBILITY_TIMEOUT,
        2,
    )
    .await
    .expect("identity propagation gate cleared before round-trip fetch (QA-911)");

    let fetched = Identity::fetch(s.test_wallet.platform_wallet().sdk(), identity_id)
        .await
        .expect("Identity::fetch round-trip after registration")
        .expect("registered identity must be fetchable on platform");
    assert_eq!(
        fetched.id(),
        identity_id,
        "POST-pin violated: fetched identity id {} != registered id {}",
        fetched.id(),
        identity_id
    );
    let fetched_master = fetched
        .public_keys()
        .get(&(0_u32 as KeyID))
        .expect("fetched identity missing slot-0 (MASTER) key");
    assert_eq!(
        fetched_master.security_level(),
        SecurityLevel::MASTER,
        "POST-pin violated: slot-0 key on fetched identity is not MASTER \
         (got {:?}). The IdentityCreate transition is required to be signed \
         by a MASTER-level key at id=0 — a non-MASTER slot-0 means the \
         protocol accepted a malformed registration.",
        fetched_master.security_level()
    );

    // Step 6: assert every tracked asset lock reached a finalised state.
    //
    // Accepted: `InstantSendLocked` / `ChainLocked` (proof materialised) and
    // `Consumed`. `Consumed` is strictly stronger evidence, not a weaker
    // fallback — Platform only consumes a lock after accepting a valid
    // IS/CL proof, and `consume_asset_lock` then clears `proof` as one-shot
    // and RETAINS the entry for historical lookup. The registry is therefore
    // NOT empty after a successful run; an `assert!(tracked.is_empty())`
    // would be wrong here.
    //
    // Still failures: `Built` / `Broadcast` (never finalised) and
    // `RecoveredFromChain` (Core finality authenticated but Platform-side
    // consumption unknown — a lock the live flow did not drive to
    // completion, which is exactly what this POST-pin exists to catch).
    let tracked = s
        .test_wallet
        .platform_wallet()
        .asset_locks()
        .list_tracked_locks()
        .await;
    for lock in &tracked {
        use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
        assert!(
            matches!(
                lock.status,
                AssetLockStatus::InstantSendLocked
                    | AssetLockStatus::ChainLocked
                    | AssetLockStatus::Consumed
            ),
            "POST-pin violated: tracked asset lock {:?} is in \
             non-finalised status {:?} after register_identity_with_funding \
             completed. The unified flow must drive every lock to a \
             finalised proof state or to Consumed.",
            lock.out_point,
            lock.status
        );
    }

    s.teardown().await.expect("teardown");
}
