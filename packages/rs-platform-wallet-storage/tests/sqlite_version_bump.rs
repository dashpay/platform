#![allow(clippy::field_reassign_with_default)]

//! `meta_data_versions` bump discipline. Covers TC-B-011
//! (bump rides the flush tx), TC-B-012 (atomic rollback — data and bump are
//! all-or-nothing), TC-B-013 (every domain maps to a bump; none silently
//! excluded), TC-B-014 (saturating seq, never wraps).

mod common;

use std::collections::BTreeMap;

use common::{ensure_wallet_meta, fresh_persister, wid};
use dpp::prelude::Identifier;
use key_wallet::account::{AccountType, StandardAccountType};
use key_wallet::managed_account::address_pool::AddressPoolType;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{AddressInfo, Network};
use platform_wallet::changeset::{
    AccountAddressPoolEntry, AccountRegistrationEntry, AssetLockChangeSet, ContactChangeSet,
    ContactRequestEntry, CoreChangeSet, IdentityChangeSet, IdentityEntry, IdentityKeyEntry,
    IdentityKeysChangeSet, PendingContactCrypto, PendingContactCryptoOp,
    PlatformAddressBalanceEntry, PlatformAddressChangeSet, PlatformWalletChangeSet,
    PlatformWalletPersistence, ProviderKeyAccountEntry, ProviderKeyExtendedPubKey,
    SentContactRequestKey, TokenBalanceChangeSet, WalletMetadataEntry,
};
use platform_wallet::wallet::identity::{ContactRequest, IdentityStatus};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::sqlite::schema::versions::{self, Domain};

fn one_external_info(seed_byte: u8) -> AddressInfo {
    let wallet = Wallet::from_seed_bytes(
        [seed_byte; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    let info = ManagedWalletInfo::from_wallet(&wallet, 0);
    for managed in info.all_managed_accounts() {
        if !matches!(
            managed.managed_account_type().to_account_type(),
            AccountType::Standard { index: 0, .. }
        ) {
            continue;
        }
        for pool in managed.managed_account_type().address_pools() {
            if pool.pool_type == AddressPoolType::External && !pool.addresses.is_empty() {
                return pool.addresses.values().next().cloned().unwrap();
            }
        }
    }
    panic!("no external pool");
}

fn std_account() -> AccountType {
    AccountType::Standard {
        index: 0,
        standard_account_type: StandardAccountType::BIP44Account,
    }
}

fn test_xpub() -> key_wallet::bip32::ExtendedPubKey {
    key_wallet::bip32::ExtendedPubKey::decode(&hex::decode(
        "0488B21E000000000000000000873DFF81C02F525623FD1FE5167EAC3A55A049DE3D314BB42EE227FFED37D5080339A36013301597DAEF41FBE593A02CC513D0B55527EC2DF1050E2E8FF49C85C2",
    ).unwrap()).unwrap()
}

/// A BLS operator-key account entry — the provider half of the
/// account-registrations domain.
fn provider_operator_entry() -> ProviderKeyAccountEntry {
    let wallet = Wallet::from_seed_bytes(
        [0x2A; 64],
        Network::Testnet,
        WalletAccountCreationOptions::Default,
    )
    .unwrap();
    ProviderKeyAccountEntry {
        account_type: AccountType::ProviderOperatorKeys,
        extended_public_key: ProviderKeyExtendedPubKey::Bls(
            wallet
                .accounts
                .bls_account_of_type(AccountType::ProviderOperatorKeys)
                .expect("BLS operator account")
                .bls_public_key
                .clone(),
        ),
    }
}

/// A changeset that touches exactly one domain, with minimal non-empty data.
/// DB-validity is irrelevant here — `touched_domains` is a pure function.
fn single_domain_changeset(domain: Domain) -> PlatformWalletChangeSet {
    let mut cs = PlatformWalletChangeSet::default();
    match domain {
        Domain::Core => {
            cs.core = Some(CoreChangeSet {
                synced_height: Some(1),
                ..Default::default()
            })
        }
        Domain::Identities => {
            let mut m = BTreeMap::new();
            let id = Identifier::from([0x01; 32]);
            m.insert(id, identity_entry(id));
            cs.identities = Some(IdentityChangeSet {
                identities: m,
                removed: Default::default(),
            });
        }
        Domain::IdentityKeys => {
            let mut keys = IdentityKeysChangeSet::default();
            let id = Identifier::from([0x02; 32]);
            keys.upserts.insert((id, 0), identity_key_entry(id));
            cs.identity_keys = Some(keys);
        }
        Domain::Contacts => {
            let mut sent = BTreeMap::new();
            sent.insert(
                SentContactRequestKey {
                    owner_id: Identifier::from([0x03; 32]),
                    recipient_id: Identifier::from([0x04; 32]),
                },
                contact_request_entry(0x03, 0x04),
            );
            cs.contacts = Some(ContactChangeSet {
                sent_requests: sent,
                ..Default::default()
            });
        }
        Domain::PlatformAddresses => {
            cs.platform_addresses = Some(PlatformAddressChangeSet {
                addresses: vec![PlatformAddressBalanceEntry {
                    wallet_id: [0; 32],
                    account_index: 0,
                    address_index: 0,
                    address: key_wallet::PlatformP2PKHAddress::new([0x05; 20]),
                    funds: dash_sdk::platform::address_sync::AddressFunds {
                        balance: 1,
                        nonce: 0,
                        as_of_height: 0,
                    },
                }],
                ..Default::default()
            });
        }
        Domain::AssetLocks => {
            cs.asset_locks = Some(AssetLockChangeSet::default());
            // Empty map is "empty" — seed one entry to mark it touched.
            cs.asset_locks = Some(asset_lock_changeset());
        }
        Domain::TokenBalances => {
            let mut balances = BTreeMap::new();
            balances.insert(
                (Identifier::from([0x06; 32]), Identifier::from([0x07; 32])),
                1u64,
            );
            cs.token_balances = Some(TokenBalanceChangeSet {
                balances,
                ..Default::default()
            });
        }
        Domain::DashpayProfiles => {
            let mut m = BTreeMap::new();
            m.insert(Identifier::from([0x08; 32]), None);
            cs.dashpay_profiles = Some(m);
        }
        Domain::DashpayPaymentsOverlay => {
            let mut inner = BTreeMap::new();
            inner.insert(
                "tx".to_string(),
                platform_wallet::wallet::identity::PaymentEntry::new_sent(
                    Identifier::from([0x0A; 32]),
                    1,
                    None,
                ),
            );
            let mut m = BTreeMap::new();
            m.insert(Identifier::from([0x09; 32]), inner);
            cs.dashpay_payments_overlay = Some(m);
        }
        Domain::WalletMetadata => {
            cs.wallet_metadata = Some(WalletMetadataEntry {
                network: Network::Testnet,
                wallet_group_id: [0; 32],
                birth_height: 1,
            });
        }
        Domain::AccountRegistrations => {
            cs.account_registrations = vec![AccountRegistrationEntry {
                account_type: std_account(),
                account_xpub: test_xpub(),
            }];
            // Provider key-material accounts share this domain — both halves
            // land in `account_registrations` rows (dashpay/platform#4113).
            cs.provider_key_account_registrations = vec![provider_operator_entry()];
        }
        Domain::AccountAddressPools => {
            cs.account_address_pools = vec![AccountAddressPoolEntry {
                account_type: std_account(),
                pool_type: AddressPoolType::External,
                addresses: vec![],
            }];
        }
        Domain::PendingContactCrypto => {
            cs.pending_contact_crypto_added = vec![PendingContactCrypto {
                owner_identity_id: Identifier::from([0x06; 32]),
                contact_id: Identifier::from([0x07; 32]),
                op: PendingContactCryptoOp::RegisterReceiving,
                enqueued_at_ms: 0,
            }];
        }
        Domain::Invitations => {
            use dashcore::hashes::Hash;
            use dashcore::{OutPoint, Txid};
            use platform_wallet::changeset::{InvitationChangeSet, InvitationEntry, InvitationStatus};
            let op = OutPoint {
                txid: Txid::from_byte_array([0x0C; 32]),
                vout: 0,
            };
            let mut invitations = BTreeMap::new();
            invitations.insert(
                op,
                InvitationEntry {
                    out_point: op,
                    funding_index: 0,
                    amount_duffs: 1,
                    expiry_unix: 0,
                    created_at_secs: 0,
                    has_inviter: false,
                    status: InvitationStatus::Created,
                },
            );
            cs.invitations = Some(InvitationChangeSet {
                invitations,
                removed: Default::default(),
            });
        }
    }
    cs
}

fn identity_entry(id: Identifier) -> IdentityEntry {
    IdentityEntry {
        id,
        balance: 1,
        revision: 1,
        identity_index: Some(0),
        last_updated_balance_block_time: None,
        last_synced_keys_block_time: None,
        dpns_names: Vec::new(),
        contested_dpns_names: Vec::new(),
        status: IdentityStatus::Active,
        wallet_id: None,
        dashpay_profile: None,
        dashpay_payments: Default::default(),
        contact_profiles: Default::default(),
        ignored_senders: Default::default(),
    }
}

fn identity_key_entry(id: Identifier) -> IdentityKeyEntry {
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::platform_value::BinaryData;
    IdentityKeyEntry {
        identity_id: id,
        key_id: 0,
        public_key: IdentityPublicKey::V0(IdentityPublicKeyV0 {
            id: 0,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(vec![2u8; 33]),
            disabled_at: None,
        }),
        public_key_hash: [3u8; 20],
        wallet_id: None,
        derivation_indices: None,
    }
}

fn contact_request_entry(sender: u8, recipient: u8) -> ContactRequestEntry {
    ContactRequestEntry {
        request: ContactRequest {
            sender_id: Identifier::from([sender; 32]),
            recipient_id: Identifier::from([recipient; 32]),
            sender_key_index: 0,
            recipient_key_index: 0,
            account_reference: 0,
            encrypted_account_label: None,
            encrypted_public_key: Vec::new(),
            auto_accept_proof: None,
            core_height_created_at: 0,
            created_at: 0,
        },
    }
}

fn asset_lock_changeset() -> AssetLockChangeSet {
    use dashcore::hashes::Hash;
    use dashcore::{OutPoint, Transaction, Txid};
    use key_wallet::wallet::managed_wallet_info::asset_lock_builder::AssetLockFundingType;
    use platform_wallet::changeset::AssetLockEntry;
    use platform_wallet::wallet::asset_lock::tracked::AssetLockStatus;
    let op = OutPoint {
        txid: Txid::from_byte_array([0x0B; 32]),
        vout: 0,
    };
    let mut cs = AssetLockChangeSet::default();
    cs.asset_locks.insert(
        op,
        AssetLockEntry {
            out_point: op,
            transaction: Transaction {
                version: 3,
                lock_time: 0,
                input: vec![],
                output: vec![],
                special_transaction_payload: None,
            },
            account_index: 0,
            funding_type: AssetLockFundingType::IdentityTopUp,
            identity_index: 0,
            amount_duffs: 1,
            status: AssetLockStatus::Built,
            proof: None,
        },
    );
    cs
}

/// TC-B-013 — every domain maps to exactly its own bump; none silently
/// excluded. Each single-field changeset yields exactly its domain, and the
/// union covers `Domain::ALL`. The exhaustive destructure in
/// `touched_domains` makes a newly added field a compile error there.
#[test]
fn tc_b_013_every_domain_maps_and_isolates() {
    use std::collections::BTreeSet;
    assert_eq!(
        Domain::ALL.len(),
        14,
        "provider-key registrations ride the account-registrations domain — no new domain"
    );
    let mut covered = BTreeSet::new();
    for domain in Domain::ALL {
        let cs = single_domain_changeset(domain);
        let touched = versions::touched_domains(&cs);
        assert_eq!(
            touched,
            vec![domain],
            "single-field changeset for {domain:?} must touch exactly that domain"
        );
        covered.insert(domain.as_str());
    }
    let all: BTreeSet<&str> = Domain::ALL.iter().map(|d| d.as_str()).collect();
    assert_eq!(covered, all, "all domains must be reachable");
}

/// TC-B-011 — a flush touching the core-pool domain commits the pool row and
/// its `meta_data_versions.seq` together (same connection, same tx).
#[test]
fn tc_b_011_bump_rides_the_flush() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB1);
    ensure_wallet_meta(&persister, &w);

    let info = one_external_info(0x11);
    persister
        .store(
            w,
            PlatformWalletChangeSet {
                account_address_pools: vec![AccountAddressPoolEntry {
                    account_type: std_account(),
                    pool_type: AddressPoolType::External,
                    addresses: vec![info.clone()],
                }],
                ..Default::default()
            },
        )
        .unwrap();

    let conn = persister.lock_conn_for_test();
    let pool_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(pool_rows >= 1, "pool row must be present");
    let seq = versions::read_seq(&conn, &w, Domain::AccountAddressPools).unwrap();
    assert_eq!(seq, 1, "the domain's seq bumped in the same flush");
    // No unrelated domain bumped.
    assert_eq!(versions::read_seq(&conn, &w, Domain::Core).unwrap(), 0);
}

/// A domain bumps once per flush; two flushes → seq 2.
#[test]
fn repeated_flush_increments_seq() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB0);
    ensure_wallet_meta(&persister, &w);
    for _ in 0..2 {
        persister
            .store(w, single_domain_changeset(Domain::WalletMetadata))
            .unwrap();
    }
    let conn = persister.lock_conn_for_test();
    assert_eq!(
        versions::read_seq(&conn, &w, Domain::WalletMetadata).unwrap(),
        2
    );
}

/// TC-B-012 — atomicity: a flush that fails partway persists neither the
/// data nor the version bump. A pool write plus a token-balance write whose
/// identity FK is absent must roll the whole tx back.
#[test]
fn tc_b_012_partial_failure_rolls_back_data_and_bump() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB2);
    ensure_wallet_meta(&persister, &w);

    let info = one_external_info(0x22);
    let mut balances = BTreeMap::new();
    // No identities row for this id → token_balances FK violation mid-flush.
    balances.insert(
        (Identifier::from([0xEE; 32]), Identifier::from([0xEF; 32])),
        1u64,
    );
    let result = persister.store(
        w,
        PlatformWalletChangeSet {
            account_address_pools: vec![AccountAddressPoolEntry {
                account_type: std_account(),
                pool_type: AddressPoolType::External,
                addresses: vec![info],
            }],
            token_balances: Some(TokenBalanceChangeSet {
                balances,
                ..Default::default()
            }),
            ..Default::default()
        },
    );
    assert!(result.is_err(), "FK violation must fail the flush");

    let conn = persister.lock_conn_for_test();
    let pool_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM core_address_pool WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    let version_rows: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM meta_data_versions WHERE wallet_id = ?1",
            rusqlite::params![w.as_slice()],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(pool_rows, 0, "pool write must roll back with the failed tx");
    assert_eq!(version_rows, 0, "no bump may survive a rolled-back flush");
}

/// TC-B-014 — a seq pre-seeded to i64::MAX saturates on the next bump and
/// never wraps to a lower value (which would look like a cache rollback).
#[test]
fn tc_b_014_seq_saturates_at_i64_max() {
    let (persister, _tmp, _path) = fresh_persister();
    let w: WalletId = wid(0xB4);
    ensure_wallet_meta(&persister, &w);
    {
        let conn = persister.lock_conn_for_test();
        conn.execute(
            "INSERT INTO meta_data_versions (wallet_id, domain, seq) \
             VALUES (?1, 'wallet_metadata', 9223372036854775807)",
            rusqlite::params![w.as_slice()],
        )
        .unwrap();
    }
    persister
        .store(w, single_domain_changeset(Domain::WalletMetadata))
        .unwrap();
    let conn = persister.lock_conn_for_test();
    assert_eq!(
        versions::read_seq(&conn, &w, Domain::WalletMetadata).unwrap(),
        i64::MAX,
        "seq must saturate, never wrap"
    );
}
