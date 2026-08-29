//! ID-003 — Identity-to-identity credit transfer.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-003).
//! Pinned status: Pass.
//!
//! Registers two identities via `setup_with_n_identities(2, …)` and
//! drives `transfer_credits_with_external_signer` between them.
//! Pins the per-identity balance deltas and the implied transfer
//! fee.

use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::setup_with_n_identities;
use crate::framework::wait::{wait_for_identity_balance, wait_for_identity_balance_change};

/// Credits committed to each identity. KEPT LARGER than 0.001 tDASH:
/// must stay above `IDENTITY_SWEEP_FLOOR` (50M, `cleanup.rs`) so the
/// teardown sweep recovers credits to the bank identity instead of
/// silently skipping (Marvin v32 forensics). 100M provides 50M
/// margin above floor + sweep transfer fee (~6.5M). The sender then pays
/// `TRANSFER_AMOUNT + transfer_fee` from its balance; receiver gains
/// `TRANSFER_AMOUNT`. Both end up ≥ 50M so both sweep.
const FUNDING_PER: u64 = 100_000_000;

/// Credits sent from `identity_a` to `identity_b` (0.001 tDASH).
const TRANSFER_AMOUNT: Credits = 100_000;

/// Identity-balance wait floor for the receiver after transfer
/// (post-registration balance + a fraction of the transfer amount).
const RECV_FLOOR_DELTA: u64 = TRANSFER_AMOUNT;

const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_003_identity_to_identity_credit_transfer() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let guard = setup_with_n_identities(2, FUNDING_PER)
        .await
        .expect("setup_with_n_identities(2)");

    let identity_a = &guard.identities[0];
    let identity_b = &guard.identities[1];
    assert_ne!(
        identity_a.id, identity_b.id,
        "registered identities must be distinct"
    );

    // Snapshot on-chain pre-balances. The wallet's cached balance
    // is set via `set_balance` only on the call that returns a
    // post-fee value; the on-chain fetch is the trustworthy
    // source for both sides here.
    let pre_a = Identity::fetch(guard.base.ctx.sdk(), identity_a.id)
        .await
        .expect("fetch pre A")
        .expect("identity_a visible")
        .balance();
    let pre_b = Identity::fetch(guard.base.ctx.sdk(), identity_b.id)
        .await
        .expect("fetch pre B")
        .expect("identity_b visible")
        .balance();
    assert!(
        pre_a >= TRANSFER_AMOUNT,
        "identity_a needs at least TRANSFER_AMOUNT credits (has {pre_a})"
    );

    guard
        .base
        .test_wallet
        .platform_wallet()
        .identity()
        .transfer_credits_with_external_signer(
            &identity_a.id,
            &identity_b.id,
            TRANSFER_AMOUNT,
            identity_a.signer.as_ref(),
            None,
        )
        .await
        .expect("transfer_credits_with_external_signer");

    // Wait for the receiver's on-chain balance to reflect the
    // transfer before reading post-balances.
    let post_b = wait_for_identity_balance(
        guard.base.ctx.sdk(),
        identity_b.id,
        pre_b + RECV_FLOOR_DELTA,
        STEP_TIMEOUT,
    )
    .await
    .expect("receiver balance never reached post-transfer floor");

    // Marvin QA-906 forensics: the receiver-side gate above only proves
    // that *some* replica has applied the transfer block — not that the
    // DAPI replica round-robined for the sender fetch has. The transfer
    // always charges the sender (credits debit + fee), so any change
    // clears the gate. Same shape as TK-006/007/008.
    let post_a =
        wait_for_identity_balance_change(guard.base.ctx.sdk(), identity_a.id, pre_a, STEP_TIMEOUT)
            .await
            .expect("sender balance never changed after transfer");

    // Receiver must gain exactly TRANSFER_AMOUNT — credit transfers
    // do NOT charge the receiver. The fee is paid out of the
    // sender's balance.
    assert_eq!(
        post_b,
        pre_b + TRANSFER_AMOUNT,
        "receiver must gain exactly TRANSFER_AMOUNT (pre={pre_b} post={post_b})"
    );

    // Sender lost the transfer amount plus a non-zero fee.
    assert!(
        post_a < pre_a,
        "sender balance must decrease (pre={pre_a} post={post_a})"
    );
    let sender_loss = pre_a.saturating_sub(post_a);
    assert!(
        sender_loss > TRANSFER_AMOUNT,
        "sender_loss {sender_loss} must exceed TRANSFER_AMOUNT {TRANSFER_AMOUNT} \
         (the difference is the on-chain transfer fee)"
    );
    let transfer_fee = sender_loss - TRANSFER_AMOUNT;
    assert!(
        transfer_fee > 0,
        "transfer fee must be non-zero (sender_loss={sender_loss})"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::id_003",
        identity_a = %identity_a.id,
        identity_b = %identity_b.id,
        pre_a,
        post_a,
        pre_b,
        post_b,
        transfer_fee,
        "credit-transfer snapshot"
    );

    guard.teardown().await.expect("teardown");
}
