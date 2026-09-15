//! Review regression: a concurrent payment must survive an adapter snapshot.
mod common;

use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{ensure_wallet_meta, secure_tempdir};
use dashcore::hashes::Hash;
use dpp::identity::{Identity, IdentityV0};
use dpp::prelude::Identifier;
use key_wallet::account::account_type::StandardAccountType;
use platform_wallet::changeset::{
    spawn_wallet_event_adapter, ClientStartState, PersistenceCapabilities, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence, WalletMetadataEntry,
};
use platform_wallet::key_wallet_manager::WalletEvent;
use platform_wallet::test_support::funded_wallet_manager;
use platform_wallet::wallet::identity::{PaymentEntry, PaymentStatus};
use platform_wallet::wallet::persister::WalletPersister;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};

struct PausedSweepStore {
    inner: Arc<SqlitePersister>,
    entered: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    resume: Mutex<std::sync::mpsc::Receiver<()>>,
}

impl PlatformWalletPersistence for PausedSweepStore {
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        self.inner.persistence_capabilities()
    }

    fn store(&self, wallet: [u8; 32], cs: PlatformWalletChangeSet) -> Result<(), PersistenceError> {
        if cs.core.as_ref().is_some_and(|core| !core.sweeps.is_empty()) {
            if let Some(entered) = self.entered.lock().unwrap().take() {
                entered.send(()).unwrap();
                self.resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(10))
                    .unwrap();
            }
        }
        self.inner.store(wallet, cs)
    }

    fn flush(&self, wallet: [u8; 32]) -> Result<(), PersistenceError> {
        self.inner.flush(wallet)
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        self.inner.load()
    }
}

#[tokio::test]
async fn should_preserve_a_payment_persisted_while_a_sweep_snapshot_is_in_flight() {
    let tmp = secure_tempdir().unwrap();
    let path = tmp.path().join("wallet.db");
    let owner = Identifier::from([0xA7; 32]);
    let contact = Identifier::from([0xB7; 32]);
    let loser = dashcore::Txid::from_byte_array([0x5f; 32]);
    let concurrent = dashcore::Txid::from_byte_array([0x6f; 32]).to_string();
    let (wm, wallet_id, _generation, _signer) =
        funded_wallet_manager(StandardAccountType::BIP44Account).await;
    {
        let sqlite = Arc::new(SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap());
        ensure_wallet_meta(&sqlite, &wallet_id);
        sqlite
            .store(
                wallet_id,
                PlatformWalletChangeSet {
                    wallet_metadata: Some(WalletMetadataEntry {
                        network: key_wallet::Network::Testnet,
                        wallet_group_id: wallet_id,
                        birth_height: 0,
                    }),
                    ..Default::default()
                },
            )
            .unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (resume_tx, resume_rx) = std::sync::mpsc::channel();
        let persister = Arc::new(PausedSweepStore {
            inner: Arc::clone(&sqlite),
            entered: Mutex::new(Some(entered_tx)),
            resume: Mutex::new(resume_rx),
        });
        let wp = WalletPersister::new(wallet_id, persister.clone());
        {
            let mut manager = wm.write().await;
            let info = manager.get_wallet_info_mut(&wallet_id).unwrap();
            info.identity_manager
                .add_identity(
                    Identity::V0(IdentityV0 {
                        id: owner,
                        public_keys: BTreeMap::new(),
                        balance: 0,
                        revision: 0,
                    }),
                    0,
                    wallet_id,
                    &wp,
                )
                .unwrap();
            info.identity_manager
                .managed_identity_mut(&owner)
                .unwrap()
                .record_dashpay_payment(
                    loser.to_string(),
                    PaymentEntry::new_sent(contact, 50_000, Some("original".into())),
                    &wp,
                )
                .unwrap();
        }
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tx.send(WalletEvent::TransactionsSwept {
            wallet_id,
            txids: vec![loser],
            superseded_by: dashcore::Txid::from_byte_array([0x77; 32]),
            winner_mined_height: Some(1_499_050),
            released_outpoints: Vec::new(),
            balance: key_wallet::WalletCoreBalance::default(),
            account_balances: BTreeMap::new(),
        })
        .unwrap();
        drop(tx);
        let adapter = spawn_wallet_event_adapter(
            Arc::clone(&wm),
            Arc::downgrade(&persister),
            rx,
            Arc::new(AtomicBool::new(false)),
            tokio_util::sync::CancellationToken::new(),
        );
        tokio::time::timeout(Duration::from_secs(10), entered_rx)
            .await
            .unwrap()
            .unwrap();

        // The adapter has captured the old identity snapshot and released the
        // manager lock. A normal production payment writer now commits B.
        {
            let mut manager = wm.write().await;
            manager
                .get_wallet_info_mut(&wallet_id)
                .unwrap()
                .identity_manager
                .managed_identity_mut(&owner)
                .unwrap()
                .record_dashpay_payment(
                    concurrent.clone(),
                    PaymentEntry::new_sent(contact, 75_000, Some("new payment memo".into())),
                    &wp,
                )
                .unwrap();
        }
        let before = sqlite.load().unwrap();
        let before_identity = &before.wallets[&wallet_id]
            .identity_manager
            .wallet_identities[&wallet_id][&0];
        assert!(
            before_identity.dashpay().payments.contains_key(&concurrent),
            "the concurrent payment was durably recorded before the stale adapter write"
        );

        resume_tx.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(10), adapter)
            .await
            .unwrap()
            .unwrap();
        sqlite.flush(wallet_id).unwrap();
    }
    let reopened = SqlitePersister::open(SqlitePersisterConfig::new(&path)).unwrap();
    let loaded = reopened.load().unwrap();
    let identity = &loaded.wallets[&wallet_id]
        .identity_manager
        .wallet_identities[&wallet_id][&0];
    assert_eq!(
        identity.dashpay().payments[&loser.to_string()].status,
        PaymentStatus::Failed
    );
    assert!(identity.dashpay().payments.contains_key(&concurrent),
        "the adapter's stale full identity snapshot deleted a successfully persisted concurrent payment");
}
