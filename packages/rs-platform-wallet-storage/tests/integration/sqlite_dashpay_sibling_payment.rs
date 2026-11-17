//! Two identities of one wallet that are each other's DashPay contacts: A's
//! receiving account for B and B's external account for A derive one script
//! per index, because B's external xpub is the one A's request carries. A
//! payment from B to A is stored the way production stores it: the wallet,
//! both registrations, B's address reservations and the processed blocks all
//! reach SQLite through the platform-wallet writers and the wallet-event
//! adapter, with no pool row written by hand. The receiving pool keeps the
//! window its registration stored, so on disk the paid address is used only
//! under the external account. A strict reopen must still load, and must
//! leave that address used in the receiving pool. In the past-window test
//! B's earlier reservations are used external rows too, so the reload's
//! marking rounds reach the paid address from them; the store only
//! discovery reaches is pinned in `rehydrate.rs`'s unit tests.

use crate::common;

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Weak};
use std::time::Duration;

use common::secure_tempdir;
use dashcore::{Address, Block, OutPoint, ScriptBuf, Transaction, TxIn, TxOut, Txid, Witness};
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use key_wallet::account::AccountType;
use key_wallet::bip32::{DerivationPath, ExtendedPubKey};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::managed_account::ManagedCoreFundsAccount;
use key_wallet::mnemonic::Mnemonic;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::Network;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::key_wallet_manager::WalletInterface;
use platform_wallet::test_support::{
    test_platform_wallet_manager_with, TEST_PLATFORM_WALLET_MNEMONIC,
};
use platform_wallet::wallet::identity::crypto::dip14::derive_contact_xpub;
use platform_wallet::wallet::identity::network::{
    ContactCryptoProvider, ContactInfoOpened, ContactInfoSealed,
};
use platform_wallet::wallet::persister::WalletPersister;
use platform_wallet::PlatformWalletError;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

const A: [u8; 32] = [0xA1; 32];
const B: [u8; 32] = [0xB2; 32];
const PAID: u64 = 250_000;
const PAYMENT_HEIGHT: u32 = 100;

/// No contact crypto is queued, so a reservation's drain never calls it.
struct NoQueuedContactCrypto;

fn not_queued() -> PlatformWalletError {
    PlatformWalletError::InvalidIdentityData("no contact crypto is queued".into())
}

#[async_trait::async_trait]
impl ContactCryptoProvider for NoQueuedContactCrypto {
    async fn receiving_xpub(
        &self,
        _path: &DerivationPath,
    ) -> Result<ExtendedPubKey, PlatformWalletError> {
        Err(not_queued())
    }

    async fn ecdh_shared_secret(
        &self,
        _path: &DerivationPath,
        _peer: &dpp::secp256k1::PublicKey,
    ) -> Result<zeroize::Zeroizing<[u8; 32]>, PlatformWalletError> {
        Err(not_queued())
    }

    async fn export_auto_accept_private_key(
        &self,
        _path: &DerivationPath,
    ) -> Result<dpp::secp256k1::SecretKey, PlatformWalletError> {
        Err(not_queued())
    }

    async fn account_reference(
        &self,
        _path: &DerivationPath,
        _compact_xpub: &[u8],
        _account_index: u32,
        _version: u32,
    ) -> Result<u32, PlatformWalletError> {
        Err(not_queued())
    }

    async fn unmask_account_reference(
        &self,
        _path: &DerivationPath,
        _compact_xpub: &[u8],
        _account_reference: u32,
    ) -> Result<(u32, u32), PlatformWalletError> {
        Err(not_queued())
    }

    async fn contact_info_seal(
        &self,
        _root_path: &DerivationPath,
        _derivation_index: u32,
        _contact_id: &[u8; 32],
        _private_data_plaintext: &[u8],
        _private_data_iv: &[u8; 16],
    ) -> Result<ContactInfoSealed, PlatformWalletError> {
        Err(not_queued())
    }

    async fn contact_info_open(
        &self,
        _root_path: &DerivationPath,
        _derivation_index: u32,
        _enc_to_user_id: &[u8; 32],
        _private_data_blob: &[u8],
    ) -> Result<ContactInfoOpened, PlatformWalletError> {
        Err(not_queued())
    }
}

fn identity(id: [u8; 32]) -> Identity {
    Identity::V0(IdentityV0 {
        id: Identifier::from(id),
        public_keys: Default::default(),
        balance: 0,
        revision: 0,
    })
}

fn transfer(previous_output: OutPoint, script_pubkey: ScriptBuf, value: u64) -> Transaction {
    Transaction {
        version: 2,
        lock_time: 0,
        input: vec![TxIn {
            previous_output,
            script_sig: ScriptBuf::new(),
            sequence: 0xffffffff,
            witness: Witness::new(),
        }],
        output: vec![TxOut {
            value,
            script_pubkey,
        }],
        special_transaction_payload: None,
    }
}

fn receiving_account() -> AccountType {
    AccountType::DashpayReceivingFunds {
        index: 0,
        user_identity_id: A,
        friend_identity_id: B,
    }
}

struct Stored {
    _tmp: tempfile::TempDir,
    path: PathBuf,
    wallet_id: [u8; 32],
    receiving_xpub: ExtendedPubKey,
    receiving_window: u32,
    paid: Address,
    payment_outpoint: OutPoint,
}

/// Stores a payment from B to A in a fresh wallet: B reserves addresses of
/// its external account for A (each marked used, with the pool snapshot
/// stored) and a block pays the last of them. With `spent_past_window` that
/// is the first address past the window A's receiving pool registered, and a
/// later block spends the coin; without, it is the first address, unspent.
///
/// The session runs on its own runtime, gone before the file is reopened, so
/// nothing the manager spawned still holds the persister's claim on it.
fn store_sibling_payment(spent_past_window: bool) -> Stored {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("session runtime");
    let (stored, persister) = runtime.block_on(store_session(spent_past_window));
    runtime.shutdown_timeout(Duration::from_secs(60));
    assert_eq!(
        persister.strong_count(),
        0,
        "the session's persister outlives its manager and runtime"
    );
    stored
}

async fn store_session(spent_past_window: bool) -> (Stored, Weak<SqlitePersister>) {
    let tmp = secure_tempdir().expect("tempdir");
    let path = tmp.path().join("wallet.db");
    let persister =
        Arc::new(SqlitePersister::open(SqlitePersisterConfig::new(&path)).expect("open persister"));
    let (manager, wallet_id) = test_platform_wallet_manager_with(Arc::clone(&persister)).await;
    let wallet = manager.get_wallet(&wallet_id).await.expect("wallet");
    let wallet_persister = WalletPersister::new(
        wallet_id,
        Arc::clone(&persister) as Arc<dyn PlatformWalletPersistence>,
    );
    let (a, b) = (Identifier::from(A), Identifier::from(B));

    {
        let mut wm = wallet.wallet_manager().write().await;
        let info = wm.get_wallet_info_mut(&wallet_id).expect("wallet info");
        for (index, id) in [(0, A), (1, B)] {
            info.identity_manager
                .add_identity(identity(id), index, wallet_id, &wallet_persister)
                .expect("add identity");
        }
    }
    // The manager's wallet holds no private key; the host's signer derives
    // the receiving xpub from the mnemonic the wallet was created from.
    let signer_wallet = Wallet::from_seed_bytes(
        *Mnemonic::from_phrase(TEST_PLATFORM_WALLET_MNEMONIC)
            .expect("test mnemonic")
            .to_seed(""),
        Network::Testnet,
        WalletAccountCreationOptions::None,
    )
    .expect("signer wallet");
    let receiving_xpub =
        derive_contact_xpub(&signer_wallet, Network::Testnet, 0, &a, &b).expect("receiving xpub");

    let dashpay = wallet.identity().dashpay();
    dashpay
        .register_contact_account(&a, &b, 0, receiving_xpub.xpub)
        .await
        .expect("register A's receiving account for B");
    let receiving_window = wallet
        .wallet_manager()
        .read()
        .await
        .get_wallet_info(&wallet_id)
        .expect("wallet info")
        .core_wallet
        .accounts
        .funds_account(&receiving_account())
        .expect("receiving account")
        .managed_account_type()
        .address_pools()[0]
        .highest_generated
        .expect("registered window");
    let shared_key = [0x5a; 32];
    let request = platform_encryption::encrypt_extended_public_key(
        &shared_key,
        &[0x11; 16],
        &receiving_xpub.compact.to_bytes(),
    );
    dashpay
        .register_external_contact_account(
            &b,
            &identity(A),
            &request,
            zeroize::Zeroizing::new(shared_key),
        )
        .await
        .expect("register B's external account for A");

    let mut paid = None;
    let reservations = if spent_past_window {
        receiving_window + 2
    } else {
        1
    };
    for _ in 0..reservations {
        paid = Some(
            dashpay
                .reserve_payment_address(&b, &a, &NoQueuedContactCrypto)
                .await
                .expect("reserve a payment address"),
        );
    }
    let paid = paid.expect("at least one reservation");

    let payment = transfer(
        OutPoint::new(Txid::from_byte_array([0x3c; 32]), 0),
        paid.script_pubkey(),
        PAID,
    );
    let payment_outpoint = OutPoint::new(payment.txid(), 0);
    let mut blocks = vec![Block::dummy(PAYMENT_HEIGHT, vec![payment])];
    if spent_past_window {
        blocks.push(Block::dummy(
            PAYMENT_HEIGHT + 1,
            vec![transfer(
                payment_outpoint,
                Address::dummy(Network::Testnet, 7).script_pubkey(),
                PAID - 1_000,
            )],
        ));
    }
    {
        let mut wm = wallet.wallet_manager().write().await;
        let wallets = BTreeSet::from([wallet_id]);
        for (height, block) in (PAYMENT_HEIGHT..).zip(&blocks) {
            wm.process_block_for_wallets(block, block.block_hash(), height, &wallets)
                .await;
        }
    }

    drop(wallet);
    // A registered wallet and the manager's event handlers keep each other,
    // and with them the persister, alive past `shutdown`; removing it ends that.
    drop(
        manager
            .remove_wallet(&wallet_id)
            .await
            .expect("remove the wallet"),
    );
    assert!(manager.shutdown().await.all_clean(), "clean shutdown");
    persister.flush(wallet_id).expect("flush");
    let released = Arc::downgrade(&persister);

    let stored = Stored {
        _tmp: tmp,
        path,
        wallet_id,
        receiving_xpub: receiving_xpub.xpub,
        receiving_window,
        paid,
        payment_outpoint,
    };
    (stored, released)
}

/// The paid script's pool rows on disk, as (account kind, used).
fn stored_rows(
    persister: &SqlitePersister,
    wallet_id: &[u8; 32],
    paid: &Address,
) -> Vec<(String, bool)> {
    let conn = persister.lock_conn_for_test();
    let mut statement = conn
        .prepare(
            "SELECT account_type, used FROM core_address_pool \
             WHERE wallet_id = ?1 AND script = ?2 ORDER BY account_type",
        )
        .expect("prepare");
    statement
        .query_map(
            rusqlite::params![&wallet_id[..], paid.script_pubkey().as_bytes()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("rows")
}

fn reopen(stored: &Stored) -> SqlitePersister {
    SqlitePersister::open(SqlitePersisterConfig::new(&stored.path)).expect("reopen")
}

fn used_in_receiving_pool(account: &ManagedCoreFundsAccount, paid: &Address) -> bool {
    account
        .managed_account_type()
        .address_pools()
        .iter()
        .any(|pool| pool.address_info(paid).is_some_and(|info| info.is_used()))
}

/// Every account holding `outpoint`: the funds accounts and the DashPay
/// contact accounts.
fn holders(info: &ManagedWalletInfo, outpoint: &OutPoint) -> Vec<AccountType> {
    (info.accounts.all_funding_accounts().into_iter())
        .chain(info.accounts.dashpay_external_accounts.values())
        .filter(|account| account.utxos.contains_key(outpoint))
        .map(|account| account.managed_account_type().to_account_type())
        .collect()
}

/// B pays A at the first address past the receiving pool's window, and the
/// coin is spent. No unspent coin and no receiving pool row name the address.
/// A strict reload succeeds, the address reads used in the receiving pool,
/// and it is not handed out again.
#[test]
fn a_spent_sibling_payment_past_the_receiving_window_reloads_used_and_not_handed_out() {
    let stored = store_sibling_payment(true);
    let window = stored.receiving_window;

    let persister = reopen(&stored);
    assert_eq!(
        stored_rows(&persister, &stored.wallet_id, &stored.paid),
        vec![("dashpay_external".to_string(), true)],
        "production stores the paid address only under the external account"
    );
    let mut state = persister.load().expect("strict load");
    let slice = state
        .wallets
        .get_mut(&stored.wallet_id)
        .expect("wallet slice");
    assert_eq!(
        holders(&slice.wallet_info, &stored.payment_outpoint),
        Vec::<AccountType>::new(),
        "the spent coin is not restored as unspent"
    );
    assert_eq!(slice.wallet_info.balance.total(), 0);
    let receiving = slice
        .wallet_info
        .accounts
        .funds_account_mut(&receiving_account())
        .expect("receiving account restored");
    assert!(
        used_in_receiving_pool(receiving, &stored.paid),
        "the paid address must come back used in the receiving pool"
    );
    for _ in 0..window + 3 {
        let next = receiving
            .next_address(Some(&stored.receiving_xpub), true)
            .expect("next receiving address");
        assert_ne!(next, stored.paid, "the paid address is handed out again");
        assert!(receiving.mark_address_used(&next));
    }
}

/// B pays A at an address inside the receiving pool's window, and the coin
/// stays unspent: its row and the receiving row exist, but only the external
/// row is used. The two owner resolvers must agree, so a strict load succeeds.
#[test]
fn an_unspent_sibling_payment_inside_the_receiving_window_loads_strictly() {
    let stored = store_sibling_payment(false);

    let persister = reopen(&stored);
    assert_eq!(
        stored_rows(&persister, &stored.wallet_id, &stored.paid),
        vec![
            ("dashpay_external".to_string(), true),
            ("dashpay_receiving".to_string(), false),
        ],
        "production marks the paid address used only under the external account"
    );
    let state = persister.load().expect("strict load");
    let slice = state.wallets.get(&stored.wallet_id).expect("wallet slice");
    let receiving = slice
        .wallet_info
        .accounts
        .funds_account(&receiving_account())
        .expect("receiving account restored");
    assert!(
        used_in_receiving_pool(receiving, &stored.paid),
        "the paid address must come back used in the receiving pool"
    );
    assert_eq!(
        receiving
            .utxos
            .get(&stored.payment_outpoint)
            .map(|utxo| utxo.txout.value),
        Some(PAID),
        "the receiving account holds the coin"
    );
    assert_eq!(
        holders(&slice.wallet_info, &stored.payment_outpoint),
        vec![receiving_account()],
        "no other account holds the coin"
    );
    assert_eq!(slice.wallet_info.balance.confirmed(), PAID);
    assert_eq!(slice.wallet_info.balance.total(), PAID);
}
