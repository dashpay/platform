//! SH-009 — Funded zero-amount operations reject before Orchard proving.
//! Transfer, unshield and withdraw remain RED until their zero guards land.

use std::time::Duration;

use platform_wallet::error::PlatformWalletError;
use platform_wallet::wallet::shielded::{ShieldedStore, SubwalletId};

use crate::framework::prelude::*;
use crate::framework::shielded::{
    bind_shielded, shielded_default_address_43, shielded_prover, teardown_sweep_shielded,
    wait_for_shielded_balance, NoProofProver,
};
use crate::framework::wait::{
    wait_for_address_balance_chain_confirmed_n, CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
};

const SHIELD_AMOUNT: u64 = 1_120_000_000;
const FUNDING_CREDITS: u64 = 2_400_000_000;
const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_009_zero_amount_rejected() {
    let s = setup().await.expect("e2e setup failed");
    let handle = bind_shielded(&s.test_wallet, &[0, 1], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let funding = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("funding address");
    s.ctx
        .bank()
        .fund_address(&funding, FUNDING_CREDITS)
        .await
        .expect("fund real note");
    wait_for_address_balance_chain_confirmed_n(
        s.ctx.sdk(),
        &funding,
        FUNDING_CREDITS,
        CHAIN_CONFIRMED_CONSECUTIVE_SUCCESSES,
        STEP_TIMEOUT,
    )
    .await
    .expect("funding observed");
    s.test_wallet
        .sync_balances()
        .await
        .expect("pre-shield sync");
    let pw = s.test_wallet.platform_wallet();
    pw.shielded_shield_from_account(
        &handle.coordinator,
        0,
        0,
        SHIELD_AMOUNT,
        s.test_wallet.address_signer(),
        shielded_prover(),
    )
    .await
    .expect("fund shielded note");
    wait_for_shielded_balance(&s.test_wallet, &handle, 0, SHIELD_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("spendable note observed");
    let id = SubwalletId::new(s.test_wallet.id(), 0);
    let before = {
        let store = handle.coordinator.store().read().await;
        store.get_unspent_notes(id).expect("unspent notes")
    };
    assert_eq!(
        before.len(),
        1,
        "zero guards must be tested with a selectable note"
    );
    let before_balances = handle.balances(&s.test_wallet).await.expect("balances");
    let recipient = shielded_default_address_43(&s.test_wallet, 1)
        .await
        .expect("recipient");
    let destination = funding.to_bech32m_string(s.ctx.bank().network());
    let core = s
        .ctx
        .bank()
        .primary_core_receive_address()
        .await
        .expect("Core address")
        .to_string();
    let seed = s.test_wallet.seed_bytes();
    let prover = &NoProofProver;

    assert_zero_error(
        pw.shielded_shield_from_account(
            &handle.coordinator,
            0,
            0,
            0,
            s.test_wallet.address_signer(),
            prover,
        )
        .await,
    );
    assert_zero_error(
        pw.shielded_transfer_to(
            &handle.coordinator,
            &seed,
            0,
            &recipient,
            0,
            [0; 36],
            prover,
        )
        .await,
    );
    assert_zero_error(
        pw.shielded_unshield_to(&handle.coordinator, &seed, 0, &destination, 0, prover)
            .await,
    );
    assert_zero_error(
        pw.shielded_withdraw_to(&handle.coordinator, &seed, 0, &core, 0, 1, prover)
            .await,
    );

    assert_eq!(
        before_balances,
        handle
            .balances(&s.test_wallet)
            .await
            .expect("balances after rejection")
    );
    {
        let store = handle.coordinator.store().read().await;
        let all = store.get_all_notes(id).expect("all notes");
        let selectable = store.get_unspent_notes(id).expect("unreserved notes");
        // get_unspent_notes excludes pending reservations as well as spent notes.
        assert_eq!(all.len(), before.len());
        assert_eq!(
            selectable.len(),
            before.len(),
            "zero requests must not leave reservations"
        );
        for (original, current) in before.iter().zip(&selectable) {
            assert_eq!(original.nullifier, current.nullifier);
            assert_eq!(original.note_data, current.note_data);
            assert_eq!(original.value, current.value);
            assert!(!current.is_spent);
        }
    }
    let bank = s
        .ctx
        .bank()
        .primary_receive_address()
        .to_bech32m_string(s.ctx.bank().network());
    teardown_sweep_shielded(&s.test_wallet, &handle, &bank).await;
    s.teardown().await.expect("teardown");
}

fn assert_zero_error(result: Result<(), PlatformWalletError>) {
    assert!(
        matches!(&result, Err(PlatformWalletError::ShieldedBuildError(message))
        if message == "amount must be > 0"),
        "expected exact zero-amount guard: {result:?}"
    );
}
