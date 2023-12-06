//! Wallet for managing keys assets in Dash Core and Platform.
pub(crate) mod core_grpc_wallet;

pub use self::core_grpc_wallet::CoreClient;
use crate::core::subscriber::Subscriber;
use crate::{wallet::Wallet, Error};
use async_trait::async_trait;
use dashcore_rpc::dashcore::secp256k1::rand::{rngs::StdRng, SeedableRng};
use dashcore_rpc::dashcore::secp256k1::Secp256k1;
use dashcore_rpc::dashcore::transaction::special_transaction::TransactionPayload;
use dashcore_rpc::dashcore::transaction::{
    special_transaction::asset_lock::AssetLockPayload, Transaction,
};
use dashcore_rpc::dashcore::{Address, Network, OutPoint, ScriptBuf, TxIn, TxOut, Txid};
use dashcore_rpc::dashcore_rpc_json::{ListUnspentResultEntry, SignRawTransactionInput};
use dpp::{
    dashcore::{PrivateKey, PublicKey},
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
use futures::lock::Mutex;
use rand::Rng;
use rs_dapi_client::{AddressList, DapiClient, RequestSettings};
use simple_signer::signer::SimpleSigner;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Mock wallet that uses core grpc wallet and platform [SimpleSigner] to implement wallet trait.
///
/// ## See also
///
/// * [CoreGrpcWallet](crate::mock::wallet::CoreGrpcWallet)
/// * [PlatformSignerWallet](crate::mock::wallet::PlatformSignerWallet)
#[derive(Debug)]
pub struct MockWallet {
    /// Dash Core RPC client, used to connect to Core wallet
    pub grpc_wallet: CoreClient,

    /// DAPI Client, used in subscriptions
    pub dapi: Arc<DapiClient>,

    /// Signer used to sign Platform transactions.
    pub signer: SimpleSigner,

    /// Default keys used to sign Platform transactions for each purpose.
    pub default_keys: BTreeMap<dpp::identity::Purpose, KeyID>,

    /// Network type to use
    pub network: Network,

    /// Default confirmation target - how many blocks we want to wait for confirmation.
    /// Defaults to 2.
    ///
    pub confirmation_target: u64,

    /// Random number generator to use
    pub rng: Mutex<StdRng>,

    /// Subscription to core events
    pub core_subscriptions: crate::core::subscriber::Subscriber,

    pub cancel: CancellationToken,
}

impl MockWallet {
    /// Create new mock wallet using Dash Core GRPC interface to access Dash Core, and a fixed private key.
    pub fn new_mock(
        network_type: Network,
        ip: &str,
        port: u16,
        user: &str,
        password: &str,
        cancel: CancellationToken,
    ) -> Result<Self, WalletError> {
        let grpc_wallet = CoreClient::new(ip, port, user, password)?;

        let address = format!("http://{}:{}", ip, port);
        let address_list = AddressList::from(address.as_str());
        let dapi = Arc::new(DapiClient::new(address_list, RequestSettings::default()));

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
            grpc_wallet,
            dapi: Arc::clone(&dapi),
            signer,
            default_keys,
            network: network_type,
            confirmation_target: 2,
            rng: Mutex::new(rng),
            core_subscriptions: Subscriber::new(Arc::clone(&dapi), cancel.clone()),
            cancel,
        })
    }

    async fn generate_ssecp256k1_key_pair(&self) -> (PrivateKey, PublicKey) {
        let mut rng = self.rng.lock().await;
        let random_private_key: [u8; 32] = rng.gen();
        drop(rng);

        let private_key = PrivateKey::from_slice(&random_private_key, Network::Testnet)
            .expect("expected a private key");

        let secp = Secp256k1::new();
        let public_key = private_key.public_key(&secp);

        (private_key, public_key)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    /// Invalid version
    #[error("Dash Platform Protocol error: {0}")]
    Protocol(#[from] ProtocolError),
    /// Error from Dash Sdk
    #[error("Error from Dash Sdk: {0}")]
    Sdk(#[from] crate::Error),
    /// Dash Core RPC error
    #[error("Dash Core RPC error: {0}")]
    CoreRpc(#[from] dashcore_rpc::Error),
    /// Not enough funds to execute operation
    #[error("Not enough funds")]
    NotEnoughFunds,
    /// Json formatting error
    #[error("Json formatting error: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid address
    #[error("invalid Dash Core address: {0}")]
    InvalidCoreAddress(String),
}

#[async_trait]
impl Wallet for MockWallet {
    async fn platform_sign(
        &self,
        pubkey: &IdentityPublicKey,
        message: &[u8],
    ) -> Result<BinaryData, WalletError> {
        todo!("Not yet implemented")
    }

    async fn core_sign_tx(
        &self,
        tx: &Transaction,
        utxos: Option<&[SignRawTransactionInput]>,
    ) -> Result<BinaryData, WalletError> {
        let signed = self
            .grpc_wallet
            .sign_raw_transaction_with_wallet(tx, utxos, None)?;

        Ok(signed.hex.into())
    }

    async fn lock_assets(&self, amount: u64) -> Result<(AssetLockProof, PrivateKey), WalletError> {
        let fee = self.core_estimate_fee(self.confirmation_target).await?;

        // Now, let's generate key pair
        let asset_lock_key = self.generate_ssecp256k1_key_pair().await;
        let asset_lock_key_hash = asset_lock_key.1.pubkey_hash();

        let utxos = self.core_utxos(Some(amount)).await?;

        let unspent_amount: u64 = utxos.iter().map(|utxo| utxo.amount.to_sat()).sum();
        let change = unspent_amount - amount - fee;

        let change_address = self.core_change_address().await?;

        let payload_output = TxOut {
            value: amount, // 1 Dash
            script_pubkey: ScriptBuf::new_p2pkh(&asset_lock_key_hash),
        };
        let burn_output = TxOut {
            value: amount, // 1 Dash
            script_pubkey: ScriptBuf::new_op_return(&[]),
        };
        if change < fee {
            return Err(WalletError::NotEnoughFunds);
        }
        let change_output = TxOut {
            value: change - fee,
            script_pubkey: change_address.script_pubkey(),
        };
        let payload = AssetLockPayload {
            version: 1,
            credit_outputs: vec![payload_output],
        };

        // we need to get all inputs from utxos to add them to the transaction

        let mut inputs: Vec<TxIn> = utxos
            .iter()
            .map(|utxo| {
                let previous_output = OutPoint {
                    txid: utxo.txid,
                    vout: utxo.vout,
                };

                let mut tx_in = TxIn::default();
                // OutPoint{ hash: addr.txid, index: addr.vout}
                tx_in.previous_output = previous_output;

                tx_in
            })
            .collect::<Vec<TxIn>>();

        let sighash_u32 = 1u32;

        let mut tx: Transaction = Transaction {
            version: 3,
            lock_time: 0,
            input: inputs,
            output: vec![burn_output, change_output],
            special_transaction_payload: Some(TransactionPayload::AssetLockPayloadType(payload)),
        };

        let utxos: Vec<SignRawTransactionInput> = utxos
            .into_iter()
            .map(|utxo| SignRawTransactionInput {
                txid: utxo.txid,
                vout: utxo.vout,
                script_pub_key: utxo.script_pub_key,
                redeem_script: utxo.redeem_script,
                amount: Some(utxo.amount),
            })
            .collect();

        let signed_tx = self.core_sign_tx(&tx, Some(&utxos)).await?;

        let txid = self
            .grpc_wallet
            .send_raw_transaction(signed_tx.as_slice())?;

        // subscribe to channel with core chain updates
        let mut channel = self.core_subscriptions.subscribe();
        let msg = channel.recv().await.unwrap();

        todo!("broadcast and wait, see /core/transaction.rs")
    }

    async fn core_utxos(
        &self,
        sum: Option<u64>,
    ) -> Result<Vec<ListUnspentResultEntry>, WalletError> {
        let unspent: Vec<dashcore_rpc::dashcore_rpc_json::ListUnspentResultEntry> =
            self.grpc_wallet.list_unspent(sum)?;
        Ok(unspent)
    }

    async fn core_balance(&self) -> Result<u64, WalletError> {
        Ok(self.grpc_wallet.get_balance()?.to_sat())
    }

    async fn core_change_address(&self) -> Result<Address, WalletError> {
        let addr = self.grpc_wallet.change_address()?;
        addr.require_network(self.network)
            .map_err(|e| WalletError::InvalidCoreAddress(e.to_string()))
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

    async fn core_estimate_fee(&self, confirmation_target: u64) -> Result<u64, WalletError> {
        // TODO implement
        Ok(20000)
    }

    async fn core_broadcast_tx(&self, signed_tx: &[u8]) -> Result<Txid, WalletError> {
        let tx: Transaction = serde_json::from_slice(signed_tx)?;
        let txid = self.grpc_wallet.send_raw_transaction(&tx)?;
        Ok(txid)
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
