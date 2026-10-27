//! An outbound DashPay account (`DashpayExternalAccount`) registered again
//! under another xpub — the contact rotated — replaces its stored pool, and
//! the digest of the request it was built from reaches the next load.

use crate::common;

use common::fresh_persister;
use key_wallet::account::account_collection::DashpayAccountKey;
use key_wallet::account::{Account, AccountType};
use key_wallet::bip32::ExtendedPubKey;
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, PlatformWalletChangeSet,
    PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

const OWNER: [u8; 32] = [0x0A; 32];
const CONTACT: [u8; 32] = [0x0B; 32];

fn contact_account() -> AccountType {
    AccountType::DashpayExternalAccount {
        index: 0,
        user_identity_id: OWNER,
        friend_identity_id: CONTACT,
    }
}

fn xpub_from_seed(seed: u8) -> ExtendedPubKey {
    Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet")
    .accounts
    .all_accounts()
    .first()
    .expect("an account")
    .account_xpub
}

/// The registration round an outbound account built from `xpub` emits: the
/// row with `digest`, its pool snapshot, and, when `reset`, the reset that
/// drops whatever pool the account stored before.
fn registration_round(
    xpub: ExtendedPubKey,
    digest: [u8; 32],
    reset: bool,
) -> PlatformWalletChangeSet {
    let account = Account {
        parent_wallet_id: None,
        account_type: contact_account(),
        network: key_wallet::Network::Testnet,
        account_xpub: xpub,
        is_watch_only: true,
    };
    let managed = ManagedCoreFundsAccount::from_account(&account);
    let pools = managed
        .managed_account_type()
        .address_pools()
        .into_iter()
        .map(|pool| AccountAddressPoolEntry {
            account_type: contact_account(),
            pool_type: pool.pool_type,
            addresses: pool.addresses.values().cloned().collect(),
        })
        .collect();
    PlatformWalletChangeSet {
        account_registrations: vec![AccountRegistrationEntry {
            account_type: contact_account(),
            account_xpub: xpub,
            encrypted_xpub_digest: Some(digest),
        }],
        account_address_pools: pools,
        account_pool_resets: if reset {
            vec![contact_account()]
        } else {
            Vec::new()
        },
        ..Default::default()
    }
}

/// A wallet whose standard accounts are registered, so it loads.
fn registered_wallet(persister: &SqlitePersister) -> WalletId {
    let wallet = Wallet::from_seed_bytes(
        [0x31; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .expect("wallet");
    let wallet_id = wallet.compute_wallet_id();
    persister
        .store(
            wallet_id,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 0,
                }),
                account_registrations: wallet
                    .accounts
                    .all_accounts()
                    .into_iter()
                    .map(|account| AccountRegistrationEntry {
                        account_type: account.account_type,
                        account_xpub: account.account_xpub,
                        encrypted_xpub_digest: None,
                    })
                    .collect(),
                ..Default::default()
            },
        )
        .expect("register wallet");
    wallet_id
}

fn stored_pool_keys(persister: &SqlitePersister, wallet_id: &WalletId) -> Vec<Vec<u8>> {
    let conn = persister.lock_conn_for_test();
    let mut stmt = conn
        .prepare(
            "SELECT public_key FROM core_address_pool \
             WHERE wallet_id = ?1 AND account_type = 'dashpay_external' \
             ORDER BY pool_type, address_index",
        )
        .expect("prepare");
    stmt.query_map([wallet_id.as_slice()], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn expected_pool_keys(xpub: ExtendedPubKey) -> Vec<Vec<u8>> {
    let round = registration_round(xpub, [0; 32], false);
    let mut keys: Vec<(u32, Vec<u8>)> = round
        .account_address_pools
        .iter()
        .flat_map(|entry| entry.addresses.iter())
        .map(|info| {
            let key = match info.public_key.as_ref().expect("a derived key") {
                key_wallet::managed_account::address_pool::PublicKeyType::ECDSA(bytes) => {
                    bytes.clone()
                }
                other => panic!("unexpected key type {other:?}"),
            };
            (info.index, key)
        })
        .collect();
    keys.sort();
    keys.into_iter().map(|(_, key)| key).collect()
}

#[test]
fn should_replace_the_stored_pool_when_an_outbound_account_is_registered_under_another_xpub() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet_id = registered_wallet(&persister);
    let (old, new) = (xpub_from_seed(0x41), xpub_from_seed(0x42));
    persister
        .store(wallet_id, registration_round(old, [1; 32], true))
        .expect("first registration");

    persister
        .store(wallet_id, registration_round(new, [2; 32], true))
        .expect("a replacement with a reset stores");

    assert_eq!(
        stored_pool_keys(&persister, &wallet_id),
        expected_pool_keys(new),
        "only the new xpub's rows remain"
    );
}

#[test]
fn should_refuse_another_xpub_at_the_same_slots_without_a_reset() {
    let (persister, _tmp, _path) = fresh_persister();
    let wallet_id = registered_wallet(&persister);
    persister
        .store(
            wallet_id,
            registration_round(xpub_from_seed(0x41), [1; 32], true),
        )
        .expect("first registration");

    let refused = persister.store(
        wallet_id,
        registration_round(xpub_from_seed(0x42), [2; 32], false),
    );

    let error = refused.expect_err(
        "the pool keeps refusing new key material at a stored slot: the reset is what lets a \
         replacement through",
    );
    assert!(
        format!("{error:?}").contains("TypedPoolKeyConflict"),
        "refused as a key conflict; got {error:?}"
    );
}

#[test]
fn should_hand_the_outbound_account_digest_to_the_next_load() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = registered_wallet(&persister);
    persister
        .store(
            wallet_id,
            registration_round(xpub_from_seed(0x41), [7; 32], true),
        )
        .expect("registration");
    drop(persister);

    let state = SqlitePersister::open(SqlitePersisterConfig::new(&path))
        .expect("reopen")
        .load()
        .expect("load");

    let key = DashpayAccountKey {
        index: 0,
        user_identity_id: OWNER,
        friend_identity_id: CONTACT,
    };
    assert_eq!(
        state.wallets[&wallet_id].outbound_account_digests.get(&key),
        Some(&vec![7u8; 32])
    );
}

#[test]
fn should_keep_only_the_latest_registrations_digest() {
    let (persister, _tmp, path) = fresh_persister();
    let wallet_id = registered_wallet(&persister);
    let xpub = xpub_from_seed(0x41);
    persister
        .store(wallet_id, registration_round(xpub, [1; 32], false))
        .expect("first registration");
    persister
        .store(wallet_id, registration_round(xpub, [2; 32], false))
        .expect("the same account kept from another request");
    let digest = |path: &std::path::Path| {
        SqlitePersister::open(SqlitePersisterConfig::new(path))
            .expect("reopen")
            .load()
            .expect("load")
            .wallets[&wallet_id]
            .outbound_account_digests
            .get(&DashpayAccountKey {
                index: 0,
                user_identity_id: OWNER,
                friend_identity_id: CONTACT,
            })
            .cloned()
    };
    drop(persister);
    assert_eq!(
        digest(&path),
        Some(vec![2u8; 32]),
        "the later request's digest wins"
    );

    let persister = SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("reopen");
    let mut without = registration_round(xpub, [0; 32], false);
    without.account_registrations[0].encrypted_xpub_digest = None;
    without.account_address_pools.clear();
    persister
        .store(wallet_id, without)
        .expect("registered again without a digest");
    drop(persister);
    assert_eq!(digest(&path), None, "a registration without one clears it");
}
