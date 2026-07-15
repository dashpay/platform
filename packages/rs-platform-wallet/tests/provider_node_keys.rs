use std::sync::{Arc, Mutex};

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::Network;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
    ProviderPlatformNodePubKey,
};
use platform_wallet::events::{EventHandler, PlatformEventHandler};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet::PlatformWalletManager;

struct ProviderNodeKeyPersister {
    keys: Vec<ProviderPlatformNodePubKey>,
    requested_wallets: Mutex<Vec<WalletId>>,
}

impl PlatformWalletPersistence for ProviderNodeKeyPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }

    fn provider_node_keys(
        &self,
        wallet_id: WalletId,
    ) -> Result<Vec<ProviderPlatformNodePubKey>, PersistenceError> {
        self.requested_wallets.lock().unwrap().push(wallet_id);
        Ok(self.keys.clone())
    }
}

struct NoopEventHandler;

impl EventHandler for NoopEventHandler {}
impl PlatformEventHandler for NoopEventHandler {}

#[tokio::test]
async fn platform_wallet_exposes_provider_node_keys_from_its_persister() {
    let keys = vec![ProviderPlatformNodePubKey {
        index: 3,
        public_key: [0x11; 32],
        node_id: [0x22; 20],
    }];
    let persister = Arc::new(ProviderNodeKeyPersister {
        keys: keys.clone(),
        requested_wallets: Mutex::new(Vec::new()),
    });
    let manager = PlatformWalletManager::new(
        Arc::new(dash_sdk::SdkBuilder::new_mock().build().expect("mock sdk")),
        Arc::clone(&persister),
        Arc::new(NoopEventHandler),
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            Network::Testnet,
            &[0x41; 64],
            WalletAccountCreationOptions::Default,
            Some(0),
        )
        .await
        .expect("create wallet");

    assert_eq!(
        wallet
            .persister()
            .provider_node_keys()
            .expect("read provider node keys"),
        keys
    );
    assert_eq!(
        *persister.requested_wallets.lock().unwrap(),
        vec![wallet.wallet_id()]
    );
}
