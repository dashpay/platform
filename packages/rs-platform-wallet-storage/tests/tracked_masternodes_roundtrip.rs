//! Tracked-masternode rows: whole-set replace + per-network scoping +
//! restart survival through the `PlatformWalletPersistence` trait methods.

mod common;

use common::fresh_persister;
use platform_wallet::changeset::{PersistenceCapabilities, PlatformWalletPersistence};
use platform_wallet::masternode::{
    PlatformKeySnapshot, TrackedMasternode, TrackedMasternodeSnapshot,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn tracked(byte: u8, label: Option<&str>) -> TrackedMasternode {
    TrackedMasternode {
        pro_tx_hash: [byte; 32],
        label: label.map(str::to_string),
        added_at: byte as u64,
        snapshot: TrackedMasternodeSnapshot {
            platform: Some(PlatformKeySnapshot {
                owner_key_hash: Some([byte; 20]),
                payout_key_hash: Some([byte ^ 0xFF; 20]),
                operator_payout_key_hash: None,
                owner_identity_balance: Some(1_000 + byte as u64),
            }),
            ever_listed: true,
            ..Default::default()
        },
    }
}

#[test]
fn capability_is_attested() {
    let (p, _tmp, _path) = fresh_persister();
    assert!(p
        .persistence_capabilities()
        .contains(PersistenceCapabilities::TRACKED_MASTERNODES));
}

#[test]
fn whole_set_replace_and_network_scoping() {
    let (p, _tmp, path) = fresh_persister();
    let mainnet = dashcore::Network::Mainnet;
    let testnet = dashcore::Network::Testnet;

    p.persist_tracked_masternodes(mainnet, &[tracked(1, Some("alpha")), tracked(2, None)])
        .expect("persist mainnet");
    p.persist_tracked_masternodes(testnet, &[tracked(9, Some("testnode"))])
        .expect("persist testnet");

    // Whole-set replace: dropping node 2 and renaming node 1 must not
    // resurrect anything.
    p.persist_tracked_masternodes(mainnet, &[tracked(1, Some("renamed"))])
        .expect("replace mainnet");

    let mainnet_rows = p.load_tracked_masternodes(mainnet).expect("load mainnet");
    assert_eq!(mainnet_rows.len(), 1);
    assert_eq!(mainnet_rows[0].pro_tx_hash, [1u8; 32]);
    assert_eq!(mainnet_rows[0].label.as_deref(), Some("renamed"));
    assert_eq!(
        mainnet_rows[0]
            .snapshot
            .platform
            .as_ref()
            .and_then(|pl| pl.owner_identity_balance),
        Some(1_001),
        "snapshot JSON round-trips through the row"
    );

    // The other network's rows are untouched.
    let testnet_rows = p.load_tracked_masternodes(testnet).expect("load testnet");
    assert_eq!(testnet_rows.len(), 1);
    assert_eq!(testnet_rows[0].pro_tx_hash, [9u8; 32]);

    // Restart: a fresh persister over the same file sees the same rows.
    drop(p);
    let reopened =
        SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen persister");
    let rows = reopened
        .load_tracked_masternodes(mainnet)
        .expect("load after reopen");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].label.as_deref(), Some("renamed"));
}

#[test]
fn empty_set_clears_the_network() {
    let (p, _tmp, _path) = fresh_persister();
    let network = dashcore::Network::Mainnet;
    p.persist_tracked_masternodes(network, &[tracked(3, None)])
        .expect("persist");
    p.persist_tracked_masternodes(network, &[]).expect("clear");
    assert!(p
        .load_tracked_masternodes(network)
        .expect("load")
        .is_empty());
}
