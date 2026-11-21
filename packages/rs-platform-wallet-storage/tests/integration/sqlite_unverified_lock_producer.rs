use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use dash_spv::error::NetworkResult;
use dash_spv::network::{Inbound, MessageType, NetworkEvent, NetworkManager, RequestKey};
use dash_spv::test_utils::{InstantLockFixture, MockNetworkManager};
use dash_spv::{ClientConfig, DashSpvClient, EventHandler};
use dash_spv::{SyncEvent, SyncProgress};
use dashcore::network::message::NetworkMessage;
use dashcore::{Network, OutPoint, Txid};
use key_wallet::managed_account::managed_account_trait::ManagedAccountTrait;
use key_wallet::transaction_checking::TransactionContext;
use key_wallet::wallet::managed_wallet_info::wallet_info_interface::WalletInfoInterface;
use platform_wallet::changeset::recorded_history::ReplayEvidence;
use platform_wallet::changeset::PlatformWalletPersistence;
use platform_wallet::key_wallet_manager::{WalletEvent, WalletManager};
use platform_wallet::test_support::core_fixtures::{
    outside, pay, spend_of, wallet_with_receive_address,
};
use platform_wallet::wallet::platform_wallet::PlatformWalletInfo;
use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
use tokio::sync::{broadcast, mpsc, RwLock};

use crate::common::{
    authorize, fresh_persister, mine, persist_events, registration, secure_tempdir,
};

/// Shared handle to the existing in-process transport. No method opens sockets or resolves DNS.
#[derive(Clone)]
struct InjectedNetwork {
    inner: Arc<MockNetworkManager>,
}

impl InjectedNetwork {
    fn new() -> Self {
        Self {
            inner: Arc::new(MockNetworkManager::new()),
        }
    }
}

#[async_trait::async_trait]
impl NetworkManager for InjectedNetwork {
    async fn start(&self) {
        self.inner.start().await;
    }
    async fn stop(&self) {
        self.inner.stop().await;
    }
    async fn enqueue(&self, height: u32, message: NetworkMessage) {
        self.inner.enqueue(height, message).await;
    }
    async fn send_to(&self, address: SocketAddr, message: NetworkMessage) -> bool {
        self.inner.send_to(address, message).await
    }
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()> {
        self.inner.broadcast(message).await
    }
    async fn dispatch_local(&self, message: NetworkMessage) {
        self.inner.dispatch_local(message).await;
    }
    async fn release_request(&self, key: RequestKey) {
        self.inner.release_request(key).await;
    }
    async fn subscribe(&self, types: &[MessageType]) -> mpsc::UnboundedReceiver<Inbound> {
        self.inner.subscribe(types).await
    }
    fn events(&self) -> broadcast::Receiver<NetworkEvent> {
        self.inner.events()
    }
    fn tip(&self) -> u32 {
        self.inner.tip()
    }
    async fn connected_count(&self) -> u32 {
        self.inner.connected_count().await
    }
    async fn ban_peer(&self, address: SocketAddr) {
        self.inner.ban_peer(address).await;
    }
}

#[derive(Debug)]
enum Notice {
    Detected(Txid),
    Locked(Txid),
    Proof(Txid),
    Rejected(u32),
    Error(String),
}

struct Observer(mpsc::UnboundedSender<Notice>);
impl EventHandler for Observer {
    fn on_sync_event(&self, event: &SyncEvent) {
        if let SyncEvent::InstantLockReceived { instant_lock } = event {
            let _ = self.0.send(Notice::Proof(instant_lock.txid()));
        }
    }
    fn on_progress(&self, progress: &SyncProgress) {
        if let Ok(instantsend) = progress.instantsend() {
            if instantsend.invalid() > 0 {
                let _ = self.0.send(Notice::Rejected(instantsend.invalid()));
            }
        }
    }
    fn on_wallet_event(&self, event: &WalletEvent) {
        let notice = match event {
            WalletEvent::TransactionDetected { record, .. } => Notice::Detected(record.txid),
            WalletEvent::TransactionInstantLocked { txid, .. } => Notice::Locked(*txid),
            _ => return,
        };
        let _ = self.0.send(notice);
    }
    fn on_error(&self, error: &str) {
        let _ = self.0.send(Notice::Error(error.to_owned()));
    }
}

async fn receive_until(
    receiver: &mut mpsc::UnboundedReceiver<Notice>,
    predicate: impl Fn(&Notice) -> bool,
) -> Vec<Notice> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut seen = Vec::new();
        loop {
            let notice = receiver.recv().await.expect("SPV observer remains alive");
            if let Notice::Error(error) = &notice {
                panic!("SPV failed: {error}");
            }
            let done = predicate(&notice);
            seen.push(notice);
            if done {
                return seen;
            }
        }
    })
    .await
    .expect("in-process SPV must process the injected messages")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_never_persist_unverified_lock_from_live_spv_before_or_after_its_transaction() {
    for early in [false, true] {
        let (persister, _directory, path) = fresh_persister();
        let persister = Arc::new(persister);
        let (wallet, info, address) = wallet_with_receive_address();
        let wallet_id = wallet.wallet_id;
        let funding = spend_of(
            &[outside(191)],
            vec![pay(50_000, &address), pay(50_000, &address)],
        );
        let sends: Vec<_> = (0..2)
            .map(|vout| {
                authorize(
                    &wallet,
                    &info,
                    &address,
                    spend_of(
                        &[OutPoint::new(funding.txid(), vout)],
                        vec![pay(49_000, &address)],
                    ),
                )
            })
            .collect();
        let invalid_id = sends[0].txid();
        let valid_id = sends[1].txid();
        let invalid_fixture = InstantLockFixture::new(Network::Testnet, &sends[0]).unwrap();
        let valid_fixture = InstantLockFixture::new(Network::Testnet, &sends[1]).unwrap();
        let mut invalid = invalid_fixture.instant_lock().clone();
        invalid.signature = valid_fixture.instant_lock().signature;
        let mut manager = WalletManager::<PlatformWalletInfo>::new(Network::Testnet);
        let mut persistence_events = manager.take_persistence_receiver().unwrap();
        let mut managed = PlatformWalletInfo::from_wallet(&wallet, 0);
        managed.core_wallet = info;
        persister
            .store(wallet_id, registration(&wallet, None))
            .unwrap();
        manager.insert_wallet(wallet, managed).unwrap();
        let manager = Arc::new(RwLock::new(manager));
        mine(&manager, wallet_id, funding, 100).await;
        persist_events(&manager, &persister, &mut persistence_events).await;

        let network = InjectedNetwork::new();
        let spv_directory = secure_tempdir().unwrap();
        let (notices, mut observed) = mpsc::unbounded_channel();
        let client = DashSpvClient::new(
            ClientConfig::new(Network::Testnet)
                .without_filters()
                .with_storage_path(spv_directory.path()),
            network.clone(),
            manager.clone(),
            vec![Arc::new(Observer(notices))],
        )
        .await
        .unwrap();
        client.run().await.unwrap();
        valid_fixture.install(&client).await.unwrap();
        assert!(client.verify_instant_lock(&invalid).await.is_err());
        let checked = client
            .verify_instant_lock(valid_fixture.instant_lock())
            .await
            .unwrap()
            .unwrap();
        let evidence = ReplayEvidence {
            instant_locks: [(valid_id, checked)].into(),
        };

        if early {
            network
                .dispatch_local(NetworkMessage::ISLock(invalid.clone()))
                .await;
            let seen = receive_until(&mut observed, |n| matches!(n, Notice::Rejected(1))).await;
            assert!(!seen
                .iter()
                .any(|n| matches!(n, Notice::Locked(id) | Notice::Proof(id) if *id == invalid_id)));
        }
        for send in &sends {
            network
                .dispatch_local(NetworkMessage::Tx(Box::new(send.clone())))
                .await;
            receive_until(
                &mut observed,
                |n| matches!(n, Notice::Detected(id) if *id == send.txid()),
            )
            .await;
        }
        if !early {
            network
                .dispatch_local(NetworkMessage::ISLock(invalid))
                .await;
            let seen = receive_until(&mut observed, |n| matches!(n, Notice::Rejected(1))).await;
            assert!(!seen
                .iter()
                .any(|n| matches!(n, Notice::Locked(id) | Notice::Proof(id) if *id == invalid_id)));
        }
        network
            .dispatch_local(NetworkMessage::ISLock(valid_fixture.instant_lock().clone()))
            .await;
        let seen = receive_until(
            &mut observed,
            |n| matches!(n, Notice::Locked(id) if *id == valid_id),
        )
        .await;
        assert!(!seen
            .iter()
            .any(|n| matches!(n, Notice::Locked(id) | Notice::Proof(id) if *id == invalid_id)));
        // Both locks traverse the same ordered IS message receiver; the valid wallet
        // event is a processing barrier, rather than a sleep asserting absence.
        client.stop().await;
        persist_events(&manager, &persister, &mut persistence_events).await;
        let raw: i64 = persister
            .lock_conn_for_test()
            .query_row(
                "SELECT count(*) FROM core_instant_locks WHERE wallet_id=?1 AND txid=?2",
                rusqlite::params![wallet_id.as_slice(), invalid_id.as_byte_array().as_slice()],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            raw, 0,
            "a rejected lock must not produce durable IS evidence"
        );
        assert!(matches!(
            persister
                .get_core_tx_record(wallet_id, &invalid_id)
                .unwrap()
                .unwrap()
                .context,
            TransactionContext::Mempool
        ));
        drop((client, manager, persistence_events, persister));
        let reopened = SqlitePersister::open(SqlitePersisterConfig::new(path)).unwrap();
        let loaded = reopened.load_with_replay(&evidence).unwrap();
        let account = &loaded.state.wallets[&wallet_id]
            .wallet_info
            .accounts
            .standard_bip44_accounts[&0];
        assert!(matches!(
            account.transactions()[&invalid_id].context,
            TransactionContext::Mempool
        ));
        assert!(!account.utxos[&OutPoint::new(invalid_id, 0)].is_instantlocked);
        assert!(account.utxos[&OutPoint::new(valid_id, 0)].is_instantlocked);
    }
}
