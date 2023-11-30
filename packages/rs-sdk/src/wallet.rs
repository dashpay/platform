//! Wallet for managing keys assets in Dash Core and Platform.

use async_trait::async_trait;
use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::bls_signatures::PrivateKey;
use dpp::identity::{signer::Signer, IdentityPublicKey, Purpose};
use dpp::platform_value::BinaryData;
use dpp::prelude::AssetLockProof;
use dpp::ProtocolError;

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

impl Signer for Box<dyn Wallet> {
    fn sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.as_ref().sign(pubkey, message)
    }
}

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
pub trait PlatformWallet: Signer + Send + Sync {
    /// Return default identity public key for the provided purpose.
    fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey>;
}
