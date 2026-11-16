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
//! 2. Overlay-only writes fold into the canonical identity blob and survive load.
//! 3. Unknown scalar profile snapshots retain the canonical profile, while
//!    explicit profile tombstones clear it.

use crate::common;

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

    // Loading the canonical blob must include the overlay and unrelated core state.
    let state = persister
        .load()
        .expect("load must succeed despite overlay-only write");
    let wallet = state
        .wallets
        .get(&w)
        .expect("wallet present in loaded state");
    assert_eq!(
        wallet.identity_manager.out_of_wallet_identities[&Identifier::from(identity)]
            .dashpay()
            .profile
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("bob")
    );
    assert_eq!(
        wallet.wallet_info.metadata.synced_height, 99,
        "core state must rehydrate intact alongside an unread overlay"
    );
}

#[test]
fn should_preserve_profile_on_unknown_scalar_snapshot_and_apply_explicit_absence() {
    use platform_wallet::changeset::{IdentityChangeSet, IdentityEntry, Merge};
    let (persister, _tmp, _path) = fresh_persister();
    let w = wid(0xC1);
    let raw = [0xC2; 32];
    let id = Identifier::from(raw);
    ensure_wallet_meta(&persister, &w);
    ensure_identity(&persister, &raw, Some(&w));
    let apply_profile = |profile| {
        persister
            .store(
                w,
                PlatformWalletChangeSet {
                    dashpay_profiles: Some(BTreeMap::from([(id, profile)])),
                    ..Default::default()
                },
            )
            .unwrap();
        persister.flush(w).unwrap();
    };
    apply_profile(Some(profile("Retained")));
    let state = persister.load().unwrap();
    let managed = &state.wallets[&w].identity_manager.out_of_wallet_identities[&id];
    assert_eq!(
        managed
            .dashpay()
            .profile
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("Retained")
    );
    let mut scalar = IdentityEntry::from_managed(managed);
    scalar.dashpay_profile = None;
    scalar.balance = 321;
    let scalar_cs = IdentityChangeSet {
        identities: BTreeMap::from([(id, scalar)]),
        ..Default::default()
    };
    persister.store(w, scalar_cs.clone().into()).unwrap();
    persister.flush(w).unwrap();
    let state = persister.load().unwrap();
    let managed = &state.wallets[&w].identity_manager.out_of_wallet_identities[&id];
    assert_eq!(
        managed
            .dashpay()
            .profile
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("Retained")
    );
    let mut cleared = PlatformWalletChangeSet {
        dashpay_profiles: Some(BTreeMap::from([(id, None)])),
        ..Default::default()
    };
    cleared.merge(scalar_cs.into());
    persister.store(w, cleared).unwrap();
    persister.flush(w).unwrap();
    let state = persister.load().unwrap();
    let managed = &state.wallets[&w].identity_manager.out_of_wallet_identities[&id];
    assert!(managed.dashpay().profile.is_none());
    assert_eq!(
        dpp::identity::accessors::IdentityGettersV0::balance(&managed.identity),
        321
    );
    apply_profile(Some(profile("Restored")));
    let state = persister.load().unwrap();
    assert_eq!(
        state.wallets[&w].identity_manager.out_of_wallet_identities[&id]
            .dashpay()
            .profile
            .as_ref()
            .unwrap()
            .display_name
            .as_deref(),
        Some("Restored")
    );
}
