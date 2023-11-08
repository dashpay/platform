//! Wallet for managing keys assets in Dash Core and Platform.

use dashcore_rpc::dashcore::address::NetworkUnchecked;
use dashcore_rpc::dashcore::Address;
use dashcore_rpc::json;
use dpp::bls_signatures::PrivateKey;
pub use dpp::identity::signer::Signer;
use dpp::prelude::AssetLockProof;
use rs_dapi_client::transport::CoreGrpcClient;
use simple_signer::signer::SimpleSigner;

use crate::{core_client::CoreClient, Error, Sdk};

/// Default wallet implementation for Dash Platform SDK.
///
/// This wallet uses Dash Core wallet RPC client to manage Core keys, and [Signer] instance to manage Platform keys.
///
pub struct Wallet {
    core_wallet: CoreWallet,
    platform_wallet: SimpleSigner,
}

impl Wallet {
    /// Create new wallet.
    ///
    /// Create new wallet using dash core wallet RPC client to manage Core keys, and Signer instance to manage Platform keys.
    ///
    /// # Arguments
    ///
    /// * `core` - Dash Core RPC client [CoreClient]. Should be owned by [Sdk].
    /// * `signer` - [Signer] instance, for example [simple_signer::signer::SimpleSigner].
    /// Should be owned by [Sdk].
    pub(crate) fn new_with_clients<S: Signer>(
        core_client: CoreClient,
        signer: SimpleSigner,
    ) -> Self {
        Self {
            core_wallet: CoreWallet { core_client },
            platform_wallet: signer,
        }
    }

    /// Return Core wallet client.
    pub(crate) fn core<'a>(&'a self) -> &'a CoreWallet {
        &self.core_wallet
    }

    /// Return Platform wallet client.
    pub(crate) fn platform<'a>(&'a self) -> &'a SimpleSigner {
        &self.platform_wallet
    }
}

pub(crate) struct CoreWallet {
    core_client: CoreClient,
}
impl CoreWallet {
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
    pub(crate) async fn lock_assets(
        &self,
        sdk: &Sdk,
        amount: u64,
    ) -> Result<(AssetLockProof, PrivateKey), Error> {
        let change = self.core_client.change_address();
        todo!("implement asset lock tx")
    }
}
