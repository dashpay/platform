//! Wallet for managing keys assets in Dash Core and Platform.

use async_trait::async_trait;
pub use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::bls_signatures::PrivateKey;
use dpp::identity::{signer::Signer, IdentityPublicKey, Purpose};
use dpp::platform_value::BinaryData;
use dpp::prelude::AssetLockProof;
use dpp::ProtocolError;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::Error;

/// Wallet used by Dash Platform SDK.
///
/// Wallet is used to manage keys and addresses, and sign transactions.
/// It must support:
///
/// * Dash Core operations, as defined in [CoreWallet]
/// * Platform operations, as defined in [PlatformWallet]
#[async_trait]
pub trait Wallet: Send + Sync {
    // PLATFORM WALLET FUNCTIONS

    /// Sign message using private key associated with provided public key
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, Error>;

    /// Return default identity public key for the provided purpose.
    async fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey>;

    // CORE WALLET FUNCTIONS

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

// struct WalletSigner<W>(W);
// impl<W: Wallet> From<W> for WalletSigner<W> {
//     fn from(wallet: W) -> Self {
//         Self(wallet)
//     }
// }
// impl AsRef<dyn Signer> for dyn Wallet
// where
//     Self: Send + Sync + Wallet,
// {
//     fn as_ref(&self) -> &'static dyn Signer {
//         &WalletSigner(self)
//     }
// }

impl Signer for Box<dyn Wallet> {
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        let wallet = self.as_ref();
        tokio::task::block_in_place(|| {
            Handle::current().block_on(wallet.platform_sign(identity_public_key, data))
        })
        .map_err(|e| ProtocolError::Generic(e.to_string()))
    }
}

// impl Signer for WalletSigner<&dyn Wallet>
// where
//     Self: Send + Sync,
// {
//     fn sign(
//         &self,
//         pubkey: &IdentityPublicKey,
//         message: &[u8],
//     ) -> Result<BinaryData, ProtocolError> {
//         let wallet = self.0;
//         // TODO: check if this tokio construct works
//         tokio::task::block_in_place(|| {
//             Handle::current().block_on(wallet.platform_sign(pubkey, message))
//         })
//         .map_err(|e| ProtocolError::Generic(e.to_string()))
//     }
// }
// impl<W: AsRef<dyn Wallet>> Signer for WalletSigner<W>
// where
//     W: Send + Sync,
// {
//     fn sign(
//         &self,
//         pubkey: &IdentityPublicKey,
//         message: &[u8],
//     ) -> Result<BinaryData, ProtocolError> {
//         let wallet = self.0.as_ref();
//         // TODO: check if this tokio construct works
//         tokio::task::block_in_place(|| {
//             Handle::current().block_on(wallet.platform_sign(pubkey, message))
//         })
//         .map_err(|e| ProtocolError::Generic(e.to_string()))
//     }
// }

#[async_trait]
//impl<W: AsRef<dyn Wallet>> Wallet for W
impl Wallet for &'static dyn Wallet
where
    Self: Send + Sync,
{
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, Error> {
        (*self).platform_sign(pubkey, message).await
    }

    async fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey> {
        (*self).identity_public_key(purpose).await
    }

    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
        (*self).lock_assets(amount).await
    }

    async fn core_balance(&self) -> Result<u64, Error> {
        (*self).core_balance().await
    }

    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
        (*self).core_utxos(sum).await
    }
}

// #[async_trait]
// impl Wallet for Box<dyn Wallet> {
//     async fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey> {
//         self.as_ref().identity_public_key(purpose).await
//     }

//     async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
//         self.as_ref().lock_assets(amount).await
//     }

//     async fn core_balance(&self) -> Result<u64, Error> {
//         self.as_ref().core_balance().await
//     }

//     async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
//         self.as_ref().core_utxos(sum).await
//     }
// }

#[async_trait]
impl<W: Wallet> Wallet for Arc<W> {
    /// Sign message using private key associated with provided public key
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, Error> {
        self.as_ref().platform_sign(pubkey, message).await
    }

    async fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey> {
        self.as_ref().identity_public_key(purpose).await
    }

    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
        self.as_ref().lock_assets(amount).await
    }

    async fn core_balance(&self) -> Result<u64, Error> {
        self.as_ref().core_balance().await
    }

    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
        self.as_ref().core_utxos(sum).await
    }
}
