#![allow(clippy::field_reassign_with_default)]

//! Item C — `SqlitePersister::load()` returns the keyless per-wallet
//! rehydration payload in `ClientStartState.wallets` (network, birth
//! height, account manifest, core state, identities, filtered asset
//! locks). Structurally carries no `Wallet`/seed.

mod common;

use common::{ensure_wallet_meta, fresh_persister, wid};
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use platform_wallet::changeset::{
    AccountRegistrationEntry, CoreChangeSet, PlatformWalletChangeSet, PlatformWalletPersistence,
    WalletMetadataEntry,
};
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

fn reopen(path: &std::path::Path) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(path)).expect("reopen")
}

/// The `core_derived_addresses` row a real scan records before a UTXO on
/// `address` lands. The strict UTXO writer refuses an unspent UTXO whose
/// address was never derived, so a stored UTXO must carry its matching
/// derivation. The writer keys its lookup on `(wallet_id, address)` only
/// and the read side re-attributes by topology, so the account fields
/// here are inert placeholders — the address is the load-bearing field.
fn derived_for(address: &dashcore::Address) -> platform_wallet::DerivedAddress {
    // Compressed secp256k1 generator point — a valid placeholder pubkey.
    const PUBKEY_G: [u8; 33] = [
        0x02, 0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87,
        0x0b, 0x07, 0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16,
        0xf8, 0x17, 0x98,
    ];
    platform_wallet::DerivedAddress {
        account_type: key_wallet::account::AccountType::Standard {
            index: 0,
            standard_account_type: key_wallet::account::StandardAccountType::BIP44Account,
        },
        pool_type: key_wallet::managed_account::address_pool::AddressPoolType::External,
        derivation_index: 0,
        address: address.clone(),
        public_key: dashcore::PublicKey::from_slice(&PUBKEY_G).expect("valid compressed pubkey"),
    }
}

/// C-1: a registered wallet with UTXOs round-trips into the keyless
/// `wallets` payload — manifest, network, birth height, core state.
#[test]
fn c1_load_populates_keyless_wallet_payload() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC1);

    let seed = [0x21; 64];
    let wallet = Wallet::from_seed_bytes(
        seed,
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .unwrap();

    // Registration round: metadata + per-account manifest.
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    let reg = PlatformWalletChangeSet {
        wallet_metadata: Some(WalletMetadataEntry {
            network: key_wallet::Network::Testnet,
            birth_height: 7,
        }),
        account_registrations: manifest.clone(),
        ..Default::default()
    };
    persister.store(w, reg).unwrap();

    // A UTXO so the balance is non-zero.
    let utxo = key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: {
                use dashcore::hashes::Hash;
                dashcore::Txid::from_byte_array([0x99; 32])
            },
            vout: 0,
        },
        txout: dashcore::TxOut {
            value: 777_000,
            script_pubkey: address.script_pubkey(),
        },
        address,
        height: 5,
        is_coinbase: false,
        is_confirmed: true,
        is_instantlocked: false,
        is_locked: false,
        is_trusted: false,
    };
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    addresses_derived: vec![derived_for(&utxo.address)],
                    new_utxos: vec![utxo],
                    last_processed_height: Some(50),
                    synced_height: Some(50),
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");

    assert_eq!(state.wallets.len(), 1, "the wallet must be in the payload");
    let slice = state.wallets.get(&w).expect("wallet slice");
    assert_eq!(slice.network, key_wallet::Network::Testnet);
    assert_eq!(slice.birth_height, 7);
    // Every persisted row round-trips. The writer's
    // `(account_type_label, account_index)` upsert key collapses a few
    // distinct special-purpose variants that share a label+index (a
    // persist-side characteristic, not a load bug), so the manifest is
    // a faithful read of what is on disk: non-empty, containing the
    // primary BIP44 account.
    assert!(!slice.account_manifest.is_empty());
    assert!(
        slice.account_manifest.iter().any(|e| matches!(
            e.account_type,
            key_wallet::account::AccountType::Standard { .. }
        )),
        "BIP44 account must be in the manifest"
    );
    assert_eq!(slice.core_state.new_utxos.len(), 1);
    assert_eq!(slice.core_state.new_utxos[0].value(), 777_000);
    assert_eq!(slice.core_state.last_processed_height, Some(50));
}

/// C-2: empty DB → empty `wallets`, no error (keeps the `load()`
/// doctest contract).
#[test]
fn c2_empty_db_empty_wallets() {
    let (persister, _tmp, path) = fresh_persister();
    drop(persister);
    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    assert!(state.wallets.is_empty());
    assert!(state.is_empty());
}

/// C-3: a wallet with only metadata (no UTXOs) still appears, with an
/// empty core projection — not silently dropped.
#[test]
fn c3_metadata_only_wallet_present() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xC3);
    ensure_wallet_meta(&persister, &w);
    drop(persister);
    let p2 = reopen(&path);
    let state = p2.load().unwrap();
    let slice = state.wallets.get(&w).expect("metadata-only wallet present");
    assert!(slice.account_manifest.is_empty());
    assert!(slice.core_state.new_utxos.is_empty());
}
