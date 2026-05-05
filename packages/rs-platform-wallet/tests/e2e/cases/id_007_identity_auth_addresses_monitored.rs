//! ID-007 — Identity-auth addresses ARE visible to SPV monitor.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-007).
//! Pinned status: FAILING — documents an open upstream issue.
//!
//! Asserts the CORRECT behavior:
//! - identity-auth addresses derived via
//!   [`derive_ecdsa_identity_auth_keypair_from_master`] ARE in
//!   [`WalletInfoInterface::monitored_addresses`].
//! - Sending Core duffs to one of those addresses INCREASES the
//!   wallet's Core balance.
//! - The wallet's UTXO set ends up holding the new UTXO.
//!
//! This test currently FAILS because rust-dashcore's
//! `WalletAccountCreationOptions::Default` does not include the
//! `BlockchainIdentities*` `AccountType` variants (closed PR
//! `dashpay/rust-dashcore#554` attempted this; closed without
//! merge). When upstream lands the fix and exposes those accounts as
//! part of `Default`, this test will start passing — and that's the
//! point: green = feature works, red = feature broken.
//!
//! DET parallel: `dash-evo-tool#692` (the follow-up issue PR
//! `dashpay/rust-dashcore#554` referenced for the DET-side
//! `spv_account_metadata()` match arm).

use std::time::Duration;

use dashcore::secp256k1::PublicKey as SecpPublicKey;
use dashcore::{Address, Network, PublicKey};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;

use crate::framework::prelude::*;

/// Funding committed to the registered identity. Modest — the
/// scenario doesn't need a fat identity, only one that exists so the
/// `identity_index = 0` slot is canonically "in use".
const REGISTRATION_FUNDING: u64 = 30_000_000;

/// Layer-1 send amount targeted at the identity-auth address. ~0.001
/// DASH; well above the dust threshold so the bank's Core path
/// doesn't reject it on amount alone, well below any per-test budget
/// concern.
const CORE_SEND_DUFFS: u64 = 100_000;

/// Window for `wait_for_core_balance` to observe the inbound UTXO at
/// confirmed depth. The waiter polls
/// [`TestWallet::core_balance_confirmed`] (see
/// `framework/wait.rs`), which only counts confirmed UTXOs. Testnet
/// block time is ~2.5 minutes; allow generous headroom for one
/// confirmation plus SPV bloom-filter propagation.
const CORE_BALANCE_CONFIRMATION_WINDOW: Duration = Duration::from_secs(300);

#[ignore = "ID-007 — pins upstream rust-dashcore#554 / blockchain-identities work; \
            currently FAILS by design until WalletAccountCreationOptions::Default \
            includes BlockchainIdentities* AccountType variants. Run with \
            `cargo test -- --ignored` expecting failure. When this test starts \
            passing, the upstream fix has landed."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_007_identity_auth_addresses_monitored() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: register one identity at slot 0 with modest funding.
    // Reuses `setup_with_n_identities` so the canonical identity-
    // funding path is exercised; the identity itself isn't load-
    // bearing in the assertions, only that slot 0 is "in use".
    let s = crate::framework::setup_with_n_identities(1, REGISTRATION_FUNDING)
        .await
        .expect("setup_with_n_identities failed");
    let identity_zero = s
        .identities
        .first()
        .expect("setup_with_n_identities returned no identities");
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_007",
        identity_id = %identity_zero.id,
        "registered slot-0 identity for ID-007"
    );

    let network = s.base.ctx.config.network;
    let seed_bytes = s.base.test_wallet.seed_bytes();

    // Derive `auth_addr` for (identity_index = 0, key_index = 0) —
    // the slot we just registered. Pure derivation; bypasses the
    // wallet's `AccountCollection` entirely. P2PKH the resulting
    // pubkey to get a Core (Layer-1) address.
    let auth_addr_zero = derive_auth_address(&seed_bytes, network, 0, 0)
        .expect("derive identity-auth address (identity_index=0, key_index=0)");

    // Negative-axis variant — same derivation at an UNREGISTERED
    // slot. Registration status is irrelevant to monitoring (the
    // derivation is pure), so the same correct-behavior assertions
    // hold: every (identity_index, key_index) pair under the DIP-9
    // identity-authentication subfeature MUST be monitored.
    let auth_addr_one = derive_auth_address(&seed_bytes, network, 1, 0)
        .expect("derive identity-auth address (identity_index=1, key_index=0)");

    // TODO(ID-007): add BLS subfeature variant once
    // `derive_*_bls_identity_auth_keypair_from_master` lands in the
    // upstream `key-wallet` API. Path:
    // `m/9'/coinType'/5'/2'/identity_index'/key_index'`. Same
    // correct-behavior assertions apply.

    // Step 3: snapshot `monitored_addresses()` BEFORE any Core send.
    // Once upstream lands the fix, both addresses MUST already be in
    // the monitored set (the bloom filter regenerates from
    // `accounts.all_accounts()` and `BlockchainIdentities*` accounts
    // are part of `WalletAccountCreationOptions::Default`).
    let monitored_before = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .monitored_addresses();
    assert!(
        monitored_before.contains(&auth_addr_zero),
        "identity-auth address (slot 0) is NOT in monitored_addresses() \
         before the Core send. Expected the SPV bloom filter to cover \
         every (identity_index, key_index) pair on the DIP-9 \
         identity-authentication subfeature path. This assertion will \
         start passing when upstream rust-dashcore exposes \
         BlockchainIdentities* AccountType variants in \
         WalletAccountCreationOptions::Default \
         (closed PR dashpay/rust-dashcore#554; DET parallel \
         dash-evo-tool#692)."
    );
    assert!(
        monitored_before.contains(&auth_addr_one),
        "identity-auth address (slot 1, unregistered) is NOT in \
         monitored_addresses(). Registration status is irrelevant — \
         the derivation is pure — so every (identity_index, key_index) \
         pair on the DIP-9 identity-authentication subfeature path \
         MUST be monitored. Tracks closed PR dashpay/rust-dashcore#554."
    );

    // Step 4: send `CORE_SEND_DUFFS` from the bank to `auth_addr_zero`
    // on Layer-1 via `BankWallet::send_core_to` (CR-003). Returns a
    // broadcast `Txid`; we wait below for confirmation via
    // `wait_for_core_balance`.
    // Use the same lock-free confirmed-balance accessor that
    // `wait_for_core_balance` polls — pinning `pre_balance + 1` against
    // the same metric the waiter compares against keeps the assertion
    // crisp.
    let pre_balance = s.base.test_wallet.core_balance_confirmed();
    let _txid = s
        .base
        .ctx
        .bank()
        .send_core_to(&auth_addr_zero, CORE_SEND_DUFFS)
        .await
        .expect("bank.send_core_to (CR-003 prerequisite)");

    // Step 5: snapshot `monitored_addresses()` AFTER the broadcast.
    // The bloom filter is regenerated from `accounts.all_accounts()`;
    // identity-auth addresses MUST still appear post-broadcast.
    let monitored_after = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .monitored_addresses();
    assert!(
        monitored_after.contains(&auth_addr_zero),
        "identity-auth address (slot 0) is NOT in monitored_addresses() \
         after the Layer-1 send. Upstream BlockchainIdentities* support \
         is required for the SPV bloom filter to cover this path \
         (rust-dashcore#554)."
    );
    assert!(
        monitored_after.contains(&auth_addr_one),
        "identity-auth address (slot 1, unregistered) is NOT in \
         monitored_addresses() after the Layer-1 send. Registration \
         status is irrelevant; every (identity_index, key_index) pair \
         on the DIP-9 identity-authentication subfeature path must be \
         monitored (rust-dashcore#554)."
    );

    // Step 6: wait UP TO `CORE_BALANCE_CONFIRMATION_WINDOW` for the
    // wallet's confirmed Core balance to reflect the inbound UTXO.
    // With the upstream fix in place, the SPV bloom filter carries
    // `auth_addr_zero` and the inbound UTXO becomes visible once
    // confirmed.
    let observed = wait_for_core_balance(
        &s.base.test_wallet,
        pre_balance + 1,
        CORE_BALANCE_CONFIRMATION_WINDOW,
    )
    .await
    .expect(
        "wait_for_core_balance timed out waiting for the inbound \
         UTXO at the identity-auth address. Either the SPV bloom \
         filter doesn't carry DIP-9 subfeature 0..3 (the current \
         upstream state — rust-dashcore#554 not merged), or the send \
         didn't confirm within the window. The test asserts the \
         CORRECT contract; failure here documents the open issue.",
    );
    tracing::info!(
        target: "platform_wallet::e2e::cases::id_007",
        observed,
        pre_balance,
        delta = observed.saturating_sub(pre_balance),
        "wallet observed Core balance increase from identity-auth send"
    );

    // Step 7: snapshot the UTXO set and assert it contains the new
    // entry to `auth_addr_zero` for `CORE_SEND_DUFFS`.
    let utxo_count_to_auth_addr = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .utxos()
        .iter()
        .filter(|u| u.value() == CORE_SEND_DUFFS && u.address == auth_addr_zero)
        .count();
    assert!(
        utxo_count_to_auth_addr >= 1,
        "wallet's UTXO set does NOT contain a {CORE_SEND_DUFFS}-duff \
         entry to the identity-auth address. The SPV bloom filter \
         needs to carry DIP-9 subfeature 0..3 \
         (rust-dashcore#554)."
    );

    s.teardown().await.expect("teardown");
}

/// Derive the P2PKH `dashcore::Address` for the identity-auth keypair
/// at `(identity_index, key_index)` on `network`. Mirrors the
/// derivation in `framework::signer::derive_identity_key` but stops
/// at the public-key → address step instead of building an
/// `IdentityPublicKey`.
fn derive_auth_address(
    seed_bytes: &[u8; 64],
    network: Network,
    identity_index: u32,
    key_index: u32,
) -> Result<Address, String> {
    let root_priv = RootExtendedPrivKey::new_master(seed_bytes)
        .map_err(|err| format!("invalid seed for root xpriv: {err}"))?;
    let master = root_priv.to_extended_priv_key(network);
    let derived =
        derive_ecdsa_identity_auth_keypair_from_master(&master, network, identity_index, key_index)
            .map_err(|err| format!("derive ({identity_index}, {key_index}): {err}"))?;
    let secp_pubkey = SecpPublicKey::from_slice(&derived.public_key).map_err(|err| {
        format!("public_key bytes from derive are not a valid secp256k1 pubkey: {err}")
    })?;
    Ok(Address::p2pkh(&PublicKey::new(secp_pubkey), network))
}
