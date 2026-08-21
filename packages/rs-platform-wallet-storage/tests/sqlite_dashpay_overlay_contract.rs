#![allow(clippy::field_reassign_with_default)]

//! DashPay write-only overlay contract.
//!
//! `dashpay_profiles` / `dashpay_payments_overlay` are a write-only
//! indexed overlay: data written via the dedicated `dashpay_*` changeset
//! slots IS persisted to the tables, but `load()` rehydrates DashPay
//! state from the identities `entry_blob`, NOT from these tables. These
//! tests pin both halves of that contract:
//!
//! 1. A `dashpay_*` write lands in the overlay tables (queryable directly).
//! 2. Writing ONLY the overlay (no identity blob carrying the same data)
//!    does not corrupt `load()` — load succeeds and surfaces the wallet's
//!    other state intact.

mod common;

use std::collections::BTreeMap;

use common::{ensure_identity, ensure_wallet_meta, fresh_persister, wid};
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::wallet::identity::DashPayProfile;

fn profile(name: &str) -> DashPayProfile {
    DashPayProfile {
        display_name: Some(name.to_string()),
        ..Default::default()
    }
}

#[test]
fn dashpay_overlay_write_is_persisted_to_its_table() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xA1);
    let identity = [0xA2u8; 32];
    ensure_wallet_meta(&persister, &w);
    ensure_identity(&persister, &identity, Some(&w));

    let mut profiles: BTreeMap<Identifier, Option<DashPayProfile>> = BTreeMap::new();
    profiles.insert(Identifier::from(identity), Some(profile("alice")));

    let mut cs = PlatformWalletChangeSet::default();
    cs.dashpay_profiles = Some(profiles);
    persister.store(w, cs).expect("store dashpay profile");
    persister.flush(w).expect("flush");

    // The overlay row is physically present in its dedicated table.
    let conn = persister.lock_conn_for_test();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM dashpay_profiles WHERE identity_id = ?1",
            rusqlite::params![&identity[..]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "dashpay_profiles overlay row must be persisted");
}

#[test]
fn overlay_only_write_does_not_corrupt_load() {
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xB1);
    let identity = [0xB2u8; 32];
    ensure_wallet_meta(&persister, &w);
    ensure_identity(&persister, &identity, Some(&w));

    // Give the wallet real, loadable core state plus an overlay-only
    // DashPay write (no identity blob carries this profile).
    let mut core_cs = PlatformWalletChangeSet::default();
    core_cs.wallet_metadata = Some(WalletMetadataEntry {
        network: key_wallet::Network::Testnet,
        wallet_group_id: w,
        birth_height: 0,
    });
    core_cs.core = Some(CoreChangeSet {
        synced_height: Some(99),
        last_processed_height: Some(99),
        ..Default::default()
    });
    persister.store(w, core_cs).expect("store core");
    persister.flush(w).expect("flush core");

    let mut profiles: BTreeMap<Identifier, Option<DashPayProfile>> = BTreeMap::new();
    profiles.insert(Identifier::from(identity), Some(profile("bob")));
    let mut overlay_cs = PlatformWalletChangeSet::default();
    overlay_cs.dashpay_profiles = Some(profiles);
    persister.store(w, overlay_cs).expect("store overlay");
    persister.flush(w).expect("flush overlay");

    // The documented contract: load() reads DashPay from the identities
    // blob (not the overlay table), so the overlay-only write neither
    // appears in nor corrupts the loaded state. load() must still
    // succeed and surface the wallet's core state.
    let state = persister
        .load()
        .expect("load must succeed despite overlay-only write");
    let wallet = state
        .wallets
        .get(&w)
        .expect("wallet present in loaded state");
    assert_eq!(
        wallet.wallet_info.metadata.synced_height, 99,
        "core state must rehydrate intact alongside an unread overlay"
    );
}
