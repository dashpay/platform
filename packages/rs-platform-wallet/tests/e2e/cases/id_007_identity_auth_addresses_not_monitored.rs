//! ID-007 — Identity-auth addresses are intentionally NOT monitored.
//!
//! Spec: `tests/e2e/TEST_SPEC.md` (### Identity (ID) → ID-007).
//! Pinned status: Pass — pins the intended architecture.
//!
//! Asserts the CORRECT, intentional contract:
//! - identity-auth addresses (DIP-9 subfeature 0..3, 6-component path
//!   `m/9'/coinType'/5'/{0,1,2,3}'/identity_index'/key_index'`) derived
//!   via [`derive_ecdsa_identity_auth_keypair_from_master`] are NOT in
//!   [`WalletInfoInterface::monitored_addresses`]. They are pure key
//!   material — used for signing identity state transitions, NOT for
//!   receiving Layer-1 Dash.
//! - Sending Core duffs to one of these addresses does NOT increase
//!   the wallet's Core balance — the SPV bloom filter intentionally
//!   excludes them.
//! - The UTXO set does NOT contain entries for these addresses.
//!
//! Architecture rationale:
//! - dash-evo-tool (the canonical Platform client) treats these as
//!   pure key material; `account_summary.rs:226-229` explicitly states
//!   they "usually hold zero balance".
//! - DET's `receive_address()` returns BIP-44 paths only, never
//!   identity-auth paths.
//! - DET's UI hides them outside developer-mode "Identity System"
//!   view.
//! - No standard flow sends Layer-1 Dash to these addresses.
//!
//! When this test starts FAILING, it means a regression has happened:
//! either `WalletAccountCreationOptions::Default` started including
//! `BlockchainIdentities*` `AccountType`s (the closed
//! `dashpay/rust-dashcore#554` was a speculative attempt), OR some
//! other code path has begun monitoring these addresses without
//! corresponding architecture review. Investigate before flipping the
//! assertions — the change may be a real architecture shift (in which
//! case flip them) or an accident (in which case revert the breakage).

use std::time::Duration;

use dashcore::secp256k1::PublicKey as SecpPublicKey;
use dashcore::{Address, Network, PublicKey};
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::root_extended_keys::RootExtendedPrivKey;
use platform_wallet::wallet::identity::network::derive_ecdsa_identity_auth_keypair_from_master;

use crate::framework::prelude::*;

/// Funding committed to the registered identity. KEPT LARGER than
/// 0.001 tDASH: must stay above `IDENTITY_SWEEP_FLOOR` (50M,
/// `cleanup.rs`) so the teardown sweep recovers credits back to
/// the bank identity (Marvin v32 forensics — silent leak when an
/// identity ends below the floor). 100M provides 50M margin above
/// floor + sweep transfer fee (~6.5M). Up from the prior 30M which
/// was itself below-floor and leaking ~30M per run invisibly.
const REGISTRATION_FUNDING: u64 = 100_000_000;

/// Layer-1 send amount targeted at the identity-auth address. ~0.001
/// DASH; well above the dust threshold so the bank's Core path
/// doesn't reject it on amount alone, well below any per-test budget
/// concern.
const CORE_SEND_DUFFS: u64 = 100_000;

/// Negative-window for `wait_for_core_balance`: the test pins that
/// the Core balance does NOT reach `CORE_SEND_DUFFS` even after this
/// long, so the wait is EXPECTED to time out under the intentional
/// not-monitored contract. 30 seconds matches Marvin's spec.
const CORE_BALANCE_NEGATIVE_WINDOW: Duration = Duration::from_secs(30);

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

    // Negative-axis variant — same derivation at an UNREGISTERED
    // slot. Registration status is irrelevant to monitoring (the
    // derivation is pure), so the same intended-contract assertions
    // hold: every (identity_index, key_index) pair under the DIP-9
    // identity-authentication subfeature must remain unmonitored.
    let auth_addr_one = derive_auth_address(&seed_bytes, network, 1, 0)
        .expect("derive identity-auth address (identity_index=1, key_index=0)");

    // TODO(ID-007): add BLS subfeature variant once
    // `derive_*_bls_identity_auth_keypair_from_master` lands in the
    // upstream `key-wallet` API. Path:
    // `m/9'/coinType'/5'/2'/identity_index'/key_index'`. Same
    // intended-contract assertions apply.

    // Step 3: snapshot `monitored_addresses()` BEFORE any Core send.
    // The wallet has been live since `setup_with_n_identities`
    // returned, so this is the steady-state monitored set — it
    // intentionally excludes identity-auth addresses.
    let monitored_before = s
        .base
        .test_wallet
        .platform_wallet()
        .state()
        .await
        .monitored_addresses();
    assert!(
        !monitored_before.contains(&auth_addr_zero),
        "PRE-pin violated: identity-auth address (slot 0) is in \
         monitored_addresses(). DET treats these as pure key material \
         (account_summary.rs:226-229) and the wallet's Default \
         monitored set must not include DIP-9 subfeature 0..3. If \
         this fires, either the architecture has shifted (review \
         before flipping) or an accident has started monitoring \
         these addresses (revert the breakage)."
    );
    assert!(
        !monitored_before.contains(&auth_addr_one),
        "PRE-pin violated: identity-auth address (slot 1, unregistered) \
         is in monitored_addresses(). Registration status is \
         irrelevant — the derivation is pure — so the same intended \
         contract applies to every (identity_index, key_index) pair."
    );

    // Step 4: send `CORE_SEND_DUFFS` from the bank to `auth_addr_zero`
    // on Layer-1 via `BankWallet::send_core_to` (CR-003). Returns a
    // broadcast `Txid`; we don't wait for instant-lock because the
    // intended contract is "the wallet's monitored set never sees
    // this". The `wait_for_core_balance` call below bounds
    // observation of the (expected absent) UTXO.
    // Use the same lock-free confirmed-balance accessor that
    // `wait_for_core_balance` polls — pinning `pre_balance + 1` against
    // the same metric the waiter compares against keeps the negative
    // contract crisp (the timeout fires because `auth_addr_zero` isn't
    // in `monitored_addresses()`, not because the two readings drift).
    let pre_balance = s.base.test_wallet.core_balance_confirmed();
    let _txid = s
        .base
        .ctx
        .bank()
        .send_core_to(&auth_addr_zero, CORE_SEND_DUFFS)
        .await
        .expect("bank.send_core_to (CR-003 prerequisite)");

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
         monitored_addresses() after a Layer-1 send. The Default \
         monitored set must remain free of DIP-9 subfeature 0..3 — \
         if it doesn't, the wallet has begun treating identity keys \
         as funds-bearing addresses without architecture review."
    );
    assert!(
        !monitored_after.contains(&auth_addr_one),
        "POST-pin violated (slot 1): identity-auth address for an \
         unregistered slot appeared in monitored_addresses() after a \
         Layer-1 send. The send didn't even target this slot — \
         something has flipped the default monitored set."
    );

    // Step 6: wait UP TO `CORE_BALANCE_NEGATIVE_WINDOW` for the Core
    // balance to reflect the inbound UTXO. Per the intended contract
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
         after sending to an identity-auth address. The intended \
         contract is that DIP-9 subfeature 0..3 is unmonitored; if \
         this assertion fires, either the SPV path now reaches into \
         that subfeature, or an unrelated UTXO landed concurrently \
         (rare in the isolated test environment). \
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
         The intended contract is that the SPV bloom filter does not \
         carry DIP-9 subfeature 0..3 — investigate before flipping \
         the assertions."
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
