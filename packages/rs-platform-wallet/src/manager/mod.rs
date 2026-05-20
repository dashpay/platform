//! Multi-wallet manager with SPV coordination.

pub mod accessors;
pub mod identity_sync;
mod load;
pub mod load_outcome;
pub mod platform_address_sync;
pub mod rehydrate;
#[cfg(feature = "shielded")]
pub mod shielded_sync;
mod wallet_lifecycle;

use std::sync::Arc;

use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use key_wallet_manager::WalletManager;

use crate::changeset::{spawn_wallet_event_adapter, PlatformWalletPersistence};
use crate::events::{PlatformEventHandler, PlatformEventManager};
use crate::manager::identity_sync::IdentitySyncManager;
use crate::manager::platform_address_sync::PlatformAddressSyncManager;
#[cfg(feature = "shielded")]
use crate::manager::shielded_sync::ShieldedSyncManager;
use crate::spv::SpvRuntime;
use crate::wallet::asset_lock::LockNotifyHandler;
use crate::wallet::core::BalanceUpdateHandler;
use crate::wallet::platform_wallet::{PlatformWalletInfo, WalletId};
use crate::wallet::PlatformWallet;

/// Multi-wallet coordinator with SPV sync and event handling.
///
/// Events are dispatched through [`PlatformEventManager`] to all registered
/// [`PlatformEventHandler`]s by reference (no cloning).
pub struct PlatformWalletManager<P: PlatformWalletPersistence + 'static> {
    pub(super) sdk: Arc<dash_sdk::Sdk>,
    pub(super) wallet_manager: Arc<RwLock<WalletManager<PlatformWalletInfo>>>,
    /// Map of registered wallets. Held in an `Arc` so the
    /// `BalanceUpdateHandler` can hold a clone and look up wallets to
    /// update their lock-free balance atomics from event-handler
    /// context, without touching the SPV-contended `wallet_manager`
    /// lock.
    pub(super) wallets: Arc<RwLock<std::collections::BTreeMap<WalletId, Arc<PlatformWallet>>>>,
    /// Notified on InstantLock / ChainLock events for `AssetLockManager` waiters.
    pub(super) lock_notify: Arc<Notify>,
    pub(super) spv_manager: Arc<SpvRuntime>,
    /// Periodic platform-address (BLAST) balance sync coordinator.
    /// Not auto-started — call `start` after wallets are registered.
    pub(super) platform_address_sync_manager: Arc<PlatformAddressSyncManager>,
    /// Periodic per-identity token state sync coordinator. Refreshes
    /// the per-(identity, token) balance cache on every registered
    /// wallet. Not auto-started — call `start` after wallets are
    /// registered. See [`IdentitySyncManager`].
    pub(super) identity_sync_manager: Arc<IdentitySyncManager<P>>,
    /// Periodic shielded (Orchard) note + nullifier sync coordinator.
    /// Iterates every wallet that has been bound via
    /// [`PlatformWallet::bind_shielded`](crate::wallet::PlatformWallet::bind_shielded);
    /// unbound wallets are skipped silently. Not auto-started — call
    /// `start` after wallets are registered.
    #[cfg(feature = "shielded")]
    pub(super) shielded_sync_manager: Arc<ShieldedSyncManager>,
    pub(super) persister: Arc<P>,
    /// Fan-out for platform-wallet lifecycle events
    /// ([`PlatformEvent`](crate::events::PlatformEvent)). Held so
    /// `load_from_persistor` can surface per-wallet skip notifications
    /// to the app handler via the established channel.
    pub(super) event_manager: Arc<PlatformEventManager>,
    /// Cancellation token + join handle for the wallet-event adapter
    /// task. Held so [`shutdown`] can stop it cleanly when the manager
    /// is torn down.
    pub(super) event_adapter_cancel: CancellationToken,
    pub(super) event_adapter_join: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Create a new PlatformWalletManager.
    ///
    /// `app_handler` receives all SPV and platform events by reference.
    /// Internally, a `LockNotifyHandler` is also registered to wake
    /// `AssetLockManager` async waiters on lock events.
    pub fn new(
        sdk: Arc<dash_sdk::Sdk>,
        persister: Arc<P>,
        app_handler: Arc<dyn PlatformEventHandler>,
    ) -> Self {
        let wallet_manager = Arc::new(RwLock::new(WalletManager::new(sdk.network)));
        let wallets = Arc::new(RwLock::new(std::collections::BTreeMap::new()));
        let lock_notify = Arc::new(Notify::new());

        // Spawn the wallet-event adapter that translates upstream
        // `WalletEvent`s into `CoreChangeSet`s and forwards them to
        // the persister.
        let event_adapter_cancel = CancellationToken::new();
        let event_adapter_join = spawn_wallet_event_adapter(
            Arc::clone(&wallet_manager),
            Arc::clone(&persister),
            event_adapter_cancel.clone(),
        );

        // Build handler list: app handler + internal handlers.
        // BalanceUpdateHandler holds a clone of the wallets map (a
        // separate lock from wallet_manager) so it can look up
        // PlatformWallets and write to their lock-free balance
        // atomics from broadcast-handler context without contending
        // with SPV's write lock.
        let lock_handler = Arc::new(LockNotifyHandler::new(Arc::clone(&lock_notify)));
        let balance_handler = Arc::new(BalanceUpdateHandler::new(Arc::clone(&wallets)));
        let event_manager = Arc::new(PlatformEventManager::new(vec![
            app_handler,
            lock_handler,
            balance_handler,
        ]));

        let spv = Arc::new(SpvRuntime::new(
            Arc::clone(&wallet_manager),
            Arc::clone(&event_manager),
        ));
        let platform_address_sync = Arc::new(PlatformAddressSyncManager::new(
            Arc::clone(&wallets),
            Arc::clone(&event_manager),
        ));
        let identity_sync = Arc::new(IdentitySyncManager::new(
            Arc::clone(&sdk),
            Arc::clone(&persister),
        ));
        #[cfg(feature = "shielded")]
        let shielded_sync = Arc::new(ShieldedSyncManager::new(
            Arc::clone(&wallets),
            Arc::clone(&event_manager),
        ));
        Self {
            sdk,
            wallet_manager,
            wallets,
            lock_notify,
            spv_manager: spv,
            platform_address_sync_manager: platform_address_sync,
            identity_sync_manager: identity_sync,
            #[cfg(feature = "shielded")]
            shielded_sync_manager: shielded_sync,
            persister,
            event_manager,
            event_adapter_cancel,
            event_adapter_join: tokio::sync::Mutex::new(Some(event_adapter_join)),
        }
    }

    /// Stop all background tasks and wait for them to exit.
    ///
    /// Stops the periodic coordinators (`PlatformAddressSyncManager`,
    /// `IdentitySyncManager`) and the wallet-event adapter task.
    /// Idempotent. Call before dropping the manager when a clean
    /// shutdown is required (e.g. on app termination); a dirty drop
    /// simply leaks the tasks until the runtime exits.
    pub async fn shutdown(&self) {
        self.platform_address_sync_manager.stop();
        self.identity_sync_manager.stop();
        #[cfg(feature = "shielded")]
        self.shielded_sync_manager.stop();

        self.event_adapter_cancel.cancel();
        if let Some(handle) = self.event_adapter_join.lock().await.take() {
            if let Err(e) = handle.await {
                tracing::warn!(error = ?e, "Wallet event adapter task join error");
            }
        }
    }
}
