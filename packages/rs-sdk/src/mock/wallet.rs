//! Wallet for managing keys assets in Dash Core and Platform.
pub(crate) mod core_grpc_wallet;

pub use self::core_grpc_wallet::CoreClient;
use crate::core::subscriber::{Message, SubscriptionController, SubscriptionError};
use crate::wallet::Wallet;
use async_trait::async_trait;
use dashcore_rpc::dashcore::consensus::Decodable;
use dashcore_rpc::dashcore::secp256k1::rand::{rngs::StdRng, SeedableRng};
use dashcore_rpc::dashcore::secp256k1::Secp256k1;
use dashcore_rpc::dashcore::transaction::special_transaction::TransactionPayload;
use dashcore_rpc::dashcore::transaction::{
    special_transaction::asset_lock::AssetLockPayload, Transaction,
};
use dashcore_rpc::dashcore::{Address, Network, OutPoint, ScriptBuf, TxIn, TxOut, Txid};
use dashcore_rpc::dashcore_rpc_json::{ListUnspentResultEntry, SignRawTransactionInput};
use dpp::identity::state_transition::asset_lock_proof::InstantAssetLockProof;
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
    version::PlatformVersion,
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
    pub core_subscriptions: crate::core::subscriber::SubscriptionController,

    /// Cancellation token that will shutdown the wallet when cancelled
    pub cancel: CancellationToken,
}

impl MockWallet {
    /// Create new mock wallet using Dash Core GRPC interface to access Dash Core, and a fixed private key.
    pub fn new_mock(
        network_type: Network,
        hostname: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
        cancel: CancellationToken,
        platform_version: &PlatformVersion,
    ) -> Result<Self, WalletError> {
        let grpc_wallet = CoreClient::new(hostname, core_port, core_user, core_password)?;

        let address = format!("http://{}:{}", hostname, core_port);
        let address_list = AddressList::from(address.as_str());
        let dapi = Arc::new(DapiClient::new(address_list, RequestSettings::default()));

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
            .ok_or(ProtocolError::PublicKeyGenerationError(
                "At least one key is required in Signer".to_string(),
            ))?;

        let mut default_keys: BTreeMap<Purpose, KeyID> = BTreeMap::new();
        default_keys.insert(first_key.purpose(), first_key.id());
        let core_subscriptions = SubscriptionController::new(Arc::clone(&dapi), cancel.clone());

        Ok(Self {
            grpc_wallet,
            dapi: Arc::clone(&dapi),
            signer,
            default_keys,
            network: network_type,
            confirmation_target: 2,
            rng: Mutex::new(rng),
            core_subscriptions,
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

/// Errors that can occur when using mock wallet
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    /// Invalid version
    #[error("Dash Platform Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

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

    /// Subscription failed
    #[error("Subscription failed: {0}")]
    Subscription(#[from] SubscriptionError),

    /// Timed out
    #[error("Timed out: {0}")]
    TimedOut(String),
}

#[async_trait]
impl Wallet for MockWallet {
    async fn platform_sign(
        &self,
        _pubkey: &IdentityPublicKey,
        _message: &[u8],
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
        let first_utxo = utxos.first().cloned().ok_or(WalletError::NotEnoughFunds)?;

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
        let inputs: Vec<TxIn> = utxos
            .iter()
            .map(|utxo| {
                let previous_output = OutPoint {
                    txid: utxo.txid,
                    vout: utxo.vout,
                };

                TxIn {
                    previous_output,
                    ..Default::default()
                }
                // OutPoint{ hash: addr.txid, index: addr.vout}
            })
            .collect::<Vec<TxIn>>();

        let tx: Transaction = Transaction {
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

        // Sign transaction with core wallet
        let signed_tx = self.core_sign_tx(&tx, Some(&utxos)).await?;
        let mut tx_reader = signed_tx.as_slice();
        let signed_transaction = Transaction::consensus_decode(&mut tx_reader)
            .map_err(|e| WalletError::Protocol(e.into()))?;
        let txid = signed_transaction.txid();

        // subscribe to channel with core chain updates, filter by address of first utxo
        let first_address = first_utxo
            .address
            .clone()
            .ok_or(WalletError::InvalidCoreAddress(
                "Empty address in first UTXO".to_string(),
            ))?
            .require_network(self.network)
            .map_err(|e| {
                WalletError::InvalidCoreAddress(format!("invalid network for core address: {}", e))
            })?;

        let _watch_guard = self
            .core_subscriptions
            .subscribe_to_address(first_address, None)
            .await;

        let mut channel = self.core_subscriptions.receiver();

        // Now submit the transaction to the network
        let received_txid = self
            .grpc_wallet
            .send_raw_transaction(signed_tx.as_slice())?;

        if txid != received_txid {
            panic!("txid mismatch: {} != {}", txid, received_txid)
        }

        // Wait for result - we expect to get an InstantAssetLock message for our txid
        loop {
            tokio::select! {
                _ = self.cancel.cancelled() => {
                    return Err(WalletError::TimedOut("shutdown initiated".to_string()));
                }
                recv = channel.recv() => {
                    match recv {
                        // recv failed
                        Err(e) => return Err(SubscriptionError::Cancelled(format!("subscription error: {}", e)).into()),
                        // processing of message failed; we don't really care about this as this is not necessarily
                        // our msg
                        Ok(Err(e)) => {
                            tracing::warn!("error processing subscription message: {}", e);
                            // non-fatal error, continue
                        }
                        Ok(Ok(Message::InstantAssetLock{instant_lock, ..})) => {
                           if let Some(outpoint) = instant_lock.inputs.clone().into_iter().find(|input| input.txid == txid){
                               // we found our txid in the inputs, so we can stop
                               return Ok((AssetLockProof::Instant(InstantAssetLockProof{
                                      transaction: tx,
                                      instant_lock,
                                      output_index: outpoint.vout, // FIXME: is this correct?
                               }),asset_lock_key.0));
                           }
                        },
                        Ok(Ok(Message::MerkleBlock{..})) => {
                            todo!("merkle block processing is a TODO")
                        }
                    }
                }
            }
        }
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
