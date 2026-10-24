//! SH-030 — Client recipient decoding and network mismatch checks.
//! The wire PlatformAddress contains a type and hash, not an HRP/network;
//! a backend cross-network-HRP check is therefore inapplicable. Own-address
//! unshield and transfers between own accounts are covered by SH-002/006.

#![cfg(feature = "shielded")]

use dashcore::Network;
use dpp::address_funds::PlatformAddress;
use platform_wallet::error::PlatformWalletError;

use crate::framework::prelude::*;
use crate::framework::shielded::{adversarial_enabled, bind_shielded, shielded_prover};

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn sh_030_cross_network_recipient() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    if !adversarial_enabled() {
        tracing::info!(
            target: "platform_wallet::e2e::cases::sh_030",
            "PLATFORM_WALLET_E2E_SHIELDED_ADVERSARIAL set to a falsy value — abuse case opted out (no-op pass)"
        );
        return;
    }

    let s = setup().await.expect("e2e setup failed");
    let prover = shielded_prover();
    let handle = bind_shielded(&s.test_wallet, &[0], &s.ctx.workdir)
        .await
        .expect("bind_shielded");
    let pw = s.test_wallet.platform_wallet();

    // (a) Wrong-network HRP: a mainnet `dash1…` platform address on a
    // testnet wallet must be rejected with a typed network-mismatch /
    // parse error BEFORE any proof build.
    let mainnet_hrp = PlatformAddress::P2pkh([0x11; 20]).to_bech32m_string(Network::Mainnet);
    let wrong_net = pw
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            &mainnet_hrp,
            1_000_000,
            prover,
        )
        .await;
    assert!(
        matches!(&wrong_net, Err(PlatformWalletError::ShieldedBuildError(message))
            if message.starts_with("platform address network mismatch:")),
        "wrong-network-HRP recipient must be rejected with a typed ShieldedBuildError; \
         observed {wrong_net:?}"
    );

    // (b) Malformed bech32m: garbage must not parse.
    let malformed = "tdash1notavalidaddressxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";
    let bad = pw
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            malformed,
            1_000_000,
            prover,
        )
        .await;
    assert!(
        matches!(&bad, Err(PlatformWalletError::ShieldedBuildError(message))
            if message.starts_with("invalid platform address:")),
        "malformed recipient address must be rejected with a typed ShieldedBuildError; \
         observed {bad:?}"
    );

    // (c) Wrong-type address (a Core base58 address where a platform
    // bech32m is expected) must also fail to parse as a platform address.
    let core_typed = "yXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    let wrong_type = pw
        .shielded_unshield_to(
            &handle.coordinator,
            &s.test_wallet.seed_bytes(),
            0,
            core_typed,
            1_000_000,
            prover,
        )
        .await;
    assert!(
        matches!(&wrong_type, Err(PlatformWalletError::ShieldedBuildError(message))
            if message.starts_with("invalid platform address:")),
        "wrong-type (Core) recipient must be rejected with a typed ShieldedBuildError; \
         observed {wrong_type:?}"
    );

    // None of the above built a proof or shielded any funds, so teardown
    // is a no-op sweep.
    s.teardown().await.expect("teardown");
}
