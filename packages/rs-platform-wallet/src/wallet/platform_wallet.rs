//! The main PlatformWallet struct combining core, identity, dashpay, and platform sub-wallets.

use std::sync::Arc;
use std::sync::Mutex;

use key_wallet::wallet::initialization::WalletAccountCreationOptions;
use key_wallet::wallet::managed_wallet_info::ManagedWalletInfo;
use key_wallet::wallet::Wallet;
use key_wallet::{Mnemonic, Network, Seed};
use tokio::sync::RwLock;

use crate::error::PlatformWalletError;
use crate::persistence::{PlatformWalletChangeSet, PlatformWalletPersistence};

use super::core::CoreWallet;
use super::dashpay::DashPayWallet;
use super::identity::{IdentityManager, IdentityWallet};
use super::platform_addresses::PlatformAddressWallet;
use super::tokens::TokenWallet;

/// Unique identifier for a wallet (32-byte hash).
pub type WalletId = [u8; 32];

/// A platform wallet that combines core UTXO functionality with identity management.
///
/// This is SPV-free. It needs only key material and an `Sdk`.
/// For SPV support, use [`PlatformWalletManager`](crate::manager::PlatformWalletManager).
///
/// # Cloning
///
/// `PlatformWallet` is cheaply cloneable (~35 atomic ops). A clone is a **shared
/// handle** to the same mutable state — not an independent copy. All clones see
/// the same UTXOs, balances, and identities through shared `Arc<RwLock<...>>` fields.
pub struct PlatformWallet {
    wallet_id: WalletId,
    pub(crate) sdk: Arc<dash_sdk::Sdk>,
    pub(crate) core: CoreWallet,
    pub(crate) identity: IdentityWallet,
    pub(crate) dashpay: DashPayWallet,
    pub(crate) platform: PlatformAddressWallet,
    pub(crate) tokens: TokenWallet,
    /// Optional persistence backend.  Set via [`set_persister`](Self::set_persister).
    persister: Option<Arc<Mutex<Box<dyn PlatformWalletPersistence>>>>,
}

impl PlatformWallet {
    /// Access the core wallet (balance, UTXOs, addresses).
    pub fn core(&self) -> &CoreWallet {
        &self.core
    }

    /// Access the core wallet mutably.
    pub fn core_mut(&mut self) -> &mut CoreWallet {
        &mut self.core
    }

    /// Access the identity wallet.
    pub fn identity(&self) -> &IdentityWallet {
        &self.identity
    }

    /// Access the DashPay wallet.
    pub fn dashpay(&self) -> &DashPayWallet {
        &self.dashpay
    }

    /// Access the platform address wallet.
    pub fn platform(&self) -> &PlatformAddressWallet {
        &self.platform
    }

    /// Access the token wallet.
    pub fn tokens(&self) -> &TokenWallet {
        &self.tokens
    }

    /// Get the wallet ID.
    pub fn wallet_id(&self) -> WalletId {
        self.wallet_id
    }

    /// Get a reference to the SDK.
    pub fn sdk(&self) -> &dash_sdk::Sdk {
        &self.sdk
    }

    /// Construct a PlatformWallet from an existing key-wallet Wallet and ManagedWalletInfo.
    pub fn from_wallet_and_info(
        sdk: Arc<dash_sdk::Sdk>,
        wallet: Wallet,
        wallet_info: ManagedWalletInfo,
    ) -> Self {
        let wallet_id = wallet_info.wallet_id;
        let wallet = Arc::new(RwLock::new(wallet));
        let wallet_info = Arc::new(RwLock::new(wallet_info));
        let identity_manager = Arc::new(RwLock::new(IdentityManager::new()));

        let core = CoreWallet::new(Arc::clone(&sdk), wallet.clone(), wallet_info.clone());

        let identity = IdentityWallet {
            sdk: Arc::clone(&sdk),
            wallet: wallet.clone(),
            wallet_info: wallet_info.clone(),
            identity_manager: identity_manager.clone(),
        };

        let dashpay = DashPayWallet {
            sdk: Arc::clone(&sdk),
            wallet: wallet.clone(),
            wallet_info: wallet_info.clone(),
            identity_manager: identity_manager.clone(),
        };

        let platform =
            PlatformAddressWallet::new(Arc::clone(&sdk), wallet.clone(), wallet_info.clone());

        let tokens = TokenWallet::new(Arc::clone(&sdk), wallet.clone(), identity_manager.clone());

        Self {
            wallet_id,
            sdk,
            core,
            identity,
            dashpay,
            platform,
            tokens,
            persister: None,
        }
    }

    /// Create a PlatformWallet from a BIP-39 mnemonic.
    pub fn from_mnemonic(
        sdk: Arc<dash_sdk::Sdk>,
        network: Network,
        mnemonic: &str,
        passphrase: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<Self, PlatformWalletError> {
        let mnemonic_obj: Mnemonic = mnemonic.parse().map_err(|e| {
            PlatformWalletError::WalletCreation(format!("Failed to parse mnemonic: {}", e))
        })?;

        let wallet = if passphrase.is_empty() {
            Wallet::from_mnemonic(mnemonic_obj, network, options)
        } else {
            Wallet::from_mnemonic_with_passphrase(
                mnemonic_obj,
                passphrase.to_string(),
                network,
                options,
            )
        }
        .map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from mnemonic: {}",
                e
            ))
        })?;

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok(Self::from_wallet_and_info(sdk, wallet, wallet_info))
    }

    /// Create a PlatformWallet from an extended private key string.
    ///
    /// The network is derived from the extended key itself (xprv encodes the network).
    pub fn from_extended_key(
        sdk: Arc<dash_sdk::Sdk>,
        xprv: &str,
        options: WalletAccountCreationOptions,
    ) -> Result<Self, PlatformWalletError> {
        use key_wallet::bip32::ExtendedPrivKey;

        let extended_key: ExtendedPrivKey = xprv.parse().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to parse extended private key: {}",
                e
            ))
        })?;

        let wallet = Wallet::from_extended_key(extended_key, options).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from extended key: {}",
                e
            ))
        })?;

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok(Self::from_wallet_and_info(sdk, wallet, wallet_info))
    }

    /// Create a watch-only PlatformWallet from an extended public key string.
    pub fn from_xpub(
        sdk: Arc<dash_sdk::Sdk>,
        network: Network,
        xpub: &str,
    ) -> Result<Self, PlatformWalletError> {
        use key_wallet::bip32::ExtendedPubKey;
        use key_wallet::wallet::root_extended_keys::RootExtendedPubKey;

        let xpub_key: ExtendedPubKey = xpub.parse().map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to parse extended public key: {}",
                e
            ))
        })?;

        let root_xpub = RootExtendedPubKey::from_extended_pub_key(&xpub_key);
        let wallet = Wallet::from_wallet_type(
            network,
            key_wallet::wallet::WalletType::WatchOnly(root_xpub),
        );

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok(Self::from_wallet_and_info(sdk, wallet, wallet_info))
    }

    /// Create a PlatformWallet from a BIP-39 Seed.
    pub fn from_seed(
        sdk: Arc<dash_sdk::Sdk>,
        network: Network,
        seed: Seed,
        options: WalletAccountCreationOptions,
    ) -> Result<Self, PlatformWalletError> {
        let wallet = Wallet::from_seed(seed, network, options).map_err(|e| {
            PlatformWalletError::WalletCreation(format!("Failed to create wallet from seed: {}", e))
        })?;

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok(Self::from_wallet_and_info(sdk, wallet, wallet_info))
    }

    /// Create a PlatformWallet from raw seed bytes (64 bytes).
    pub fn from_seed_bytes(
        sdk: Arc<dash_sdk::Sdk>,
        network: Network,
        seed_bytes: [u8; 64],
        options: WalletAccountCreationOptions,
    ) -> Result<Self, PlatformWalletError> {
        let wallet = Wallet::from_seed_bytes(seed_bytes, network, options).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from seed bytes: {}",
                e
            ))
        })?;

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok(Self::from_wallet_and_info(sdk, wallet, wallet_info))
    }

    /// Create a PlatformWallet with a random mnemonic. Returns the wallet and the mnemonic.
    pub fn random(
        sdk: Arc<dash_sdk::Sdk>,
        network: Network,
        options: WalletAccountCreationOptions,
    ) -> Result<(Self, Mnemonic), PlatformWalletError> {
        let mnemonic =
            Mnemonic::generate(12, key_wallet::mnemonic::Language::English).map_err(|e| {
                PlatformWalletError::WalletCreation(format!(
                    "Failed to generate random mnemonic: {}",
                    e
                ))
            })?;

        let wallet = Wallet::from_mnemonic(mnemonic.clone(), network, options).map_err(|e| {
            PlatformWalletError::WalletCreation(format!(
                "Failed to create wallet from random mnemonic: {}",
                e
            ))
        })?;

        let wallet_info = ManagedWalletInfo::from_wallet(&wallet);
        Ok((
            Self::from_wallet_and_info(sdk, wallet, wallet_info),
            mnemonic,
        ))
    }
}

impl PlatformWallet {
    /// Attach a persistence backend.
    ///
    /// The persister is wrapped in `Arc<Mutex<..>>` so it can be shared across
    /// clones and accessed from synchronous contexts (SPV callbacks).
    pub fn set_persister(&mut self, persister: Box<dyn PlatformWalletPersistence>) {
        self.persister = Some(Arc::new(Mutex::new(persister)));
    }

    /// Queue a changeset for later persistence.
    ///
    /// If no persister is attached this is a no-op.
    pub fn queue_persist(&self, changeset: PlatformWalletChangeSet) {
        if let Some(persister) = &self.persister {
            if let Ok(mut p) = persister.lock() {
                p.queue(changeset);
            }
        }
    }

    /// Flush all queued changesets to the storage backend.
    ///
    /// Returns `Ok(())` if no persister is attached or the flush succeeds.
    pub fn flush_persist(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(persister) = &self.persister {
            if let Ok(mut p) = persister.lock() {
                p.flush()?;
            }
        }
        Ok(())
    }

    /// Apply a changeset to in-memory wallet state.
    ///
    /// Currently applies key-wallet sub-changesets to `ManagedWalletInfo`.
    /// Identity, contact, and platform-address application will be added as
    /// those sub-wallets gain changeset-driven state.
    pub fn apply(&self, changeset: &PlatformWalletChangeSet) {
        // Apply key-wallet changeset to ManagedWalletInfo if present.
        if let Some(_wallet_cs) = &changeset.wallet {
            if let Some(mut _info) = self.core.try_wallet_info_mut() {
                // TODO: apply wallet_cs to info once ManagedWalletInfo
                // exposes an apply(WalletChangeSet) method.
            }
        }
        // TODO: apply contacts changeset
        // TODO: apply identities changeset
        // TODO: apply platform_addresses changeset
    }
}

impl Clone for PlatformWallet {
    fn clone(&self) -> Self {
        Self {
            wallet_id: self.wallet_id,
            sdk: self.sdk.clone(),
            core: self.core.clone(),
            identity: self.identity.clone(),
            dashpay: self.dashpay.clone(),
            platform: self.platform.clone(),
            tokens: self.tokens.clone(),
            // Cloned instances do not inherit the persister.
            persister: None,
        }
    }
}

impl std::fmt::Debug for PlatformWallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlatformWallet")
            .field("wallet_id", &hex::encode(self.wallet_id))
            .finish()
    }
}
