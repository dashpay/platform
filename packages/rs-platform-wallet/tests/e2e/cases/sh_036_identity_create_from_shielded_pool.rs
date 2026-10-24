//! SH-036 — IdentityCreateFromShieldedPool (Type 20): create a brand-new
//! Platform identity funded directly from the shielded pool.
//! Spec: `tests/e2e/TEST_SPEC.md` §3 "### Shielded (SH)" → SH-036.
//! Priority: P1.
//!
//! Shields `DENOMINATION` into Orchard account 0, retains transparent headroom,
//! then drives
//! `PlatformWallet::shielded_identity_create_from_pool` to spend the pool note
//! and register a new identity. The proof-verified `Identity::fetch` is the
//! verdict — the test asserts on the resulting on-chain STATE, not on broadcast
//! success.
//!
//! `DENOMINATION` is read at runtime from the protocol's versioned
//! exit-denomination set; never hardcoded.
//!
//! Every assertion is fatal, including the funding-note replay check. A bank
//! below this case's computed budget (denomination + client fee reserve +
//! predicted shield fee) is an explicit
//! `E2E-SKIP`, never a silent green.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dash_sdk::query_types::AddressInfo;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Identity, IdentityPublicKey, Purpose, SecurityLevel};
use dpp::prelude::Identifier;
use dpp::shielded::compute_minimum_shielded_fee;
use dpp::shielded::compute_shielded_identity_create_fee;
use dpp::shielded::compute_shielded_unshield_fee;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use platform_wallet::wallet::shielded::operations::shield_fee_reserve_credits;

use crate::framework::prelude::*;
use crate::framework::shielded::{
    assert_adv_rejected, bind_shielded, broadcast_raw, build_unshield_st_against_notes,
    shielded_prover, teardown_sweep_shielded, unspent_notes, wait_for_shielded_balance,
};
use crate::framework::signer::{derive_identity_key, SeedBackedIdentitySigner};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

/// DIP-9 identity-registration slot the new identity occupies. Distinct from
/// the bank identity's slot; a fresh-seed wallet has nothing else here.
const IDENTITY_INDEX: u32 = 0;

/// Per-step wait deadline.
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_036_identity_create_from_shielded_pool() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let prover = shielded_prover();
    let network = s.ctx.bank().network();

    // The exit denomination is a protocol-versioned fixed set; pick its
    // smallest member to minimise funding. Read from the live SDK version, not
    // hardcoded — the consensus check uses this same set.
    let denomination = *s
        .ctx
        .sdk()
        .version()
        .drive_abci
        .validation_and_processing
        .event_constants
        .shielded_identity_create_denominations
        .iter()
        .min()
        .expect("protocol must define at least one shielded identity-create denomination");
    // Transparent budget, computed from the live version: the denomination,
    // the client's fee reserve withheld on input 0, and the predicted 2-action
    // shield fee on top.
    let version = s.ctx.sdk().version();
    let fee_reserve = shield_fee_reserve_credits(version).expect("shield fee reserve");
    let shield_fee = compute_minimum_shielded_fee(2, version).expect("predicted shield fee");
    let funding_credits = denomination + fee_reserve + shield_fee;

    // Case-specific funding gate. A drained live bank is a legitimate reason
    // not to run, but the skip must be loud (E2E-SKIP, the token-suite
    // convention) and keyed to THIS case's budget, not the ~88.8B token floor.
    s.ctx
        .bank()
        .sync_balances()
        .await
        .expect("bank balance refresh");
    let bank_credits = s.ctx.bank().effective_platform_credits().await;
    if bank_credits < funding_credits {
        let refill = s
            .ctx
            .bank()
            .primary_receive_address()
            .to_bech32m_string(network);
        tracing::warn!(
            target: "platform_wallet::e2e::cases::sh_036",
            available = bank_credits,
            required = funding_credits,
            denomination,
            fee_reserve,
            shield_fee,
            refill_address = %refill,
            "E2E-SKIP: sh_036 did NOT run — bank Platform balance {bank_credits} below this \
             case's required {funding_credits} (denomination + fee reserve + shield fee); \
             this is a SKIP reporting PASS, not a verified pass"
        );
        eprintln!(
            "E2E-SKIP: sh_036 did NOT run (available {bank_credits} < required {funding_credits}); refill {refill}"
        );
        return;
    }

    // --- 1. Fund a fresh transparent address with the computed budget and
    //        wait until the chain-confirmed view sees it. ---
    let funding_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive funding address");
    s.ctx
        .bank()
        .fund_address(&funding_addr, funding_credits)
        .await
        .expect("bank.fund_address");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &funding_addr,
        funding_credits,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("funding never observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");

    // Bind account 0 and shield the denomination, retaining transparent headroom.
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    // Shield only `denomination`, not `funding_credits`. `select_shield_inputs`
    // withholds `shield_fee_reserve_credits` from the fee-bearing input to
    // guarantee the shield fee can be paid from unshielded balance. Asking to
    // shield the full `funding_credits` leaves no room for the reserve and hits
    // the accumulated_claim < amount guard in `select_shield_inputs`. The
    // reserve + shield fee stay transparent to cover the fee.
    s.test_wallet
        .platform_wallet()
        .shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            denomination,
            s.test_wallet.address_signer(),
            prover,
        )
        .await
        .expect("shield_from_account");
    let pool_before =
        wait_for_shielded_balance(&s.test_wallet, &handle, 0, denomination, STEP_TIMEOUT)
            .await
            .expect("shielded balance never reached the denomination");

    let funding_notes = unspent_notes(&s.test_wallet, &handle, 0)
        .await
        .expect("capture funding notes");
    assert_eq!(funding_notes.len(), 1);

    // --- 3. Build the new identity's key set: MASTER + HIGH + TRANSFER +
    //        CRITICAL, exactly as the address-funded id_* path does. Convert
    //        each to its `IdentityPublicKeyInCreation` form via the same
    //        `(&key).into()` the FFI create path uses. ---
    let seed = s.test_wallet.seed_bytes();
    let key_specs = [
        (0u32, Purpose::AUTHENTICATION, SecurityLevel::MASTER),
        (1, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
        (2, Purpose::TRANSFER, SecurityLevel::CRITICAL),
        (3, Purpose::AUTHENTICATION, SecurityLevel::CRITICAL),
    ];
    let public_keys: Vec<(IdentityPublicKey, IdentityPublicKeyInCreation)> = key_specs
        .iter()
        .map(|&(key_index, purpose, security_level)| {
            let key = derive_identity_key(
                &seed,
                network,
                IDENTITY_INDEX,
                key_index,
                purpose,
                security_level,
            )
            .expect("derive_identity_key");
            let in_creation: IdentityPublicKeyInCreation = (&key).into();
            (key, in_creation)
        })
        .collect();
    let submitted_keys: Vec<IdentityPublicKey> =
        public_keys.iter().map(|(key, _)| key.clone()).collect();
    let identity_signer = SeedBackedIdentitySigner::new(&seed, network, IDENTITY_INDEX)
        .expect("SeedBackedIdentitySigner");

    // A wallet address is the mandatory fallback recipient if Platform rejects
    // the create after the proof; the happy path never sends here.
    let failure_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive fallback failure address");

    // --- 4. Create the identity from the shielded pool (Type 20). ---
    let identity_id: Identifier = s
        .test_wallet
        .platform_wallet()
        .shielded_identity_create_from_pool(
            &handle.coordinator,
            &seed,
            0,
            IDENTITY_INDEX,
            public_keys,
            denomination,
            failure_addr,
            &identity_signer,
            prover,
        )
        .await
        .expect("shielded_identity_create_from_pool");

    // A1: a non-nil identifier came back.
    assert_ne!(
        identity_id.to_buffer(),
        [0u8; 32],
        "A1: created identity id must be non-nil"
    );

    // A2 (THE verdict): the identity is visible on chain via the proof-verified
    // fetch — the verdict is on state, not on broadcast success.
    let on_chain = Identity::fetch(s.ctx.sdk(), identity_id)
        .await
        .expect("Identity::fetch")
        .expect("A2: identity must be visible on chain (proof-verified)");
    assert_eq!(
        on_chain.id(),
        identity_id,
        "A2: fetched identity id must match the created id"
    );

    // A3: the on-chain key set matches the submitted set (count + each key's
    // purpose / security level / data).
    let on_chain_keys = on_chain.public_keys();
    assert_eq!(
        on_chain_keys.len(),
        submitted_keys.len(),
        "A3: on-chain key count {} must equal submitted {}",
        on_chain_keys.len(),
        submitted_keys.len()
    );
    for submitted in &submitted_keys {
        let matched = on_chain_keys
            .get(&submitted.id())
            .unwrap_or_else(|| panic!("A3: submitted key id {} missing on chain", submitted.id()));
        assert_eq!(
            matched.purpose(),
            submitted.purpose(),
            "A3: key {} purpose mismatch",
            submitted.id()
        );
        assert_eq!(
            matched.security_level(),
            submitted.security_level(),
            "A3: key {} security level mismatch",
            submitted.id()
        );
        assert_eq!(
            matched.data(),
            submitted.data(),
            "A3: key {} data mismatch",
            submitted.id()
        );
    }

    // A4: the shielded pool dropped by EXACTLY the denomination. The fee is
    // carved from the denomination at execution. Headroom stayed transparent;
    // the single denomination note is fully consumed.
    handle.sync().await;
    let pool_after = handle
        .balances(&s.test_wallet)
        .await
        .expect("post-create shielded_balances")
        .get(&0)
        .copied()
        .unwrap_or(0);
    assert_eq!(
        pool_before - pool_after,
        denomination,
        "A4: shielded pool must drop by exactly the denomination ({denomination}); \
         before={pool_before} after={pool_after}"
    );

    // A5: preserve the fee-tolerant identity credit bound.
    let identity_balance = on_chain.balance();
    // The create spends the single note produced by the one shield above.
    // Mirror the builder's `spends.len().max(2)` action-padding floor
    // (identity_create_from_shielded_pool.rs) instead of a bare literal, so
    // this predictor doesn't drift if the padding floor ever changes.
    let spend_note_count: usize = 1;
    let num_actions = spend_note_count.max(2);
    let predicted_fee = compute_shielded_identity_create_fee(
        num_actions,
        submitted_keys.len(),
        s.ctx.sdk().version(),
    )
    .expect("compute_shielded_identity_create_fee");
    assert!(
        identity_balance > 0 && identity_balance <= denomination,
        "A5: identity balance {identity_balance} must be in (0, {denomination}]"
    );

    // A6: rebuild against the captured funding note, bypassing local selection.
    // An empty local pool cannot make this backend replay check pass.
    let replay_destination = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("replay destination");
    let replay_st = build_unshield_st_against_notes(
        &s.test_wallet,
        &handle,
        0,
        &replay_destination,
        1_000_000,
        compute_shielded_unshield_fee(2, s.ctx.sdk().version()).expect("unshield fee"),
        &funding_notes,
    )
    .await
    .expect("build replay of Type-20 funding note");
    let replay = broadcast_raw(s.ctx.sdk(), &replay_st).await;
    assert_adv_rejected(
        s.ctx.sdk(),
        "SH-036/replay",
        &replay,
        &replay_st,
        Duration::from_secs(120),
        &[40901],
    )
    .await;
    assert_eq!(
        AddressInfo::fetch(s.ctx.sdk(), replay_destination)
            .await
            .expect("replay destination balance")
            .map_or(0, |info| info.balance),
        0
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::sh_036",
        identity_id = %identity_id,
        denomination,
        identity_balance,
        predicted_fee,
        pool_before,
        pool_after,
        "SH-036 snapshot"
    );

    // --- Teardown: the identity is permanent; only sweep residual shielded
    //     balance (the change note) back to the bank. ---
    let bank_addr = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(network);
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank_addr).await;
    s.teardown().await.expect("teardown");
}
