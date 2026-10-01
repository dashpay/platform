//! CR-006 — a no-change asset lock's wallet-level accounting.
//!
//! The shape dashpay/platform#5150 broke: an asset lock that spends one
//! wallet UTXO entirely, so its only output is the OP_RETURN burn and no
//! change returns to the wallet. Upstream `key-wallet` classifies the
//! funding account's view of it as `Outgoing` (the burn is not an owned
//! output), although the value becomes the wallet's own Platform credits.
//! Locks with change (CR-003 / CR-005) never exercise that path.
//!
//! Flow: fund a fresh test wallet with ONE bank send, register an identity
//! from an asset lock that drains BIP44 account 0
//! (`AssetLockFunding::DrainAccountBalance`), then pin, on the harness
//! persister, that the lock transaction has one input and no change, and
//! that every stored wallet-level record plus the row reread after a reload
//! is `Internal` with `net_amount == -(lock + fee)`. The identity is swept
//! back to the bank by teardown.

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::{Purpose, SecurityLevel};
use key_wallet::managed_account::transaction_record::OutputRole;
use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingAccount;
use platform_wallet::AssetLockFunding;

use crate::framework::prelude::*;
use crate::framework::signer::{
    derive_identity_key, SeedBackedCoreSigner, SeedBackedIdentitySigner,
};
use crate::framework::tx_accounting::{assert_tracked_lock_accounting, wait_for_live_record};

/// DIP-9 identity index the drained lock registers.
const IDENTITY_INDEX: u32 = 0;

/// Core duffs the bank sends the test wallet in ONE transaction: the single
/// UTXO the lock drains. 0.3 tDASH nets ample identity credits.
const TEST_WALLET_CORE_FUNDING: u64 = 30_000_000;

/// Floor on the drained lock value: the funding minus a generous fee bound.
const MINIMUM_LOCK_DUFFS: u64 = TEST_WALLET_CORE_FUNDING - 100_000;

#[tokio_shared_rt::test(shared, flavor = "multi_thread", worker_threads = 12)]
async fn cr_006_no_change_asset_lock_accounting() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,platform_wallet=debug".into()),
        )
        .with_test_writer()
        .try_init();

    // Step 1: one bank send → one confirmed UTXO on BIP44 account 0.
    let s = crate::framework::setup_with_core_funded_test_wallet(TEST_WALLET_CORE_FUNDING)
        .await
        .expect("setup_with_core_funded_test_wallet failed");
    let network = s.ctx.config.network;
    let seed_bytes = s.test_wallet.seed_bytes();
    let wallet = s.test_wallet.platform_wallet();
    let wallet_id = wallet.wallet_id();

    // Step 2: the identity's keys (MASTER, HIGH, TRANSFER — the trio the
    // teardown identity sweep needs).
    let keys = [
        (0, Purpose::AUTHENTICATION, SecurityLevel::MASTER),
        (1, Purpose::AUTHENTICATION, SecurityLevel::HIGH),
        (2, Purpose::TRANSFER, SecurityLevel::CRITICAL),
    ];
    let mut keys_map = BTreeMap::new();
    for (key_index, purpose, level) in keys {
        let key = derive_identity_key(
            &seed_bytes,
            network,
            IDENTITY_INDEX,
            key_index,
            purpose,
            level,
        )
        .expect("derive identity key");
        keys_map.insert(key.id() as u32, key);
    }
    let identity_signer = SeedBackedIdentitySigner::new(&seed_bytes, network, IDENTITY_INDEX)
        .expect("build SeedBackedIdentitySigner");
    let asset_lock_signer = SeedBackedCoreSigner::new(seed_bytes, network);

    // Step 3: register from a lock that drains the account — no change.
    let identity = wallet
        .identity()
        .register_identity_with_funding(
            AssetLockFunding::DrainAccountBalance {
                account: AssetLockFundingAccount::Bip44 { account_index: 0 },
                minimum_lock_duffs: Some(MINIMUM_LOCK_DUFFS),
            },
            IDENTITY_INDEX,
            keys_map,
            &identity_signer,
            &asset_lock_signer,
            None,
        )
        .await
        .expect("register_identity_with_funding (CR-006, drained asset lock)");
    tracing::info!(
        target: "platform_wallet::e2e::cases::cr_006",
        identity_id = %identity.id(),
        balance = identity.balance(),
        "CR-006: identity registered from a drained asset lock"
    );

    let tracked = wallet.asset_locks().list_tracked_locks().await;
    assert_eq!(tracked.len(), 1, "CR-006 builds exactly one asset lock");
    let lock = &tracked[0];
    let txid = lock.out_point.txid;

    // Step 4: the precondition this case exists for — one input, and the
    // OP_RETURN burn as the only output (no change).
    let live = wait_for_live_record(&s.ctx.persister, wallet_id, &txid).await;
    let tx = &live.transaction;
    assert_eq!(
        tx.input.len(),
        1,
        "CR-006 precondition: the lock {txid} must spend exactly one UTXO, got {}",
        tx.input.len()
    );
    assert!(
        tx.output.len() == 1 && tx.output[0].script_pubkey.is_op_return(),
        "CR-006 precondition: the lock {txid} must have only its OP_RETURN burn \
         output (no change), got {} output(s)",
        tx.output.len()
    );
    assert!(
        !live
            .output_details
            .iter()
            .any(|d| d.role == OutputRole::Change),
        "CR-006 precondition: the lock {txid} record must carry no change output"
    );

    // Step 5: every stored record and the reloaded row — Internal,
    // net == -(lock + fee) (dashpay/platform#5150).
    assert_tracked_lock_accounting(&s.ctx.persister, wallet_id, lock).await;

    s.teardown().await.expect("teardown");
}
