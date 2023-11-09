//! Wallet for managing keys assets in Dash Core and Platform.

use async_trait::async_trait;
use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::bls_signatures::PrivateKey;
pub use dpp::identity::signer::Signer;
use dpp::prelude::AssetLockProof;

use crate::Error;

/// Wallet used by Dash Platform SDK.
///
/// Wallet is used to manage keys and addresses, and sign transactions.
/// It must support:
///
/// * Dash Core operations, as defined in [CoreWallet]
/// * Platform operations, as defined in [PlatformWallet]
///
pub trait Wallet: CoreWallet + PlatformWallet + Send + Sync {}

/// Core Wallet manages Dash Core keys, addresses and signs transactions.
///
/// This trait should be implemented by developers who use the Sdk, to provide interface to Dash Core
/// wallet that allows generation and signing of Dash Core transactions.
#[async_trait]
pub trait CoreWallet: Send + Sync {
    /// Create new asset lock transaction that locks some amount of Dash to be used in Platform.
    ///
    /// # Arguments
    ///
    /// * `amount` - Amount of Dash to lock.
    ///
    /// # Returns
    ///
    /// * `AssetLockProof` - Asset lock proof.
    /// * `PrivateKey` - One-time private key used to use locked Dash in Platform.
    /// This key should be used to sign Platform transactions.
    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error>;

    /// Return balance of the wallet, in satoshis.
    async fn core_balance(&self) -> Result<u64, Error>;

    /// Return list of unspent transactions with summarized balance at least `sum`
    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error>;
}

/// Platform Wallet that can be used to sign Platform transactions.
///
/// This trait should be implemented by developers who use the Sdk, to provide interface to Platform
/// wallet that allows signing of Platform state transitions.
pub trait PlatformWallet: Send + Sync {
    /// Return signer that can be used to sign Platform transactions.
    fn signer(&self) -> &dyn Signer;
}

/// Wallet that combines separate Core and Platform wallets into one.
///
/// ## See also
///
/// * [CoreGrpcWallet](crate::mock::wallet::CoreGrpcWallet)
/// * [PlatformSignerWallet](crate::mock::wallet::PlatformSignerWallet)
pub struct CompositeWallet<C: CoreWallet, P: PlatformWallet> {
    core_wallet: C,
    platform_wallet: P,
}

impl<C: CoreWallet, P: PlatformWallet> CompositeWallet<C, P> {
    /// Create new composite wallet.
    ///
    /// Create new composite wallet comprising of Core wallet and Platform wallet.
    pub fn new(core_wallet: C, platform_wallet: P) -> Self {
        Self {
            core_wallet,
            platform_wallet,
        }
    }

    /// Return Core wallet client.
    pub fn core(&self) -> &C {
        &self.core_wallet
    }

    /// Return Platform wallet client.
    pub fn platform(&self) -> &P {
        &self.platform_wallet
    }
}
#[async_trait]
impl<C: CoreWallet, P: PlatformWallet> CoreWallet for CompositeWallet<C, P> {
    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
        self.core_wallet.lock_assets(amount).await
    }
    /// Return balance of the wallet, in satoshis.
    async fn core_balance(&self) -> Result<u64, Error> {
        self.core_wallet.core_balance().await
    }

    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
        self.core_wallet.core_utxos(sum).await
    }
}

#[async_trait]
impl<C: CoreWallet, P: PlatformWallet> PlatformWallet for CompositeWallet<C, P> {
    fn signer(&self) -> &dyn Signer {
        self.platform_wallet.signer()
    }
}

impl<C: CoreWallet, P: PlatformWallet> Wallet for CompositeWallet<C, P> {}
