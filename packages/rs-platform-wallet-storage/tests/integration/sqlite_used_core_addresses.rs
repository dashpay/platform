#![allow(clippy::field_reassign_with_default)]

//! `load()` marks a used-then-emptied address as used in the assembled
//! `core_wallet_info` pools (derived from the full `core_utxos` set, spent +
//! unspent) so the rehydrated wallet never hands it out again as a fresh
//! receive address (address reuse).

use crate::common;

use common::{fresh_persister, wid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
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

/// A spent (zero-balance) UTXO's address is still reported in
/// `used_core_addresses`, even though it no longer contributes to balance.
#[test]
fn spent_utxo_address_is_marked_used() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0xDD);

    let wallet = Wallet::from_seed_bytes(
        [0x42; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 7);
    let address = WalletInfoInterface::monitored_addresses(&info)
        .into_iter()
        .next()
        .unwrap();

    // Register the wallet so it appears in load()'s payload.
    let manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                ..Default::default()
            },
        )
        .unwrap();

    // A UTXO on `address`, then spend it: the row stays on disk with
    // spent = 1 and contributes no balance.
    let utxo = key_wallet::Utxo {
        outpoint: dashcore::OutPoint {
            txid: {
                use dashcore::hashes::Hash;
                dashcore::Txid::from_byte_array([0x99; 32])
            },
            vout: 0,
        },
        txout: dashcore::TxOut {
            value: 500_000,
            script_pubkey: address.script_pubkey(),
        },
        address: address.clone(),
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
                    new_utxos: vec![utxo.clone()],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    spent_utxos: vec![utxo],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let p2 = reopen(&path);
    let state = p2.load().expect("load");
    let slice = state.wallets.get(&w).expect("wallet slice");

    // Spent UTXO: contributes no balance, but its address is still marked used
    // in the assembled wallet's pool so it is never handed out as fresh again.
    assert_eq!(
        slice.wallet_info.balance.total(),
        0,
        "the spent UTXO must not contribute balance"
    );
    let funds = slice
        .wallet_info
        .accounts
        .all_funding_accounts()
        .into_iter()
        .next()
        .expect("funds account present");
    let marked_used = funds
        .managed_account_type()
        .address_pools()
        .iter()
        .any(|p| {
            p.address_info(&address)
                .map(|i| i.is_used())
                .unwrap_or(false)
        });
    assert!(
        marked_used,
        "a spent UTXO's address must still be marked used in the pool"
    );
}
/// A wallet with one registered DashPay contact account, stored and closed.
struct ContactFixture {
    _tmp: tempfile::TempDir,
    path: std::path::PathBuf,
    w: [u8; 32],
    key: key_wallet::account::account_collection::DashpayAccountKey,
    contact: key_wallet::account::AccountType,
    contact_xpub: key_wallet::bip32::ExtendedPubKey,
}

fn contact_fixture(seed: u8) -> ContactFixture {
    use key_wallet::account::account_collection::DashpayAccountKey;
    use key_wallet::account::{AccountType, StandardAccountType};

    let (persister, tmp, path) = fresh_persister();
    let w = wid(seed);
    let wallet = Wallet::from_seed_bytes(
        [seed; 64],
        key_wallet::Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    // Any xpub serves as the contact's: the account is watch-only.
    let contact_xpub = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .find(|a| {
            a.account_type
                == AccountType::Standard {
                    index: 0,
                    standard_account_type: StandardAccountType::BIP44Account,
                }
        })
        .expect("BIP44 account 0")
        .account_xpub;
    let key = DashpayAccountKey {
        index: 0,
        user_identity_id: [0xC1; 32],
        friend_identity_id: [0xC2; 32],
    };
    let contact = AccountType::DashpayExternalAccount {
        index: key.index,
        user_identity_id: key.user_identity_id,
        friend_identity_id: key.friend_identity_id,
    };
    let mut manifest: Vec<AccountRegistrationEntry> = wallet
        .accounts
        .all_accounts()
        .into_iter()
        .map(|a| AccountRegistrationEntry {
            account_type: a.account_type,
            account_xpub: a.account_xpub,
        })
        .collect();
    manifest.push(AccountRegistrationEntry {
        account_type: contact,
        account_xpub: contact_xpub,
    });
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                wallet_metadata: Some(WalletMetadataEntry {
                    network: key_wallet::Network::Testnet,
                    wallet_group_id: [0u8; 32],
                    birth_height: 7,
                }),
                account_registrations: manifest,
                ..Default::default()
            },
        )
        .unwrap();
    ContactFixture {
        _tmp: tmp,
        path,
        w,
        key,
        contact,
        contact_xpub,
    }
}

impl ContactFixture {
    /// The contact pool as `persister` loads it.
    fn loaded_pool(
        &self,
        persister: &SqlitePersister,
    ) -> key_wallet::managed_account::address_pool::AddressPool {
        let state = persister.load().expect("load");
        let info = &state
            .wallets
            .get(&self.w)
            .expect("wallet slice")
            .wallet_info;
        info.accounts
            .dashpay_external_accounts
            .get(&self.key)
            .expect("the contact's external account is rebuilt")
            .managed_account_type()
            .address_pools()
            .into_iter()
            .next()
            .expect("contact pool")
            .clone()
    }

    /// The contact pool's type and addresses as `persister` loads them,
    /// sorted by index.
    fn pool(
        &self,
        persister: &SqlitePersister,
    ) -> (
        key_wallet::managed_account::address_pool::AddressPoolType,
        Vec<key_wallet::AddressInfo>,
    ) {
        let pool = self.loaded_pool(persister);
        (pool.pool_type, sorted_infos(&pool))
    }

    fn store_pool(
        &self,
        persister: &SqlitePersister,
        pool_type: key_wallet::managed_account::address_pool::AddressPoolType,
        infos: Vec<key_wallet::AddressInfo>,
    ) {
        persister
            .store(
                self.w,
                PlatformWalletChangeSet {
                    account_address_pools: vec![
                        platform_wallet::changeset::AccountAddressPoolEntry {
                            account_type: self.contact,
                            pool_type,
                            addresses: infos,
                        },
                    ],
                    ..Default::default()
                },
            )
            .unwrap();
    }
}

fn sorted_infos(
    pool: &key_wallet::managed_account::address_pool::AddressPool,
) -> Vec<key_wallet::AddressInfo> {
    let mut infos: Vec<key_wallet::AddressInfo> = pool.addresses.values().cloned().collect();
    infos.sort_by_key(|info| info.index);
    infos
}

fn state_at(
    infos: &[key_wallet::AddressInfo],
    index: u32,
) -> key_wallet::managed_account::address_pool::AddressState {
    infos
        .iter()
        .find(|info| info.index == index)
        .map(|info| info.state)
        .expect("index in the restored pool")
}

/// A contact's external account is no funds account, yet its used
/// addresses must come back on it: a relaunch that handed a paid contact
/// address out again would link two payments on-chain. A contact payment
/// stores its address reserved before the broadcast, so an unsettled
/// reservation — the process died mid-send — reloads used as well, while a
/// released one reloads available.
#[test]
fn contact_pool_reloads_used_and_unsettled_reserved_addresses() {
    use key_wallet::managed_account::address_pool::AddressState;

    let fixture = contact_fixture(0x43);

    // Index 0: paid. Index 1: reserved, then the process dies mid-send.
    // Index 2: reserved, then released after a rejection.
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    infos.truncate(3);
    infos[0].state = AddressState::Used;
    infos[1].state = AddressState::Reserved { at: 1_752_528_626 };
    infos[2].state = AddressState::Reserved { at: 1_752_528_626 };
    fixture.store_pool(&persister, pool_type, infos.clone());
    infos[2].state = AddressState::Available;
    fixture.store_pool(&persister, pool_type, infos);
    drop(persister);

    let (_, reloaded) = fixture.pool(&reopen(&fixture.path));
    assert!(
        matches!(state_at(&reloaded, 0), AddressState::Used),
        "a paid contact address reloads used"
    );
    assert!(
        matches!(state_at(&reloaded, 1), AddressState::Used),
        "an unsettled reservation must not reload as a fresh address"
    );
    assert!(
        matches!(state_at(&reloaded, 2), AddressState::Available),
        "a released reservation reloads available"
    );
}

/// Concurrent sends each take the next contact address, and the ones that
/// were rejected release theirs, so an unsettled reservation can sit past the
/// gap window with every address below it available. It must still reload
/// used, at its own index.
#[test]
fn contact_reservation_past_the_gap_window_reloads_used() {
    use key_wallet::managed_account::address_pool::{AddressState, KeySource};

    let fixture = contact_fixture(0x44);
    let persister = reopen(&fixture.path);
    let mut pool = fixture.loaded_pool(&persister);
    let deep = pool.gap_limit + 1;
    let start = pool.highest_generated.map_or(0, |h| h + 1);
    pool.generate_addresses(
        deep + 1 - start,
        &KeySource::Public(fixture.contact_xpub),
        true,
    )
    .expect("derive past the gap window");
    let mut infos = sorted_infos(&pool);
    for info in &mut infos {
        info.state = if info.index == deep {
            AddressState::Reserved { at: 1_752_528_626 }
        } else {
            AddressState::Available
        };
    }
    fixture.store_pool(&persister, pool.pool_type, infos);
    drop(persister);

    let (_, reloaded) = fixture.pool(&reopen(&fixture.path));
    assert!(
        matches!(state_at(&reloaded, deep), AddressState::Used),
        "an unsettled reservation past the gap window must not reload as a fresh address"
    );
    assert!(
        (0..deep).all(|index| matches!(state_at(&reloaded, index), AddressState::Available)),
        "the released addresses below it reload available"
    );
    assert!(
        reloaded.iter().any(|info| info.index > deep),
        "the gap window is refilled past it"
    );
}

/// A contact claim past the derivation cap is neither checked nor restored,
/// and the refill past the used addresses below it would hand its address
/// out as fresh. A Strict load fails on it; a Recovery load goes ahead with
/// the site counted.
#[test]
fn contact_claim_past_the_derivation_cap_fails_a_strict_load() {
    use key_wallet::managed_account::address_pool::{AddressState, KeySource};
    use platform_wallet_storage::sqlite::LoadSite;
    use platform_wallet_storage::LoadPolicy;

    // The rehydration derivation cap, and the first index past it.
    const CAP: u32 = 10_000;
    let fixture = contact_fixture(0x46);
    let persister = reopen(&fixture.path);
    let mut pool = fixture.loaded_pool(&persister);
    let start = pool.highest_generated.map_or(0, |h| h + 1);
    pool.generate_addresses(
        CAP + 2 - start,
        &KeySource::Public(fixture.contact_xpub),
        true,
    )
    .expect("derive past the cap");
    // A paid address within the cap, and one past it with its own address.
    let mut infos: Vec<_> = sorted_infos(&pool)
        .into_iter()
        .filter(|info| info.index == 0 || info.index == CAP + 1)
        .collect();
    for info in &mut infos {
        info.state = AddressState::Used;
    }
    fixture.store_pool(&persister, pool.pool_type, infos);
    drop(persister);

    let strict = reopen(&fixture.path).load();
    let error = format!(
        "{:?}",
        strict.expect_err("a Strict load fails on the claim")
    );
    assert!(
        error.contains(&(CAP + 1).to_string()),
        "the error names the claim's index: {error}"
    );

    let recovery = SqlitePersister::open(
        SqlitePersisterConfig::new(&fixture.path).with_load_policy(LoadPolicy::Recovery),
    )
    .expect("reopen");
    let (_, reloaded) = fixture.pool(&recovery);
    assert!(
        matches!(state_at(&reloaded, 0), AddressState::Used),
        "a claim within the cap is restored"
    );
    assert_eq!(
        recovery
            .last_load_degradation()
            .by_site
            .get(&LoadSite::RehydrationEnsureDerived)
            .copied(),
        Some(1)
    );
}

/// A contact claim whose slot derives a different address — a re-sent
/// contact request replaced the contact's xpub, or the row is corrupt — keeps
/// its slot used, as the writer keys rows on the slot: a skipped index costs
/// less than an address paid twice. The load goes ahead under Strict, with
/// the site counted.
#[test]
fn contact_claim_off_the_current_xpub_keeps_its_slot_used() {
    use key_wallet::managed_account::address_pool::AddressState;
    use platform_wallet_storage::sqlite::LoadSite;

    let fixture = contact_fixture(0x45);
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    // Index 0 stored with index 1's address, and a deep index likewise.
    infos[0].address = infos[1].address.clone();
    infos[0].script_pubkey = infos[1].script_pubkey.clone();
    infos[0].state = AddressState::Used;
    let mut deep = infos[0].clone();
    deep.index = 50;
    infos.truncate(1);
    infos.push(deep);
    fixture.store_pool(&persister, pool_type, infos);
    drop(persister);

    let persister = reopen(&fixture.path);
    let (_, reloaded) = fixture.pool(&persister);
    assert!(
        matches!(state_at(&reloaded, 0), AddressState::Used),
        "the claimed slot is kept used"
    );
    assert!(
        matches!(state_at(&reloaded, 50), AddressState::Used),
        "a deep claimed slot is derived and kept used"
    );
    assert!(
        matches!(state_at(&reloaded, 1), AddressState::Available),
        "only the claimed slots are marked"
    );
    assert_eq!(
        persister
            .last_load_degradation()
            .by_site
            .get(&LoadSite::UnresolvedUtxoAddress)
            .copied(),
        Some(2)
    );
}

/// A contact claim on a pool type the account has no pool for is a corrupt
/// row whose slot cannot be kept used: a Strict load fails on it.
#[test]
fn contact_claim_on_a_missing_pool_type_fails_a_strict_load() {
    use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};

    let fixture = contact_fixture(0x48);
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    let missing = if pool_type == AddressPoolType::Internal {
        AddressPoolType::External
    } else {
        AddressPoolType::Internal
    };
    infos.truncate(1);
    infos[0].state = AddressState::Used;
    fixture.store_pool(&persister, missing, infos);
    drop(persister);

    let error = format!(
        "{:?}",
        reopen(&fixture.path)
            .load()
            .expect_err("a Strict load fails on the claim")
    );
    assert!(
        error.contains("could not be put back into its address pool"),
        "the claim's slot could not be kept used: {error}"
    );
}

/// Claims on a contact account the wallet does not register restore
/// nothing; a Strict load goes ahead with every such claim counted.
#[test]
fn contact_claims_on_an_unregistered_account_degrade_a_strict_load() {
    use key_wallet::managed_account::address_pool::AddressState;
    use platform_wallet_storage::sqlite::LoadSite;

    let fixture = contact_fixture(0x47);
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    infos.truncate(3);
    for info in &mut infos {
        info.state = AddressState::Used;
    }
    let unregistered = key_wallet::account::AccountType::DashpayExternalAccount {
        index: 0,
        user_identity_id: [0xD1; 32],
        friend_identity_id: [0xD2; 32],
    };
    persister
        .store(
            fixture.w,
            PlatformWalletChangeSet {
                account_address_pools: vec![platform_wallet::changeset::AccountAddressPoolEntry {
                    account_type: unregistered,
                    pool_type,
                    addresses: infos,
                }],
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let persister = reopen(&fixture.path);
    let (_, reloaded) = fixture.pool(&persister);
    assert!(
        reloaded
            .iter()
            .all(|info| matches!(info.state, AddressState::Available)),
        "the registered contact's pool is untouched"
    );
    assert_eq!(
        persister
            .last_load_degradation()
            .by_site
            .get(&LoadSite::OrphanedUtxoOwner)
            .copied(),
        Some(3),
        "each lost claim is counted"
    );
}

/// A script a contact's external row shares with a funds-account row, with
/// a UTXO on it: the pool and UTXO sources must name the same owner, or a
/// Strict load fails on the conflict. Neither names the external account,
/// which never owns this wallet's funds.
#[test]
fn utxo_on_a_script_a_contact_external_row_shares_loads_strict() {
    use key_wallet::managed_account::address_pool::AddressState;

    shared_script_with_a_utxo_loads_strict(0x46, AddressState::Used);
    // The funds row not yet marked: the pool source then has no row for the
    // script, and the UTXO source must not name the external account.
    shared_script_with_a_utxo_loads_strict(0x47, AddressState::Available);
}

fn shared_script_with_a_utxo_loads_strict(
    seed: u8,
    funds_state: key_wallet::managed_account::address_pool::AddressState,
) {
    use key_wallet::account::{AccountType, StandardAccountType};
    use key_wallet::managed_account::address_pool::{AddressPoolType, AddressState};
    use platform_wallet::changeset::AccountAddressPoolEntry;

    let fixture = contact_fixture(seed);
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    infos.truncate(1);
    infos[0].state = AddressState::Used;
    fixture.store_pool(&persister, pool_type, infos.clone());
    let mut shared = infos[0].clone();
    shared.state = funds_state;
    persister
        .store(
            fixture.w,
            PlatformWalletChangeSet {
                account_address_pools: vec![AccountAddressPoolEntry {
                    account_type: AccountType::Standard {
                        index: 0,
                        standard_account_type: StandardAccountType::BIP44Account,
                    },
                    pool_type: AddressPoolType::External,
                    addresses: vec![shared.clone()],
                }],
                core: Some(CoreChangeSet {
                    new_utxos: vec![key_wallet::Utxo {
                        outpoint: dashcore::OutPoint {
                            txid: {
                                use dashcore::hashes::Hash;
                                dashcore::Txid::from_byte_array([0x98; 32])
                            },
                            vout: 0,
                        },
                        txout: dashcore::TxOut {
                            value: 500_000,
                            script_pubkey: shared.script_pubkey.clone(),
                        },
                        address: shared.address.clone(),
                        height: 5,
                        is_coinbase: false,
                        is_confirmed: true,
                        is_instantlocked: false,
                        is_locked: false,
                        is_trusted: false,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let state = reopen(&fixture.path)
        .load()
        .expect("a strict load must not see an owner conflict");
    let info = &state
        .wallets
        .get(&fixture.w)
        .expect("wallet slice")
        .wallet_info;
    assert_eq!(info.balance.total(), 500_000);
}

/// A pre-fix build stored an output to the contact as this wallet's UTXO,
/// and the contact row's used mark never reached disk: the stored coin alone
/// reloads the address used, at its index.
#[test]
fn contact_address_with_a_stored_utxo_reloads_used() {
    use key_wallet::managed_account::address_pool::AddressState;

    let fixture = contact_fixture(0x48);
    let persister = reopen(&fixture.path);
    let (pool_type, mut infos) = fixture.pool(&persister);
    infos.truncate(4);
    fixture.store_pool(&persister, pool_type, infos.clone());
    let paid = infos[3].clone();
    persister
        .store(
            fixture.w,
            PlatformWalletChangeSet {
                core: Some(CoreChangeSet {
                    new_utxos: vec![key_wallet::Utxo {
                        outpoint: dashcore::OutPoint {
                            txid: {
                                use dashcore::hashes::Hash;
                                dashcore::Txid::from_byte_array([0x97; 32])
                            },
                            vout: 0,
                        },
                        txout: dashcore::TxOut {
                            value: 500_000,
                            script_pubkey: paid.script_pubkey.clone(),
                        },
                        address: paid.address.clone(),
                        height: 5,
                        is_coinbase: false,
                        is_confirmed: true,
                        is_instantlocked: false,
                        is_locked: false,
                        is_trusted: false,
                    }],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .unwrap();
    drop(persister);

    let reopened = reopen(&fixture.path);
    let (_, reloaded) = fixture.pool(&reopened);
    assert!(
        matches!(state_at(&reloaded, 3), AddressState::Used),
        "a contact address with a stored coin must not reload as fresh"
    );
    assert!(matches!(state_at(&reloaded, 2), AddressState::Available));
    let state = reopened.load().expect("load");
    let info = &state
        .wallets
        .get(&fixture.w)
        .expect("wallet slice")
        .wallet_info;
    assert!(
        info.accounts
            .all_funding_accounts()
            .iter()
            .all(|account| account.utxos.is_empty()),
        "the contact's coin is not this wallet's to spend or count"
    );
}
