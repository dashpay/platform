//! ID-007 — Identity-auth addresses are NOT visible to SPV monitor.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-007).
//! Pinned status: FRAMEWORK-READY — full test body implemented,
//! `#[ignore]`-tagged. SPV runtime is live (Task #15) and the bank's
//! `send_core_to` helper is wired (CR-003). End-to-end runs need the
//! bank's Core (Layer-1) receive address to be pre-funded on testnet;
//! the address is logged at framework init under target
//! `platform_wallet::e2e::bank`. Tracks closed PR
//! `dashpay/rust-dashcore#554` (the parked attempt to add
//! `BlockchainIdentities*` `AccountType` variants and flip
//! `WalletAccountCreationOptions::Default` to monitor those
//! addresses) and DET follow-up issue `dash-evo-tool#692`.
//!
//! Pins the CURRENT contract:
//! - identity-auth addresses derived via
//!   [`derive_ecdsa_identity_auth_keypair_from_master`] are NOT in
//!   [`WalletInfoInterface::monitored_addresses`] (because they live
//!   on a DIP-9 subfeature path not in
//!   `WalletAccountCreationOptions::Default` at the pinned
//!   `key-wallet` revision).
//! - Sending Core duffs to one of those addresses does NOT increase
//!   the wallet's Core balance (the SPV bloom filter ignores them).
//! - The wallet's UTXO set never observes such a send.
//!
//! When `BlockchainIdentities` support lands upstream and the wallet
//! opts in (any shape — four concrete variants, parameterised
//! subfeature, etc.), FLIP these assertions and the test starts
//! passing for the right reason. The defensive-pin precedent matches
//! `Found-003` / `Found-004`.

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
/// DASH; well above the dust threshold so the bank's would-be Core
/// path doesn't reject it on amount alone, well below any per-test
/// budget concern.
const CORE_SEND_DUFFS: u64 = 100_000;

/// Negative-window for `wait_for_core_balance`: the test pins that
/// the Core balance does NOT reach `CORE_SEND_DUFFS` even after this
/// long, so the wait is EXPECTED to time out under the current
/// contract. Marvin's spec uses 30 seconds; matched here.
const CORE_BALANCE_NEGATIVE_WINDOW: Duration = Duration::from_secs(30);

#[ignore = "ID-007 — needs testnet + bank Core (Layer-1) pre-funding. \
            Framework gates cleared: SPV runtime live (Task #15) and \
            BankWallet::send_core_to implemented (CR-003). End-to-end \
            run requires operator-funded bank Core receive address \
            (logged at framework init under platform_wallet::e2e::bank \
            target). Pins the contract that DIP-9 identity-auth \
            addresses are NOT in monitored_addresses(). Tracks closed \
            PR dashpay/rust-dashcore#554."]
#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn id_007_identity_auth_addresses_not_monitored() {
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

    // Negative variant — same derivation at an UNREGISTERED slot.
    // Registration status is irrelevant to monitoring (the
    // derivation is pure), so the same three current-contract
    // assertions hold.
    let auth_addr_one = derive_auth_address(&seed_bytes, network, 1, 0)
        .expect("derive identity-auth address (identity_index=1, key_index=0)");

    // TODO(ID-007): add BLS subfeature negative variant once
    // `derive_*_bls_identity_auth_keypair_from_master` lands in the
    // upstream `key-wallet` API. Path:
    // `m/9'/coinType'/5'/2'/identity_index'/key_index'`. Same three
    // current-contract assertions are expected to hold.

    // Step 3: snapshot `monitored_addresses()` BEFORE any Core send.
    // The wallet has been live since `setup_with_n_identities`
    // returned, so this is the steady-state monitored set.
    let monitored_before = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .monitored_addresses();
    assert!(
        !monitored_before.contains(&auth_addr_zero),
        "PRE-pin violated: identity-auth address (slot 0) already in \
         monitored_addresses(). The current contract at the pinned \
         key-wallet revision excludes DIP-9 subfeature 0..3 from \
         WalletAccountCreationOptions::Default; if this fires, \
         upstream has flipped the contract and this test must flip \
         its assertions in the same PR."
    );
    assert!(
        !monitored_before.contains(&auth_addr_one),
        "PRE-pin violated: identity-auth address (slot 1, unregistered) \
         already in monitored_addresses(). Registration status is \
         irrelevant — the derivation is pure — so the same contract \
         applies to every (identity_index, key_index) pair."
    );

    // Step 4: send `CORE_SEND_DUFFS` from the bank to `auth_addr_zero`
    // on Layer-1 via `BankWallet::send_core_to` (CR-003). Returns a
    // broadcast `Txid`; we don't wait for instant-lock because the
    // negative contract is "the wallet's monitored set never sees
    // this". The `wait_for_core_balance` call below is what bounds
    // observation of the (expected absent) UTXO.
    let pre_balance = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .balance()
        .spendable();
    let _txid = s
        .base
        .ctx
        .bank()
        .send_core_to(&auth_addr_zero, CORE_SEND_DUFFS)
        .await
        .expect("bank.send_core_to (CR-003 prerequisite — currently unimplemented!)");

    // Step 5: snapshot `monitored_addresses()` AFTER the broadcast.
    // The bloom filter regenerates from `accounts.all_accounts()`,
    // which still excludes the BlockchainIdentities subfeature, so
    // the set must be unchanged with respect to `auth_addr_*`.
    let monitored_after = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .monitored_addresses();
    assert!(
        !monitored_after.contains(&auth_addr_zero),
        "POST-pin violated (slot 0): identity-auth address appeared in \
         monitored_addresses() after a Layer-1 send. Upstream has \
         silently begun monitoring DIP-9 subfeature 0..3; flip the \
         assertions in the same PR that wires the change."
    );
    assert!(
        !monitored_after.contains(&auth_addr_one),
        "POST-pin violated (slot 1): identity-auth address for an \
         unregistered slot appeared in monitored_addresses() after a \
         Layer-1 send. The send didn't even target this slot — \
         something has flipped the default monitored set."
    );

    // Step 6: wait UP TO `CORE_BALANCE_NEGATIVE_WINDOW` for the Core
    // balance to reflect the inbound UTXO. Per the current contract
    // it MUST NOT — the SPV bloom filter doesn't carry `auth_addr_zero`,
    // so the UTXO is invisible to the wallet. We pin the timeout as
    // EXPECTED.
    let core_wait = wait_for_core_balance(
        &s.base.test_wallet,
        pre_balance + 1,
        CORE_BALANCE_NEGATIVE_WINDOW,
    )
    .await;
    assert!(
        core_wait.is_err(),
        "POST-pin violated: wallet observed a Core balance increase \
         after sending to an identity-auth address. Either upstream \
         flipped the monitored-set contract, or the SPV path now \
         reaches into DIP-9 subfeature 0..3 by some other route. \
         Either way, ID-007 must flip its assertions in the same PR. \
         (observed value: {:?})",
        core_wait.ok()
    );

    // Step 7: snapshot the UTXO set and assert it does not contain
    // a `CORE_SEND_DUFFS`-valued entry to `auth_addr_zero`.
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
    assert_eq!(
        utxo_count_to_auth_addr, 0,
        "POST-pin violated: the wallet's UTXO set contains a \
         {CORE_SEND_DUFFS}-duff entry to the identity-auth address. \
         The SPV bloom filter must have started carrying DIP-9 \
         subfeature 0..3 — flip the assertions and document the new \
         contract."
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
