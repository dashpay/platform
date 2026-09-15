#![allow(clippy::field_reassign_with_default)]

//! A database written before V019 keeps its identity and DashPay profile rows
//! readable: the migration stamps the rows it finds 0, the stamped decoders
//! route them to the pre-payment-address record, and a rewrite stamps 1.
//! The fixture is written at V007 (the last published pre-rehydration
//! schema), so the whole V008 -> V019 chain runs over the legacy bytes.

use std::collections::BTreeMap;

use dpp::identity::accessors::IdentityGettersV0;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{IdentityChangeSet, IdentityEntry};
use platform_wallet::{ContactProfileEntry, DashPayProfile, IdentityStatus};
use platform_wallet_storage::sqlite::schema::identity_profile_encoding::{
    encode_legacy_identity, encode_legacy_profile,
};
use platform_wallet_storage::sqlite::schema::{blob, dashpay, identities};
use platform_wallet_storage::sqlite::{decode_identity, decode_profile, migrations};
use rusqlite::{params, Connection};

const WALLET_ID: [u8; 32] = [0x62; 32];

fn old_profile() -> DashPayProfile {
    DashPayProfile {
        display_name: Some("Alice".into()),
        bio: Some("hello".into()),
        public_message: Some("hello".into()),
        ..Default::default()
    }
}

fn entry(id: Identifier) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 123,
        revision: 2,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: vec![],
        contested_dpns_names: vec![],
        status: IdentityStatus::Unknown,
        wallet_id: Some(WALLET_ID),
        dashpay_profile: Some(old_profile()),
        dashpay_payments: BTreeMap::new(),
        contact_profiles: [(
            id,
            ContactProfileEntry {
                profile: Some(old_profile()),
                checked_at_ms: 321,
            },
        )]
        .into(),
        ignored_senders: [Identifier::from([3; 32])].into(),
    }
}

fn stamps(conn: &Connection) -> (i64, i64) {
    let entry_format = conn
        .query_row("SELECT entry_format FROM identities", [], |row| row.get(0))
        .unwrap();
    let profile_format = conn
        .query_row("SELECT profile_format FROM dashpay_profiles", [], |row| {
            row.get(0)
        })
        .unwrap();
    (entry_format, profile_format)
}

fn stored_profile(conn: &Connection) -> DashPayProfile {
    let (payload, format): (Vec<u8>, i64) = conn
        .query_row(
            "SELECT profile_blob, profile_format FROM dashpay_profiles",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    decode_profile(&payload, format).unwrap()
}

/// Rows a pre-V019 writer stored are stamped 0 by the migration and decode
/// to the pre-address shape through both readers; a rewrite stamps them 1
/// and the new fields round-trip; an insert that omits the stamp is read as
/// the current shape.
#[test]
fn pre_v019_rows_stay_readable_and_rewrites_restamp() {
    let id = Identifier::from([1; 32]);
    let old = entry(id);

    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(7))
        .run(&mut conn)
        .unwrap();
    conn.execute(
        "INSERT INTO wallet_metadata (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, wallet_index, entry_blob, tombstoned) \
         VALUES (?1, ?2, 0, ?3, 0)",
        params![
            id.as_slice(),
            WALLET_ID.as_slice(),
            encode_legacy_identity(&old).unwrap()
        ],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO dashpay_profiles (identity_id, profile_blob) VALUES (?1, ?2)",
        params![
            id.as_slice(),
            encode_legacy_profile(&old_profile()).unwrap()
        ],
    )
    .unwrap();

    migrations::run(&mut conn).unwrap();

    assert_eq!(stamps(&conn), (0, 0), "rows present at V019 are legacy");
    assert_eq!(stored_profile(&conn), old_profile());
    let restored = identities::fetch(&conn, &WALLET_ID, &id.to_buffer())
        .unwrap()
        .unwrap();
    assert_eq!(restored, old);
    assert!(restored
        .dashpay_profile
        .as_ref()
        .unwrap()
        .shielded_address
        .is_none());
    // The reader the runtime restores from dispatches on the stamp too: a
    // legacy row decoded against the current shape would fail this load.
    let state = identities::load_state(&conn, &WALLET_ID).unwrap();
    let managed = &state.wallet_identities[&WALLET_ID][&0];
    assert_eq!(managed.identity.id(), id);
    assert_eq!(managed.identity.balance(), 123);

    // Rewriting the rows through the live writers stamps them current.
    let mut current = restored;
    current.dashpay_profile.as_mut().unwrap().shielded_address = Some(vec![9; 43]);
    let tx = conn.transaction().unwrap();
    identities::apply_upserts(
        &tx,
        &WALLET_ID,
        &IdentityChangeSet {
            identities: [(id, current.clone())].into(),
            ..Default::default()
        },
    )
    .unwrap();
    dashpay::apply(
        &tx,
        &WALLET_ID,
        Some(&BTreeMap::from([(id, current.dashpay_profile.clone())])),
        None,
    )
    .unwrap();
    tx.commit().unwrap();
    assert_eq!(stamps(&conn), (1, 1));
    assert_eq!(
        stored_profile(&conn),
        current.dashpay_profile.clone().unwrap()
    );
    assert_eq!(
        identities::fetch(&conn, &WALLET_ID, &id.to_buffer())
            .unwrap()
            .unwrap(),
        current
    );
    let encoded = blob::encode(&current).unwrap();
    assert_eq!(decode_identity(&encoded, 1).unwrap(), current);

    // The stamp DEFAULT follows the writer: an insert that omits it reads as
    // the current shape, so only rows the migration found are ever legacy.
    let other = Identifier::from([7; 32]);
    conn.execute(
        "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob) \
         VALUES (?1, ?2, 1, ?3)",
        params![
            other.as_slice(),
            WALLET_ID.as_slice(),
            blob::encode(&IdentityEntry {
                id: other,
                identity_index: Some(1),
                ..current.clone()
            })
            .unwrap()
        ],
    )
    .unwrap();
    let format: i64 = conn
        .query_row(
            "SELECT entry_format FROM identities WHERE identity_id = ?1",
            [other.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(format, 1);
    assert_eq!(
        identities::fetch(&conn, &WALLET_ID, &other.to_buffer())
            .unwrap()
            .unwrap()
            .dashpay_profile
            .unwrap()
            .shielded_address,
        Some(vec![9; 43])
    );
}

/// A stamp no decoder knows is refused at the schema, not misread.
#[test]
fn unknown_stamp_is_rejected_by_the_check_constraint() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();
    let id = Identifier::from([4; 32]);
    let err = conn
        .execute(
            "INSERT INTO identities (identity_id, wallet_id, identity_index, entry_blob, entry_format) \
             VALUES (?1, ?2, NULL, ?3, 2)",
            params![id.as_slice(), WALLET_ID.as_slice(), blob::encode(&entry(id)).unwrap()],
        )
        .unwrap_err();
    assert!(err.to_string().contains("CHECK"), "{err}");
}
