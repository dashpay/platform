//! Hydrate a [`PlatformWalletManager`] from its persister.

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Rehydrate the manager's wallet maps from the configured persister.
    ///
    /// Keyless rehydration lands in #3692; #3968 ships the storage layer
    /// only, so on the independent branch this entry point is a stub.
    pub async fn load_from_persistor(&self) -> Result<(), PlatformWalletError> {
        todo!("keyless rehydration lands in #3692")
    }
}
