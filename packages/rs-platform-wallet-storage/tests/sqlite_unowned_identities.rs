//! `SqlitePersister::load_unowned_identities`: the dedicated door for
//! identities that belong to no wallet.
//!
//! These identities are deliberately absent from `load()`, which
//! enumerates registered wallets. The tests here pin both halves of that
//! contract — that the accessor returns them fully keyed, and that a
//! round trip does NOT quietly hand them to a wallet.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
use dpp::platform_value::BinaryData;
use dpp::prelude::Identifier;
use platform_wallet::changeset::{
    IdentityChangeSet, IdentityEntry, IdentityKeyEntry, IdentityKeysChangeSet,
    PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::identity::IdentityStatus;
use platform_wallet::wallet::platform_wallet::WalletId;

/// The all-zero scope: the storage spelling of "owned by no wallet".
const UNOWNED: WalletId = [0u8; 32];

fn reopen(path: &std::path::Path) -> platform_wallet_storage::SqlitePersister {
    platform_wallet_storage::SqlitePersister::open(
        platform_wallet_storage::SqlitePersisterConfig::new(path),
    )
    .expect("reopen persister")
}

/// An identity owned by nobody: no `wallet_id`, and no registration
/// index (an index is a position within a wallet, and there is none).
fn unowned_entry(id: Identifier) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 4_200,
        revision: 3,
        identity_index: None,
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Unknown,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn key_entry(id: Identifier, key_id: u32, byte: u8) -> IdentityKeyEntry {
    IdentityKeyEntry {
        identity_id: id,
        key_id,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: key_id,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![byte; 33]),
            disabled_at: None,
        }),
        public_key_hash: [byte; 20],
        wallet_id: None,
        derivation_indices: None,
    }
}

fn unowned_changeset(id: Identifier) -> PlatformWalletChangeSet {
    let mut identities = IdentityChangeSet::default();
    identities.identities.insert(id, unowned_entry(id));
    let mut keys = IdentityKeysChangeSet::default();
    keys.upserts.insert((id, 0), key_entry(id, 0, 0xAB));
    keys.upserts.insert((id, 1), key_entry(id, 1, 0xCD));
    PlatformWalletChangeSet {
        identities: Some(identities),
        identity_keys: Some(keys),
        ..Default::default()
    }
}

/// Write an unowned identity and its keys at the sentinel scope, reopen,
/// and read it back through the accessor: the identity comes back
/// carrying its keys, and it is STILL unowned. The last part is the
/// whole reason this accessor exists — routing these through a wallet's
/// bucket would let the `identities` orphan-promotion upsert claim them
/// on the next flush.
#[test]
fn unowned_identity_round_trips_with_keys_and_stays_unowned() {
    let (persister, tmp, path) = fresh_persister();
    let id = Identifier::from([0x7Au8; 32]);
    persister.store(UNOWNED, unowned_changeset(id)).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let unowned = p2.load_unowned_identities().expect("accessor reads");

    let managed = unowned.get(&id).expect("the unowned identity comes back");
    assert_eq!(managed.identity.balance(), 4_200);
    assert_eq!(
        managed.wallet_id, None,
        "an unowned identity must not come back owned — not by a real \
         wallet, and not by the all-zero sentinel either"
    );

    // Keys are folded in, so the accessor is usable on its own: an
    // identity returned without its keys would be a trap.
    let keys = managed.identity.public_keys();
    assert_eq!(keys.len(), 2, "both persisted keys must be present");
    assert_eq!(keys[&0].data().as_slice(), &[0xAB; 33]);
    assert_eq!(keys[&1].data().as_slice(), &[0xCD; 33]);

    // On disk both sides are a genuine SQL NULL, not a zero blob.
    let conn = p2.lock_conn_for_test();
    let identity_null: bool = conn
        .query_row(
            "SELECT wallet_id IS NULL FROM identities WHERE identity_id = ?1",
            rusqlite::params![id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(identity_null, "identities.wallet_id must still be NULL");
    let keys_null: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM identity_keys \
             WHERE identity_id = ?1 AND wallet_id IS NOT NULL",
            rusqlite::params![id.as_slice()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        keys_null, 0,
        "no key may have acquired a wallet scope across the round trip"
    );
    drop(conn);
    drop(tmp);
}

/// The documented limitation, pinned: `load()` does not deliver unowned
/// identities. A registered wallet is present so the store is not
/// trivially empty — its own state loads, the unowned identity does not.
#[test]
fn load_does_not_deliver_unowned_identities() {
    let (persister, tmp, path) = fresh_persister();
    let w = wid(0x2B);
    ensure_wallet_meta(&persister, &w);
    let id = Identifier::from([0x7Bu8; 32]);
    persister.store(UNOWNED, unowned_changeset(id)).unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load succeeds");
    assert!(
        state.wallets.contains_key(&w),
        "the registered wallet still loads normally"
    );
    for (wallet_id, wallet_state) in &state.wallets {
        assert!(
            !wallet_state
                .identity_manager
                .out_of_wallet_identities
                .contains_key(&id),
            "the unowned identity leaked into wallet {}'s bucket",
            hex::encode(wallet_id)
        );
    }

    // ...but it is reachable through its own door.
    assert!(
        p2.load_unowned_identities().unwrap().contains_key(&id),
        "the accessor must still find it"
    );
    drop(tmp);
}
