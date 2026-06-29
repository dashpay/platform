//! Hydrate a [`PlatformWalletManager`] from its persister.

use crate::changeset::PlatformWalletPersistence;
use crate::error::PlatformWalletError;

use super::PlatformWalletManager;

impl<P: PlatformWalletPersistence + 'static> PlatformWalletManager<P> {
    /// Rehydrate the manager's wallet maps from the configured persister.
    ///
    /// # Errors
    ///
    /// Keyless rehydration lands in #3692; #3968 ships the storage layer only,
    /// so on the independent branch this entry point returns
    /// [`PlatformWalletError::WalletCreation`] rather than performing the
    /// rebuild. It must NOT panic — this is called across the C ABI, where an
    /// unwind is undefined behaviour. The integration branch replaces the whole
    /// body with the real implementation.
    pub async fn load_from_persistor(&self) -> Result<(), PlatformWalletError> {
        Err(PlatformWalletError::WalletCreation(
            "keyless rehydration from the persister is not available on this build \
             (lands in #3692)"
                .to_string(),
        ))
    }
}
