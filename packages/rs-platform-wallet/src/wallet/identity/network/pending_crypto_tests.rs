use super::IdentityWallet;
use crate::broadcaster::SpvBroadcaster;
use crate::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use crate::events::{EventHandler, PlatformEventHandler};
use crate::wallet::persister::WalletPersister;
use dpp::identity::{v0::IdentityV0, Identity};
use dpp::prelude::Identifier;
use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub(super) struct CheckedPersistence {
    check: Mutex<Option<Box<dyn Fn() + Send + Sync>>>,
    pub writes: AtomicUsize,
}

impl PlatformWalletPersistence for CheckedPersistence {
    fn store(
        &self,
        _: [u8; 32],
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if !changeset.pending_contact_crypto_added.is_empty() {
            self.check
                .lock()
                .unwrap()
                .as_ref()
                .expect("installed check")();
            self.writes.fetch_add(
                changeset.pending_contact_crypto_added.len(),
                Ordering::SeqCst,
            );
        }
        Ok(())
    }
    fn flush(&self, _: [u8; 32]) -> Result<(), PersistenceError> {
        Ok(())
    }
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

struct NoopEvents;
impl EventHandler for NoopEvents {}
impl PlatformEventHandler for NoopEvents {}

pub(super) async fn fixture() -> (
    IdentityWallet<SpvBroadcaster>,
    Identifier,
    Arc<CheckedPersistence>,
) {
    let backend = Arc::new(CheckedPersistence::default());
    let manager = crate::PlatformWalletManager::new(
        Arc::new(dash_sdk::SdkBuilder::new_mock().build().unwrap()),
        backend.clone(),
        Arc::new(NoopEvents),
    );
    let wallet = manager
        .create_wallet_from_seed_bytes(
            key_wallet::Network::Testnet,
            &[42; 64],
            WalletAccountCreationOptions::None,
            Some(0),
        )
        .await
        .unwrap();
    let iw = wallet.identity().clone();
    let owner = Identifier::from([0xAA; 32]);
    {
        let mut wm = iw.wallet_manager.write().await;
        wm.get_wallet_info_mut(&iw.wallet_id)
            .unwrap()
            .identity_manager
            .add_identity(
                Identity::V0(IdentityV0 {
                    id: owner,
                    public_keys: Default::default(),
                    balance: 0,
                    revision: 0,
                }),
                0,
                iw.wallet_id,
                &WalletPersister::new(iw.wallet_id, backend.clone()),
            )
            .unwrap();
    }
    let wm = Arc::downgrade(&iw.wallet_manager);
    *backend.check.lock().unwrap() = Some(Box::new(move || {
        // Removal needs this same write guard before it can persist the delete.
        assert!(
            wm.upgrade().unwrap().try_write().is_err(),
            "identity removal must not overtake pending-crypto persistence"
        );
    }));
    (iw, owner, backend)
}

pub(super) async fn remove_owner(iw: &IdentityWallet<SpvBroadcaster>, owner: &Identifier) {
    let mut wm = iw.wallet_manager.write().await;
    wm.get_wallet_info_mut(&iw.wallet_id)
        .unwrap()
        .identity_manager
        .remove_identity(owner, &iw.persister)
        .unwrap();
}
