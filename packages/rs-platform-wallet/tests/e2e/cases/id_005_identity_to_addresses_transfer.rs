//! ID-005 — Transfer credits from identity to platform addresses.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-005).
//! Pinned status: Pass.
//!
//! Registers an identity with comfortable headroom, derives a fresh
//! destination address on the test wallet, and drives
//! `transfer_credits_to_addresses_with_external_signer`.
//! Pins the destination address balance, the identity-side balance
//! delta, and the implied transfer fee.

use std::collections::BTreeMap;
use std::time::Duration;

use dash_sdk::platform::Fetch;
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::Identity;

use crate::framework::prelude::*;

/// Bank-funded credits the funding address starts with. Sized to
/// cover ID-005's 60M registration plus the bank's ReduceOutput
/// fee with comfortable headroom.
const FUNDING_CREDITS: u64 = 80_000_000;
const FUNDING_FLOOR: u64 = 70_000_000;

/// Credits the registration commits to the identity. Sized so the
/// post-registration balance comfortably covers the 20M transfer
/// plus the chain-time transfer fee.
const REGISTRATION_FUNDING: u64 = 70_000_000;

/// Credits transferred from identity to the destination address.
const TRANSFER_AMOUNT: Credits = 20_000_000;

const STEP_TIMEOUT: Duration = Duration::from_secs(60);

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_005_identity_to_addresses_transfer() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    let s = setup().await.expect("e2e setup failed");

    let funding_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive funding address");
    s.ctx
        .bank()
        .fund_address(&funding_addr, FUNDING_CREDITS)
        .await
        .expect("bank.fund_address");
    wait_for_balance(&s.test_wallet, &funding_addr, FUNDING_FLOOR, STEP_TIMEOUT)
        .await
        .expect("funding never observed");

    let registered = s
        .test_wallet
        .register_identity_from_addresses(funding_addr, REGISTRATION_FUNDING, 0)
        .await
        .expect("register_identity_from_addresses");

    let pre_balance = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch pre")
        .expect("identity visible")
        .balance();
    assert!(
        pre_balance > TRANSFER_AMOUNT,
        "identity must hold > TRANSFER_AMOUNT to fund the transfer + fee \
         (pre={pre_balance} amount={TRANSFER_AMOUNT})"
    );

    let dest_addr = s
        .test_wallet
        .next_unused_address()
        .await
        .expect("derive destination address");
    assert_ne!(
        dest_addr, funding_addr,
        "destination must differ from the funding address"
    );

    let outputs: BTreeMap<PlatformAddress, Credits> =
        std::iter::once((dest_addr, TRANSFER_AMOUNT)).collect();
    let new_balance = s
        .test_wallet
        .platform_wallet()
        .identity()
        .transfer_credits_to_addresses_with_external_signer(
            &registered.id,
            outputs,
            registered.signer.as_ref(),
            None,
        )
        .await
        .expect("transfer_credits_to_addresses_with_external_signer");

    // Cross-check the wallet-returned balance with an on-chain
    // fetch.
    let on_chain_post = Identity::fetch(s.ctx.sdk(), registered.id)
        .await
        .expect("fetch post")
        .expect("identity still visible")
        .balance();
    assert_eq!(
        on_chain_post, new_balance,
        "wallet-returned balance {new_balance} must match on-chain fetch {on_chain_post}"
    );

    let identity_loss = pre_balance.saturating_sub(on_chain_post);
    assert!(
        identity_loss > TRANSFER_AMOUNT,
        "identity loss {identity_loss} must exceed TRANSFER_AMOUNT {TRANSFER_AMOUNT} \
         (the difference is the on-chain transfer fee)"
    );
    let transfer_fee = identity_loss - TRANSFER_AMOUNT;
    assert!(
        transfer_fee > 0,
        "transfer fee must be non-zero (identity_loss={identity_loss})"
    );

    // Wait for the destination address to observe the credited
    // amount, then assert it gained exactly TRANSFER_AMOUNT.
    wait_for_balance(&s.test_wallet, &dest_addr, TRANSFER_AMOUNT, STEP_TIMEOUT)
        .await
        .expect("destination address balance never reached TRANSFER_AMOUNT");

    let balances = s.test_wallet.balances().await;
    let dest_received = balances.get(&dest_addr).copied().unwrap_or(0);
    assert_eq!(
        dest_received, TRANSFER_AMOUNT,
        "destination address must receive exactly TRANSFER_AMOUNT \
         (the fee was charged on the identity side)"
    );

    tracing::info!(
        target: "platform_wallet::e2e::cases::id_005",
        identity_id = %registered.id,
        pre_balance,
        post_balance = on_chain_post,
        transfer_fee,
        dest_received,
        "identity → addresses transfer snapshot"
    );

    s.teardown().await.expect("teardown");
}
