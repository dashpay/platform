//! A database written before V024 keeps its contact rows readable: the
//! migration stamps the request blobs it finds 0, the stamped decoder routes
//! them to the record without `encrypted_shielded_address`, and a rewrite of
//! a blob stamps that blob 1.

use std::collections::BTreeMap;

use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    ContactChangeSet, ContactRequestEntry, ReceivedContactRequestKey, SentContactRequestKey,
};
use platform_wallet::wallet::identity::ContactRequest;
use platform_wallet_storage::sqlite::migrations;
use platform_wallet_storage::sqlite::schema::contacts::{self, encode_legacy_request};
use platform_wallet_storage::LoadCtx;
use rusqlite::{params, Connection};

const WALLET_ID: [u8; 32] = [0x63; 32];

fn owner() -> Identifier {
    Identifier::from([1; 32])
}

fn contact() -> Identifier {
    Identifier::from([2; 32])
}

/// A request whose every field holds a distinct value, so a positional shift
/// misreads at least one of them.
fn request(sender: Identifier, recipient: Identifier) -> ContactRequest {
    ContactRequest {
        sender_id: sender,
        recipient_id: recipient,
        sender_key_index: 3,
        recipient_key_index: 4,
        account_reference: 5,
        encrypted_account_label: Some(vec![0x2a; 48]),
        encrypted_public_key: vec![0x07; 96],
        auto_accept_proof: Some(vec![0x09; 38]),
        core_height_created_at: 200,
        created_at: 1_700_000_000_000,
        encrypted_shielded_address: None,
    }
}

fn stamps(conn: &Connection) -> (i64, i64) {
    conn.query_row(
        "SELECT outgoing_request_format, incoming_request_format FROM contacts",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
    .unwrap()
}

/// A database at the schema before V024, holding one established contact
/// whose request blobs a writer of that schema stored.
fn pre_v024_database() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::runner()
        .set_target(refinery::Target::Version(23))
        .run(&mut conn)
        .unwrap();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO contacts \
             (wallet_id, owner_id, contact_id, state, outgoing_request, incoming_request) \
         VALUES (?1, ?2, ?3, 'established', ?4, ?5)",
        params![
            WALLET_ID.as_slice(),
            owner().as_slice(),
            contact().as_slice(),
            encode_legacy_request(&request(owner(), contact())).unwrap(),
            encode_legacy_request(&request(contact(), owner())).unwrap(),
        ],
    )
    .unwrap();
    conn
}

#[test]
fn should_read_contact_requests_stored_before_v024() {
    let mut conn = pre_v024_database();
    migrations::run(&mut conn).unwrap();

    assert_eq!(stamps(&conn), (0, 0), "blobs present at V024 are legacy");
    let state = contacts::load_state_for_test(&conn, &WALLET_ID, &LoadCtx::strict()).unwrap();
    let established = &state.established[&SentContactRequestKey {
        owner_id: owner(),
        recipient_id: contact(),
    }];
    assert_eq!(established.outgoing_request, request(owner(), contact()));
    assert_eq!(established.incoming_request, request(contact(), owner()));
}

/// A row's two blobs are written at different times: a re-sync that stores a
/// newer incoming request restamps that blob and leaves the outgoing one in
/// the shape it has.
#[test]
fn should_restamp_only_the_request_blob_it_rewrites() {
    let mut conn = pre_v024_database();
    migrations::run(&mut conn).unwrap();

    let mut incoming = request(contact(), owner());
    incoming.encrypted_shielded_address = Some(vec![0x3c; 64]);
    let tx = conn.transaction().unwrap();
    contacts::apply(
        &tx,
        &WALLET_ID,
        &ContactChangeSet {
            incoming_requests: BTreeMap::from([(
                ReceivedContactRequestKey {
                    owner_id: owner(),
                    sender_id: contact(),
                },
                ContactRequestEntry {
                    request: incoming.clone(),
                },
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    tx.commit().unwrap();

    assert_eq!(stamps(&conn), (0, 1));
    let state = contacts::load_state_for_test(&conn, &WALLET_ID, &LoadCtx::strict()).unwrap();
    let established = &state.established[&SentContactRequestKey {
        owner_id: owner(),
        recipient_id: contact(),
    }];
    assert_eq!(established.outgoing_request, request(owner(), contact()));
    assert_eq!(established.incoming_request, incoming);
}

#[test]
fn should_keep_the_encrypted_shielded_address_of_a_stored_contact_request() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();

    let mut incoming = request(contact(), owner());
    incoming.encrypted_shielded_address = Some(vec![0x3c; 64]);
    let key = ReceivedContactRequestKey {
        owner_id: owner(),
        sender_id: contact(),
    };
    let tx = conn.transaction().unwrap();
    contacts::apply(
        &tx,
        &WALLET_ID,
        &ContactChangeSet {
            incoming_requests: BTreeMap::from([(
                key,
                ContactRequestEntry {
                    request: incoming.clone(),
                },
            )]),
            ..Default::default()
        },
    )
    .unwrap();
    tx.commit().unwrap();

    let state = contacts::load_state_for_test(&conn, &WALLET_ID, &LoadCtx::strict()).unwrap();
    assert_eq!(state.incoming_requests[&key].request, incoming);
}

/// A stamp no decoder knows is refused at the schema, not misread.
#[test]
fn should_refuse_a_contact_request_stamp_no_decoder_knows() {
    let mut conn = Connection::open_in_memory().unwrap();
    migrations::run(&mut conn).unwrap();
    conn.execute(
        "INSERT INTO wallets (wallet_id, network, birth_height) VALUES (?1, 'testnet', 0)",
        params![WALLET_ID.as_slice()],
    )
    .unwrap();

    let err = conn
        .execute(
            "INSERT INTO contacts \
                 (wallet_id, owner_id, contact_id, state, incoming_request, \
                  incoming_request_format) \
             VALUES (?1, ?2, ?3, 'received', ?4, 2)",
            params![
                WALLET_ID.as_slice(),
                owner().as_slice(),
                contact().as_slice(),
                encode_legacy_request(&request(contact(), owner())).unwrap(),
            ],
        )
        .unwrap_err();
    assert!(err.to_string().contains("CHECK"), "{err}");
}
