//! Wallet for managing keys assets in Dash Core and Platform.

use async_trait::async_trait;
use dashcore_rpc::dashcore::secp256k1::rand::{rngs::StdRng, SeedableRng};
use dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry;
use dpp::{
    bls_signatures::PrivateKey,
    identity::{
        signer::Signer, state_transition::asset_lock_proof::AssetLockProof, IdentityPublicKey,
    },
    platform_value::BinaryData,
    ProtocolError,
};
use dpp::{
    identity::{identity_public_key::accessors::v0::IdentityPublicKeyGettersV0, KeyID, Purpose},
    version::{PlatformVersion, PlatformVersionCurrentVersion},
};
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;
use std::fmt::Debug;

use crate::{wallet::Wallet, Error};

use self::core_client::CoreClient;
pub mod cache;
pub mod core_client;

/// Mock wallet that uses core grpc wallet and platform signer to implement wallet trait.
///
/// It provides contextual information about the state of application using
/// core wallet (connecting ) and cache for data contracts and quorum public keys

/// Wallet that combines separate Core and Platform wallets into one.
///
/// ## See also
///
/// * [CoreGrpcWallet](crate::mock::wallet::CoreGrpcWallet)
/// * [PlatformSignerWallet](crate::mock::wallet::PlatformSignerWallet)
#[derive(Debug)]
pub struct MockWallet {
    pub core_client: CoreClient,
    /// Signer used to sign Platform transactions.
    pub signer: SimpleSigner,

    /// Default keys used to sign Platform transactions for each purpose.
    pub default_keys: BTreeMap<dpp::identity::Purpose, KeyID>,
}

impl MockWallet {
    /// Create new mock wallet using Dash Core GRPC interface to access Dash Core, and a fixed private key.
    pub fn new_mock(ip: &str, port: u16, user: &str, password: &str) -> Result<Self, Error> {
        let core_client = CoreClient::new(ip, port, user, password)?;

        let platform_version = PlatformVersion::get_current()?; // TODO pass as arg
                                                                // it's a mock, so we always want the same key
        const SEED: [u8; 32] = [0u8; 32];
        let mut rng = StdRng::from_seed(SEED);
        // let privkey = PrivateKey::generate_dash(&mut rng).map_err(|e| Error::from(e))?;

        // TODO: also generate other types of keys
        let keys = IdentityPublicKey::random_authentication_keys_with_private_keys_with_rng(
            0 as KeyID,
            6,
            &mut rng,
            platform_version,
        )?;

        let mut signer = SimpleSigner::default();
        signer.add_keys(keys);

        // get first authentication key and set it as default
        let (first_key, _) = signer
            .private_keys
            .iter()
            .find(|(key, _)| key.purpose() == Purpose::AUTHENTICATION)
            .ok_or(Error::Protocol(ProtocolError::PublicKeyGenerationError(
                "At least one key is required in Signer".to_string(),
            )))?;

        let mut default_keys: BTreeMap<Purpose, KeyID> = BTreeMap::new();
        default_keys.insert(first_key.purpose(), first_key.id());

        Ok(Self {
            core_client,
            signer,
            default_keys,
        })
    }
}

#[async_trait]
impl Wallet for MockWallet {
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, Error> {
        todo!("Not yet implemented")
    }
    async fn lock_assets(&self, _amount: u64) -> Result<(AssetLockProof, PrivateKey), Error> {
        todo!("Not yet implemented")
    }

    async fn core_utxos(&self, sum: Option<u64>) -> Result<Vec<ListUnspentResultEntry>, Error> {
        let unspent: Vec<dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry> =
            self.core_client.list_unspent(sum)?;
        Ok(unspent)
    }

    async fn core_balance(&self) -> Result<u64, Error> {
        Ok(self.core_client.get_balance()?.to_sat())
    }

    async fn identity_public_key(&self, purpose: &Purpose) -> Option<IdentityPublicKey> {
        let id = self.default_keys.get(purpose)?;

        let (key, _) = self
            .signer
            .private_keys
            .iter()
            .find(|(key, _)| key.id() == *id)?;

        Some(key.clone())
    }
}

impl Signer for MockWallet {
    fn sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, ProtocolError> {
        self.signer.sign(pubkey, message)
    }
}
